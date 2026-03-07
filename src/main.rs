mod cli;
mod config;
mod providers;
mod session;
mod ui;
mod updater;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Show startup animation
    ui::startup_animation().await;

    // First-run: ask for name + optional setup
    if let Err(e) = config::run_first_time_setup().await {
        ui::print_error(&format!("Setup error: {}", e));
    }

    // Background update check (non-blocking, 4s timeout)
    let update_handle = tokio::spawn(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(4),
            updater::check_for_updates(),
        )
        .await
    });

    // Run the CLI
    if let Err(e) = cli::run().await {
        ui::print_error(&format!("{}", e));
        std::process::exit(1);
    }

    // After command finishes, show update prompt if available
    if let Ok(Ok(Ok(Some(new_version)))) = update_handle.await {
        if let Err(e) = ui::show_update_prompt(&new_version).await {
            ui::print_error(&format!("Update error: {}", e));
        }
    }

    Ok(())
}
