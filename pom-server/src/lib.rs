use anyhow;
use pom_config::Service;

pub mod server;
pub use server::Connection;

mod utils;

#[doc(hidden)]
pub fn start(commands: Vec<Service>) -> anyhow::Result<Connection> {
    server::start(commands)
}
