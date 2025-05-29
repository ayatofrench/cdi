use anyhow;
use pom_config::Service;

pub mod server;
pub use server::Connection;

mod process;
mod supervisor;
mod utils;

#[doc(hidden)]
pub fn serve(commands: Vec<Service>) -> anyhow::Result<Connection> {
    server::serve(commands)
}
