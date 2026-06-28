use serde::{Deserialize, Serialize};

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/Sparrived/galncelet/releases/latest";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub latest_tag: Option<String>,
    pub release_name: Option<String>,
    pub release_url: Option<String>,
    pub published_at: Option<String>,
    pub has_update: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    html_url: String,
    published_at: Option<String>,
}

#[tauri::command]
pub fn check_for_updates(app: tauri::AppHandle) -> Result<UpdateCheckResult, String> {
    let current_version = app.package_info().version.to_string();
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("Galncelet/{}", current_version))
        .build()
        .map_err(|e| format!("Failed to create update checker: {e}"))?;

    let release = client
        .get(GITHUB_LATEST_RELEASE_URL)
        .send()
        .map_err(|e| format!("Failed to check GitHub Releases: {e}"))?
        .error_for_status()
        .map_err(|e| format!("GitHub Releases returned an error: {e}"))?
        .json::<GitHubRelease>()
        .map_err(|e| format!("Failed to parse latest release: {e}"))?;

    let latest_version = normalize_version(&release.tag_name);
    let has_update = compare_versions(&latest_version, &current_version).is_gt();

    Ok(UpdateCheckResult {
        current_version,
        latest_version: Some(latest_version),
        latest_tag: Some(release.tag_name),
        release_name: release.name,
        release_url: Some(release.html_url),
        published_at: release.published_at,
        has_update,
    })
}

fn normalize_version(tag: &str) -> String {
    tag.trim().trim_start_matches('v').trim_start_matches('V').to_string()
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let left_parts = version_parts(left);
    let right_parts = version_parts(right);
    let max_len = left_parts.len().max(right_parts.len());

    for index in 0..max_len {
        let left_part = *left_parts.get(index).unwrap_or(&0);
        let right_part = *right_parts.get(index).unwrap_or(&0);
        match left_part.cmp(&right_part) {
            std::cmp::Ordering::Equal => {}
            ordering => return ordering,
        }
    }

    std::cmp::Ordering::Equal
}

fn version_parts(version: &str) -> Vec<u64> {
    version
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{compare_versions, normalize_version};

    #[test]
    fn strips_release_tag_prefix() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version("V1.2.3"), "1.2.3");
    }

    #[test]
    fn compares_numeric_versions() {
        assert!(compare_versions("0.2.0", "0.1.9").is_gt());
        assert!(compare_versions("1.0", "1.0.0").is_eq());
        assert!(compare_versions("1.0.0", "1.0.1").is_lt());
    }
}
