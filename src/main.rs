use miette::{Context as _, IntoDiagnostic};
use std::{env, path::PathBuf};

use cdi_config as config;
use cdi_server as server;
use cdi_shared::{event::Event, log::ProcessInfo};
use cdi_tui as tui;

fn get_config() -> miette::Result<config::Config> {
    let mut cwd: PathBuf = env::current_dir()
        .into_diagnostic()
        .with_context(|| "config not found")?;
    cwd.push(".cdi.kdl");

    config::Config::load(cwd.as_path())
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    Event::init();
    let mut services = Vec::new();
    let cfg = get_config()?;

    // for service in cfg.services.iter() {
    //     services.push(service.name.clone());
    // }

    let process_infos: Vec<ProcessInfo> = cfg
        .services
        .iter()
        .map(|s| ProcessInfo::new(s.name.clone(), s.cmd.clone(), s.cwd.clone()))
        .collect();

    let conn = server::serve(process_infos.clone()).unwrap();
    let _ = tui::run(conn, process_infos).await;

    Ok(())
}
