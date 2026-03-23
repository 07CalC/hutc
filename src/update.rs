use std::time::Duration;

const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const UPDATE_CMD: &str = "cargo install hutc";

pub async fn update_available_message() -> Option<String> {
    let latest = fetch_latest_version().await?;
    if is_newer_version(&latest, CURRENT_VERSION) {
        Some(format!(
            "update available ({CURRENT_VERSION} -> {latest}), run `{UPDATE_CMD}` to update"
        ))
    } else {
        None
    }
}

async fn fetch_latest_version() -> Option<String> {
    let url = format!("https://crates.io/api/v1/crates/{CRATE_NAME}");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1200))
        .build()
        .ok()?;

    let response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::USER_AGENT,
            format!("hutc/{CURRENT_VERSION}"),
        )
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        return None;
    }

    let payload: serde_json::Value = response.json().await.ok()?;
    payload
        .get("crate")
        .and_then(|krate| krate.get("max_version"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest = parse_version(latest);
    let current = parse_version(current);
    let (latest_parts, latest_prerelease) = match latest {
        Some(parts) => parts,
        None => return false,
    };
    let (current_parts, current_prerelease) = match current {
        Some(parts) => parts,
        None => return false,
    };

    let max_len = latest_parts.len().max(current_parts.len());
    for index in 0..max_len {
        let latest = latest_parts.get(index).copied().unwrap_or(0);
        let current = current_parts.get(index).copied().unwrap_or(0);
        if latest > current {
            return true;
        }
        if latest < current {
            return false;
        }
    }

    !latest_prerelease && current_prerelease
}

fn parse_version(version: &str) -> Option<(Vec<u64>, bool)> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let (core, prerelease) = match version.split_once('-') {
        Some((core, _)) => (core, true),
        None => (version, false),
    };
    let parts = core
        .split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    if parts.is_empty() {
        return None;
    }

    Some((parts, prerelease))
}
