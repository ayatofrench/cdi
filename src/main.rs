use std::{env, fs::File, io::Read, path::PathBuf};

use miette::{Context as _, IntoDiagnostic};
use pom_config as config;
use pom_server as server;
use pom_tui as tui;

fn get_config() -> miette::Result<config::Config> {
    let mut cwd: PathBuf = env::current_dir()
        .into_diagnostic()
        .with_context(|| "config not found")?;
    cwd.push(".pom.kdl");

    config::Config::load(cwd.as_path())
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    // let mut commands_to_run = Vec::new();
    let mut services = Vec::new();
    let cfg = get_config()?;

    for service in cfg.services.iter() {
        // if let Some(command) = split_command_into_parts(service.cmd.as_str()) {
        //     commands_to_run.push(command);
        // }
        services.push(service.name.clone());
    }

    let conn = server::start(cfg.services).unwrap();
    let _ = tui::run(conn, services).await;

    Ok(())
}
