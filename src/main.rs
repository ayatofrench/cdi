use pom_config as config;
use pom_server as server;
use pom_tui as tui;

#[tokio::main]
async fn main() {
    let commands_to_run = vec![
        ("ls".to_string(), vec!["-la".to_string()]),
        ("sleep".to_string(), vec!["20".to_string()]),
    ];
    let conn = server::start(commands_to_run).unwrap();
    let _ = tui::run(conn).await;
}
