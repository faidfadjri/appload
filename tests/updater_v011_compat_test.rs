//! Test module verifying v0.11.0's own updater logic against the new v0.12.0 release artifacts.
//!
//! Verbatim copies of v0.11.0's updater parsing and asset selection logic from
//! git show v0.11.0:src/updater.rs (Brief 007 Part B).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const GITHUB_RELEASES_PAGE_URL_V011: &str = "https://github.com/faidfadjri/shiply/releases";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct V011ReleaseInfo {
    pub tag_name: String,
    pub version: String,
    pub name: String,
    pub body: String,
    pub published_at: String,
    pub html_url: String,
    pub download_url: String,
    pub asset_name: String,
    pub asset_size: u64,
}

#[derive(Debug, Deserialize)]
struct V011LatestJsonPayload {
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    tag_name: Option<String>,
    #[serde(default)]
    pub_date: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    platforms: Option<HashMap<String, V011LatestJsonPlatform>>,
}

#[derive(Debug, Deserialize)]
struct V011LatestJsonPlatform {
    url: String,
}

#[derive(Debug, Deserialize)]
struct V011GitHubReleasePayload {
    #[serde(alias = "tagName")]
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default, alias = "publishedAt")]
    published_at: Option<String>,
    #[serde(default, alias = "url")]
    html_url: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<V011GitHubAssetPayload>,
}

#[derive(Debug, Clone, Deserialize)]
struct V011GitHubAssetPayload {
    name: String,
    #[serde(alias = "url")]
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

fn v011_normalize_version_tag(raw: &str) -> String {
    raw.trim().trim_start_matches(['v', 'V']).trim().to_string()
}

fn v011_parse_semver_tuple(raw: &str) -> Option<(u64, u64, u64, Option<String>)> {
    let cleaned = v011_normalize_version_tag(raw);
    if cleaned.is_empty() {
        return None;
    }
    let without_build = cleaned.split('+').next().unwrap_or(&cleaned);
    let (core, prerelease) = match without_build.split_once('-') {
        Some((c, pre)) => (c, Some(pre.to_string())),
        None => (without_build, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u64>().ok()?;
    Some((major, minor, patch, prerelease))
}

fn v011_is_newer_semver(current: &str, candidate: &str) -> bool {
    let Some((cur_maj, cur_min, cur_pat, cur_pre)) = v011_parse_semver_tuple(current) else {
        return false;
    };
    let Some((can_maj, can_min, can_pat, can_pre)) = v011_parse_semver_tuple(candidate) else {
        return false;
    };

    match (can_maj, can_min, can_pat).cmp(&(cur_maj, cur_min, cur_pat)) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => match (cur_pre, can_pre) {
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (Some(cp), Some(np)) => np > cp,
            (None, None) => false,
        },
    }
}

fn v011_select_platform_asset<'a>(
    assets: &'a [V011GitHubAssetPayload],
    target_os: &str,
) -> Option<&'a V011GitHubAssetPayload> {
    if assets.is_empty() {
        return None;
    }
    let is_metadata = |name_lower: &str| {
        name_lower.ends_with(".json")
            || name_lower.ends_with(".sha256")
            || name_lower.ends_with(".sig")
            || name_lower.ends_with(".txt")
            || name_lower.ends_with(".ico")
    };

    match target_os {
        "windows" => {
            if let Some(a) = assets.iter().find(|a| {
                let n = a.name.to_lowercase();
                !is_metadata(&n) && (n.contains("windows") || n.ends_with(".exe"))
            }) {
                return Some(a);
            }
        }
        "macos" => {
            if let Some(a) = assets.iter().find(|a| {
                let n = a.name.to_lowercase();
                !is_metadata(&n)
                    && (n.contains("macos") || n.contains("darwin") || n.contains("mac"))
            }) {
                return Some(a);
            }
        }
        "linux" => {
            if let Some(a) = assets.iter().find(|a| {
                let n = a.name.to_lowercase();
                !is_metadata(&n) && (n.contains("linux") || n == "shiply")
            }) {
                return Some(a);
            }
        }
        _ => {}
    }

    assets.iter().find(|a| !is_metadata(&a.name.to_lowercase()))
}

