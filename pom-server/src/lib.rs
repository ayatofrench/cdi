use anyhow;
use tokio::sync::mpsc::Receiver;

pub mod server;
pub use server::ProcessMessage;

#[doc(hidden)]
pub fn start(
    commands: Vec<(String, Vec<String>)>,
) -> anyhow::Result<Receiver<server::ProcessMessage>> {
    server::start(commands)
}
