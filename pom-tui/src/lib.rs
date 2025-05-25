use anyhow;
use pom_server::Connection;

pub mod app;

#[doc(hidden)]
pub async fn run(conn: Connection, services: Vec<String>) -> anyhow::Result<()> {
    app::run(conn, services).await
}
