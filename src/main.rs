mod agent;
mod cli;
mod config;
mod providers;
mod session;
mod ui;
mod updater;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    ui::startup_animation().await;

    // First-run: ask for name
    if let Err(e) = config::run_first_time_setup().await {
        ui::print_error(&format!("Setup error: {}", e));
    }

    // Background update check
    let update_handle = tokio::spawn(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(4),
            updater::check_for_updates(),
        )
        .await
    });

    // Run CLI
    if let Err(e) = cli::run().await {
        ui::print_error(&format!("{}", e));
        std::process::exit(1);
    }

    // Show update prompt after command finishes
    if let Ok(Ok(Ok(Some(new_version)))) = update_handle.await {
        if let Err(e) = ui::show_update_prompt(&new_version).await {
            ui::print_error(&format!("Update error: {}", e));
        }
    }

    Ok(())
}
