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

    pub async fn list_my_repos(&self) -> Result<Vec<RepoInfo>> {
        let mut all_repos = Vec::new();
        let mut page = 1u8;

        loop {
            let repos = self
                .octocrab
                .current()
                .list_repos_for_authenticated_user()
                .per_page(100)
                .page(page)
                .send()
                .await
                .map_err(|e| EciError::GitHub(format!("Failed to list repos: {}", e)))?;

            let items: Vec<RepoInfo> = repos
                .items
                .into_iter()
                .map(|r| RepoInfo {
                    full_name: r.full_name.unwrap_or_default(),
                    name: r.name,
                    description: r.description,
                    default_branch: r.default_branch.unwrap_or_else(|| "main".to_string()),
                    clone_url: r.clone_url.map(|u| u.to_string()).unwrap_or_default(),
                })
                .collect();

            let count = items.len();
            all_repos.extend(items);

            if count < 100 {
                break;
            }
            page = page.saturating_add(1);
        }

        Ok(all_repos)
    }

    pub async fn list_org_repos(&self, org: &str) -> Result<Vec<RepoInfo>> {
        let mut all_repos = Vec::new();
        let mut page = 1u8;

        loop {
            let repos = self
                .octocrab
                .orgs(org)
                .list_repos()
                .per_page(100)
                .page(page)
                .send()
                .await
                .map_err(|e| EciError::GitHub(format!("Failed to list org repos: {}", e)))?;

            let items: Vec<RepoInfo> = repos
                .items
                .into_iter()
                .map(|r| RepoInfo {
                    full_name: r.full_name.unwrap_or_default(),
                    name: r.name,
                    description: r.description,
                    default_branch: r.default_branch.unwrap_or_else(|| "main".to_string()),
                    clone_url: r.clone_url.map(|u| u.to_string()).unwrap_or_default(),
                })
                .collect();

            let count = items.len();
            all_repos.extend(items);

            if count < 100 {
                break;
            }
            page = page.saturating_add(1);
        }

        Ok(all_repos)
    }

    pub async fn list_all_repos(&self) -> Result<Vec<RepoInfo>> {
        let mut all_repos = self.list_my_repos().await?;

        // Get unique orgs from repos we already have
        let mut orgs: std::collections::HashSet<String> = std::collections::HashSet::new();
        for repo in &all_repos {
            if let Some(owner) = repo.full_name.split('/').next() {
                // Only fetch org repos if we don't already have them
                if !all_repos
                    .iter()
                    .any(|r| r.full_name.starts_with(&format!("{}/", owner)))
                {
                    orgs.insert(owner.to_string());
                }
            }
        }

        // Fetch repos from each org
        for org in orgs {
            if let Ok(org_repos) = self.list_org_repos(&org).await {
                for repo in org_repos {
                    if !all_repos.iter().any(|r| r.full_name == repo.full_name) {
                        all_repos.push(repo);
                    }
                }
            }
        }

        // Sort by full_name for consistent display
        all_repos.sort_by(|a, b| a.full_name.cmp(&b.full_name));

        Ok(all_repos)
    }

    pub fn clone_repo(clone_url: &str, dest: &PathBuf, token: &str) -> Result<()> {
        let url_with_token = clone_url.replacen("https://", &format!("https://x:{}@", token), 1);

        let _ = std::fs::remove_dir_all(dest);
        git2::Repository::clone(&url_with_token, dest)
            .map_err(|e| EciError::GitHub(format!("Failed to clone repo: {}", e)))?;
        Ok(())
    }
}