fn v011_parse_github_release_json_for_os(
    raw_json: &str,
    current_version: &str,
    target_os: &str,
) -> Result<Option<V011ReleaseInfo>, String> {
    let trimmed = raw_json.trim();
    if trimmed.is_empty() {
        return Err("Empty response from GitHub Releases API.".to_string());
    }

    // 1. Check if payload matches latest.json manifest format
    if let Ok(latest) = serde_json::from_str::<V011LatestJsonPayload>(trimmed) {
        if latest.platforms.is_some()
            || (latest.version.is_some() && trimmed.contains("\"platforms\""))
        {
            let tag = latest
                .tag_name
                .or(latest.version.clone())
                .unwrap_or_default();
            let candidate_version =
                v011_normalize_version_tag(latest.version.as_deref().unwrap_or(&tag));
            if !v011_is_newer_semver(current_version, &candidate_version) {
                return Ok(None);
            }
            let (target_key, fallback_name) = match target_os {
                "windows" => ("windows-x86_64", "shiply.exe"),
                "macos" => ("darwin-aarch64", "shiply-macos-aarch64"),
                _ => ("linux-x86_64", "shiply-linux-x86_64"),
            };
            let platforms = latest.platforms.unwrap_or_default();
            let download_url = platforms
                .get(target_key)
                .map(|p| p.url.clone())
                .unwrap_or_else(|| {
                    format!("{GITHUB_RELEASES_PAGE_URL_V011}/download/{tag}/{fallback_name}")
                });
            let name = format!("Shiply v{candidate_version}");
            let body = latest
                .notes
                .unwrap_or_else(|| format!("Shiply v{candidate_version} release."));
            let published_at = latest
                .pub_date
                .unwrap_or_else(|| "Recently published".to_string());

            return Ok(Some(V011ReleaseInfo {
                tag_name: if tag.starts_with('v') || tag.starts_with('V') {
                    tag.clone()
                } else {
                    format!("v{candidate_version}")
                },
                version: candidate_version,
                name,
                body,
                published_at,
                html_url: format!("{GITHUB_RELEASES_PAGE_URL_V011}/tag/{tag}"),
                download_url,
                asset_name: fallback_name.to_string(),
                asset_size: 0,
            }));
        }
    }

    // 2. Standard GitHub Release JSON
    let payload: V011GitHubReleasePayload = serde_json::from_str(trimmed)
        .map_err(|e| format!("Failed to parse GitHub Release JSON: {e}"))?;

    if payload.draft {
        return Ok(None);
    }

    let candidate_version = v011_normalize_version_tag(&payload.tag_name);
    if !v011_is_newer_semver(current_version, &candidate_version) {
        return Ok(None);
    }

    let html_url = payload
        .html_url
        .unwrap_or_else(|| GITHUB_RELEASES_PAGE_URL_V011.to_string());

    let (download_url, asset_name, asset_size) =
        if let Some(asset) = v011_select_platform_asset(&payload.assets, target_os) {
            (
                asset.browser_download_url.clone(),
                asset.name.clone(),
                asset.size,
            )
        } else {
            (String::new(), String::new(), 0)
        };

    let name = payload
        .name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("Shiply v{candidate_version}"));

    let body = payload
        .body
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            format!("Release v{candidate_version} is available on GitHub Releases.")
        });

    let published_at = payload
        .published_at
        .unwrap_or_else(|| "Recently published".to_string());

    Ok(Some(V011ReleaseInfo {
        tag_name: payload.tag_name,
        version: candidate_version,
        name,
        body,
        published_at,
        html_url,
        download_url,
        asset_name,
        asset_size,
    }))
}

#[test]
fn test_v011_client_parses_new_v012_latest_json_manifest() {
    // Exact schema generated by .github/workflows/release.yml for v0.12.0
    let release_latest_json = r#"{
      "version": "0.12.0",
      "tag_name": "v0.12.0",
      "pub_date": "2026-11-05T23:00:00Z",
      "notes": "AppLoad v0.12.0 automated multi-platform release.",
      "platforms": {
        "windows-x86_64": {
          "url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/appload.exe"
        },
        "linux-x86_64": {
          "url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/appload-linux-x86_64"
        },
        "darwin-aarch64": {
          "url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/appload-macos-aarch64"
        }
      }
    }"#;

    // Test from v0.11.0's perspective on Windows
    let res = v011_parse_github_release_json_for_os(release_latest_json, "0.11.0", "windows")
        .expect("must parse successfully")
        .expect("must detect newer update");

    assert_eq!(res.version, "0.12.0");
    assert_eq!(res.tag_name, "v0.12.0");
    assert_eq!(
        res.download_url,
        "https://github.com/faidfadjri/appload/releases/download/v0.12.0/appload.exe"
    );
    assert!(res.download_url.ends_with(".exe"));
}

#[test]
fn test_v011_client_parses_github_rest_api_with_compat_assets() {
    // Release JSON payload from GitHub REST API containing all published assets
    let github_api_json = r#"{
      "tag_name": "v0.12.0",
      "name": "AppLoad v0.12.0",
      "body": "AppLoad universal deployment tool.",
      "published_at": "2026-11-05T23:00:00Z",
      "html_url": "https://github.com/faidfadjri/appload/releases/tag/v0.12.0",
      "draft": false,
      "assets": [
        {
          "name": "appload.exe",
          "browser_download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/appload.exe",
          "size": 15000000
        },
        {
          "name": "AppLoad-Setup.exe",
          "browser_download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/AppLoad-Setup.exe",
          "size": 15000000
        },
        {
          "name": "setup.exe",
          "browser_download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/setup.exe",
          "size": 15000000
        },
        {
          "name": "appload-windows-x86_64.exe",
          "browser_download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/appload-windows-x86_64.exe",
          "size": 15000000
        },
        {
          "name": "shiply.exe",
          "browser_download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/shiply.exe",
          "size": 15000000
        },
        {
          "name": "shiply-windows-x86_64.exe",
          "browser_download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/shiply-windows-x86_64.exe",
          "size": 15000000
        },
        {
          "name": "latest.json",
          "browser_download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/latest.json",
          "size": 512
        },
        {
          "name": "SHA256SUMS.txt",
          "browser_download_url": "https://github.com/faidfadjri/appload/releases/download/v0.12.0/SHA256SUMS.txt",
          "size": 1024
        }
      ]
    }"#;

    let res = v011_parse_github_release_json_for_os(github_api_json, "0.11.0", "windows")
        .expect("must parse successfully")
        .expect("must detect newer update");

    assert_eq!(res.version, "0.12.0");
    assert_eq!(res.tag_name, "v0.12.0");
    assert!(res.download_url.ends_with(".exe"));
}
