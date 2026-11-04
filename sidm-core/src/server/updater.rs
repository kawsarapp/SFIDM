use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

const GITHUB_REPO: &str = "kawsarapp/SFIDM";
const GITHUB_LATEST_API: &str = "https://api.github.com/repos/kawsarapp/SFIDM/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_name: Option<String>,
    pub release_url: Option<String>,
    pub download_url: Option<String>,
    pub release_notes: Option<String>,
    pub checked_at: Option<String>,
}

impl Default for UpdateInfo {
    fn default() -> Self {
        Self {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: None,
            update_available: false,
            release_name: None,
            release_url: None,
            download_url: None,
            release_notes: None,
            checked_at: None,
        }
    }
}

pub struct UpdateChecker {
    status: Arc<RwLock<UpdateInfo>>,
}

impl UpdateChecker {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(UpdateInfo::default())),
        }
    }

    pub fn status(&self) -> Arc<RwLock<UpdateInfo>> {
        self.status.clone()
    }

    pub async fn get_status(&self) -> UpdateInfo {
        self.status.read().await.clone()
    }

    pub async fn check_for_updates(&self) -> UpdateInfo {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent(format!("SIDM-Update-Checker/{}", CURRENT_VERSION))
            .build()
        {
            Ok(c) => c,
            Err(_) => return self.get_status().await,
        };

        let response = match client
            .get(GITHUB_LATEST_API)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to check GitHub releases: {}", e);
                return self.get_status().await;
            }
        };

        if !response.status().is_success() {
            tracing::warn!("GitHub API returned non-success: {}", response.status());
            return self.get_status().await;
        }

        let json_val: serde_json::Value = match response.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to parse GitHub release JSON: {}", e);
                return self.get_status().await;
            }
        };

        let tag_name = json_val
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let release_name = json_val
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let release_url = json_val
            .get("html_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| Some(format!("https://github.com/{}/releases/latest", GITHUB_REPO)));
        let release_notes = json_val
            .get("body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut download_url = None;
        if let Some(assets) = json_val.get("assets").and_then(|a| a.as_array()) {
            for asset in assets {
                let name = asset.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let url = asset
                    .get("browser_download_url")
                    .and_then(|u| u.as_str())
                    .map(|s| s.to_string());
                #[cfg(windows)]
                {
                    if name.ends_with(".exe") {
                        download_url = url;
                        break;
                    }
                }
                #[cfg(not(windows))]
                {
                    let clean_tag = tag_name.trim_start_matches('v');
                    if name.ends_with(".deb") && (name.contains(clean_tag) || download_url.is_none()) {
                        download_url = url;
                    }
                }
            }
        }

        let update_available = compare_versions(&tag_name, CURRENT_VERSION) == std::cmp::Ordering::Greater;

        let info = UpdateInfo {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: if tag_name.is_empty() {
                None
            } else {
                Some(tag_name.clone())
            },
            update_available,
            release_name,
            release_url,
            download_url,
            release_notes,
            checked_at: Some(chrono::Utc::now().to_rfc3339()),
        };

        *self.status.write().await = info.clone();
        if update_available {
            tracing::info!(
                "🚀 New SIDM update found: {} (current: {})",
                tag_name,
                CURRENT_VERSION
            );
        }

        info
    }

    pub fn start_background_loop(self: Arc<Self>) {
        tokio::spawn(async move {
            // Initial delay of 5 seconds so startup is fast and non-blocking
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            self.check_for_updates().await;

            // Check every 6 hours
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(6 * 3600));
            loop {
                interval.tick().await;
                self.check_for_updates().await;
            }
        });
    }
}

pub fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    let parse_nums = |s: &str| -> Vec<u64> {
        let clean = s.trim().trim_start_matches('v').trim_start_matches('V');
        clean
            .split('.')
            .filter_map(|part| {
                let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
                digits.parse::<u64>().ok()
            })
            .collect()
    };
    let nums1 = parse_nums(v1);
    let nums2 = parse_nums(v2);
    let max_len = nums1.len().max(nums2.len());
    for i in 0..max_len {
        let n1 = nums1.get(i).copied().unwrap_or(0);
        let n2 = nums2.get(i).copied().unwrap_or(0);
        if n1 != n2 {
            return n1.cmp(&n2);
        }
    }
    std::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert_eq!(compare_versions("v1.1.6", "1.1.5"), std::cmp::Ordering::Greater);
        assert_eq!(compare_versions("1.1.5", "v1.1.5"), std::cmp::Ordering::Equal);
        assert_eq!(compare_versions("1.1.4", "1.1.5"), std::cmp::Ordering::Less);
        assert_eq!(compare_versions("v2.0.0", "1.9.9"), std::cmp::Ordering::Greater);
        assert_eq!(compare_versions("1.2", "1.1.9"), std::cmp::Ordering::Greater);
        assert_eq!(compare_versions("v1.1.5", "v1.1.5"), std::cmp::Ordering::Equal);
        assert_eq!(compare_versions("v1.2.0-rc1", "1.1.5"), std::cmp::Ordering::Greater);
    }
}
