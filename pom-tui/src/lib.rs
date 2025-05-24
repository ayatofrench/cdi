use anyhow;
use pom_server::Connection;

pub mod app;

#[doc(hidden)]
pub async fn run(conn: Connection) -> anyhow::Result<()> {
    app::run(conn).await
}
