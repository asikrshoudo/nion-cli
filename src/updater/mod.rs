use anyhow::Result;
use reqwest::Client;
use semver::Version;
use serde::Deserialize;

const GITHUB_REPO: &str = "asikrshoudo/nion-cli";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
}

pub async fn check_for_updates() -> Result<Option<String>> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let resp = client
        .get(&url)
        .header("User-Agent", format!("nion-cli/{}", CURRENT_VERSION))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let release: GithubRelease = resp.json().await?;
    let latest_str = release.tag_name.trim_start_matches('v');

    let current = Version::parse(CURRENT_VERSION)?;
    let latest = match Version::parse(latest_str) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };

    if latest > current {
        Ok(Some(latest.to_string()))
    } else {
        Ok(None)
    }
}

pub async fn force_update() -> Result<()> {
    use crate::ui;

    let spinner = ui::start_spinner("Checking for updates...");
    let result = check_for_updates().await;
    spinner.finish_and_clear();

    match result {
        Ok(Some(version)) => {
            ui::show_update_prompt(&version).await?;
        }
        Ok(None) => {
            ui::print_success(&format!(
                "Nion v{} is already up to date.",
                CURRENT_VERSION
            ));
        }
        Err(e) => {
            ui::print_error(&format!("Could not check for updates: {}", e));
            println!("  Update manually: npm install -g nion-cli");
        }
    }

    Ok(())
}

pub async fn download_and_replace(version: &str) -> Result<()> {
    use crate::ui;
    use std::env;

    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    // Binary names must match release.yml exactly
    let binary_name = match (os, arch) {
        ("linux", "x86_64")  => "nion-x86_64-linux",
        ("linux", "aarch64") => "nion-aarch64-linux",  // Termux
        ("macos", "x86_64")  => "nion-x86_64-macos",
        ("macos", "aarch64") => "nion-aarch64-macos",
        ("windows", _) => {
            println!("  Download the new .exe from:");
            println!(
                "  https://github.com/{}/releases/tag/v{}",
                GITHUB_REPO, version
            );
            return Ok(());
        }
        _ => {
            ui::print_error("Auto-update not available for this platform.");
            println!("  Update manually: npm install -g nion-cli");
            return Ok(());
        }
    };

    let url = format!(
        "https://github.com/{}/releases/download/v{}/{}",
        GITHUB_REPO, version, binary_name
    );

    let spinner = ui::start_spinner(&format!("Downloading v{}...", version));

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let resp = client
        .get(&url)
        .header("User-Agent", "nion-cli")
        .send()
        .await;

    spinner.finish_and_clear();

    match resp {
        Ok(r) if r.status().is_success() => {
            let bytes = r.bytes().await?;
            let current_exe = env::current_exe()?;
            let tmp = current_exe.with_extension("nion.tmp");

            std::fs::write(&tmp, &bytes)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
            }

            std::fs::rename(&tmp, &current_exe)?;
            ui::print_success(&format!("Updated to v{}. Please restart nion.", version));
        }
        Ok(r) => {
            anyhow::bail!("Download failed: HTTP {}", r.status());
        }
        Err(e) => {
            anyhow::bail!("Download error: {}", e);
        }
    }

    Ok(())
}
