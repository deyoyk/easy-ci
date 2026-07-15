use eci_core::config::Config;
use eci_core::error::{EciError, Result};
use octocrab::params::repos::Reference;
use octocrab::Octocrab;
use std::path::PathBuf;

pub struct GitHubClient {
    octocrab: Octocrab,
}

#[derive(Debug)]
pub struct RepoInfo {
    pub full_name: String,
    pub name: String,
    pub description: Option<String>,
    pub default_branch: String,
    pub clone_url: String,
}

impl GitHubClient {
    pub async fn new(config: &Config) -> Result<Self> {
        let octocrab = Octocrab::builder()
            .personal_token(config.github.token.clone())
            .build()
            .map_err(|e| EciError::GitHub(format!("Failed to create client: {}", e)))?;
        Ok(Self { octocrab })
    }

    pub async fn list_repos(&self, owner: &str) -> Result<Vec<RepoInfo>> {
        let repos = self
            .octocrab
            .users(owner)
            .repos()
            .send()
            .await
            .map_err(|e| EciError::GitHub(format!("Failed to list repos: {}", e)))?;

        Ok(repos
            .items
            .into_iter()
            .map(|r| RepoInfo {
                full_name: r.full_name.unwrap_or_default(),
                name: r.name,
                description: r.description,
                default_branch: r.default_branch.unwrap_or_else(|| "main".to_string()),
                clone_url: r.clone_url.map(|u| u.to_string()).unwrap_or_default(),
            })
            .collect())
    }

    pub async fn get_branch_sha(&self, owner: &str, repo: &str, branch: &str) -> Result<String> {
        let reference = self
            .octocrab
            .repos(owner, repo)
            .get_ref(&Reference::Branch(branch.to_string()))
            .await
            .map_err(|e| EciError::GitHub(format!("Failed to get ref: {}", e)))?;
        match reference.object {
            octocrab::models::repos::Object::Commit { sha, .. } => Ok(sha),
            octocrab::models::repos::Object::Tag { sha, .. } => Ok(sha),
            _ => Err(EciError::GitHub("Unexpected ref type".to_string())),
        }
    }

    pub fn clone_repo(clone_url: &str, dest: &PathBuf, token: &str) -> Result<()> {
        let url_with_token = clone_url
            .replacen("https://", &format!("https://x:{}@", token), 1);

        let _ = std::fs::remove_dir_all(dest);
        git2::Repository::clone(&url_with_token, dest)
            .map_err(|e| EciError::GitHub(format!("Failed to clone repo: {}", e)))?;
        Ok(())
    }
}
