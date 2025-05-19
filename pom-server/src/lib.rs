use anyhow;
use tokio::sync::mpsc::Receiver;

mod server;
pub use server::Process;

#[doc(hidden)]
pub fn start(commands: Vec<(String, Vec<String>)>) -> anyhow::Result<Receiver<server::Process>> {
    server::start(commands)
}
