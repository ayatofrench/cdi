use anyhow;
use cdi_server::Connection;
use cdi_shared::log::ProcessInfo;

pub mod app;
mod signals;

#[doc(hidden)]
pub async fn run(conn: Connection, services: Vec<ProcessInfo>) -> anyhow::Result<()> {
    app::run(conn, services).await
}
