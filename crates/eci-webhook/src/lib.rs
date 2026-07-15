use eci_core::config::Config;
use eci_core::error::Result;
use eci_core::state::State;
use eci_docker::DockerClient;
use eci_github::GitHubClient;
use hmac::{Hmac, Mac};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use sha2::Sha256;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

type HmacSha256 = Hmac<Sha256>;

struct WebhookState {
    config: Config,
    state: Mutex<State>,
    docker: Mutex<DockerClient>,
    secret: String,
}

pub async fn start_webhook_server(
    port: u16,
    config: Config,
    state: State,
    docker: DockerClient,
    secret: String,
) -> Result<()> {
    let state = Arc::new(WebhookState {
        config,
        state: Mutex::new(state),
        docker: Mutex::new(docker),
        secret,
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    println!("Webhook server listening on port {}", port);

    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();

        // Serve inline — no tokio::spawn, so Send isn't required
        let io = TokioIo::new(stream);
        let service = service_fn(move |req| {
            let state = state.clone();
            async move { handle_request(req, &state).await }
        });

        if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
            eprintln!("Error serving connection: {}", err);
        }
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: &Arc<WebhookState>,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    let response = match (req.method(), req.uri().path()) {
        (&Method::POST, "/webhook") => handle_webhook(req, state).await,
        (&Method::GET, "/health") => Response::new(Full::new(Bytes::from("ok"))),
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not found")))
            .unwrap(),
    };

    Ok(response)
}

async fn handle_webhook(
    req: Request<hyper::body::Incoming>,
    state: &Arc<WebhookState>,
) -> Response<Full<Bytes>> {
    let headers = req.headers().clone();
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Failed to read body")))
                .unwrap();
        }
    };

    let body_str = match String::from_utf8(body_bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Invalid UTF-8")))
                .unwrap();
        }
    };

    if let Some(signature) = headers.get("x-hub-signature-256") {
        let signature_str = signature.to_str().unwrap_or("");
        if !verify_signature(&state.secret, &body_str, signature_str) {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body(Full::new(Bytes::from("Invalid signature")))
                .unwrap();
        }
    }

    let event = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    match event {
        "push" => handle_push(state, &body_str).await,
        "ping" => Response::new(Full::new(Bytes::from("pong"))),
        _ => Response::new(Full::new(Bytes::from(format!("Event {} ignored", event)))),
    }
}

async fn handle_push(state: &Arc<WebhookState>, body: &str) -> Response<Full<Bytes>> {
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(p) => p,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!("Invalid JSON: {}", e))))
                .unwrap();
        }
    };

    let full_name = match payload
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(|v| v.as_str())
    {
        Some(n) => n,
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("No repository")))
                .unwrap();
        }
    };

    let ref_name = match payload.get("ref").and_then(|v| v.as_str()) {
        Some(r) => r,
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("No ref")))
                .unwrap();
        }
    };

    let branch = ref_name.strip_prefix("refs/heads/").unwrap_or(ref_name);
    println!("Push to {} ({}), deploying...", full_name, branch);

    let docker = state.docker.lock().await;
    let state_lock = state.state.lock().await;

    let github = match GitHubClient::new(&state.config).await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to create GitHub client: {}", e);
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(format!(
                    "GitHub client error: {}",
                    e
                ))))
                .unwrap();
        }
    };

    let engine = eci_deploy::DeployEngine::new(&docker, &github, &state_lock, &state.config);

    let app_name = full_name.split('/').next_back().unwrap_or("app");
    match engine
        .deploy(full_name, app_name, "default", None, None, None)
        .await
    {
        Ok(_) => Response::new(Full::new(Bytes::from(format!("Deployed {}", full_name)))),
        Err(e) => {
            eprintln!("Deploy failed: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(format!("Deploy failed: {}", e))))
                .unwrap()
        }
    }
}

fn verify_signature(secret: &str, payload: &str, signature: &str) -> bool {
    let signature = signature.strip_prefix("sha256=").unwrap_or(signature);

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let expected = hex::encode(result.into_bytes());

    expected == signature
}
