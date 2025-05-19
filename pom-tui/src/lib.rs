use anyhow;

pub mod app;

#[doc(hidden)]
pub fn start(process_state: &Vec<Vec<String>>) -> anyhow::Result<()> {
    app::start(process_state)
}
