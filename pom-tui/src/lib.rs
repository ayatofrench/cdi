use anyhow;
use pom_server::ProcessMessage;
use tokio::sync::mpsc::Receiver;

pub mod app;

#[doc(hidden)]
pub async fn run(conn: Receiver<ProcessMessage>) -> anyhow::Result<()> {
    app::run(conn).await
}
