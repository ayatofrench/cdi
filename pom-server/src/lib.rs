use anyhow;
use tokio::sync::mpsc::Receiver;

pub mod server;
pub use server::Connection;

#[doc(hidden)]
pub fn start(commands: Vec<(String, Vec<String>)>) -> anyhow::Result<Connection> {
    server::start(commands)
}
