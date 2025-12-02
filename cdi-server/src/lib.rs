use anyhow;

pub mod server;
use cdi_shared::log::ProcessInfo;
pub use server::Connection;

mod process;
mod store;
mod supervisor;
mod utils;

#[doc(hidden)]
pub fn serve(processes: Vec<ProcessInfo>) -> anyhow::Result<Connection> {
    server::serve(processes)
}
