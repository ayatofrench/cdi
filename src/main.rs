use pom_server as server;
use pom_tui as tui;

#[tokio::main]
async fn main() {
    let commands_to_run = vec![
        ("ls".to_string(), vec!["-la".to_string()]),
        ("sleep".to_string(), vec!["5".to_string()]),
    ];

    let mut process_output_state: Box<Vec<Vec<String>>> =
        Box::new(Vec::with_capacity(commands_to_run.len()));
    for _ in 0..commands_to_run.len() {
        process_output_state.push(vec![String::new()]);
    }

    println!("len: {}", process_output_state.len());

    let mut conn = server::start(commands_to_run).unwrap();
    tokio::spawn(async {
        while let Some(output) = conn.recv().await {
            let process_id: usize = output.process_id.try_into().unwrap();
            process_output_state[process_id].push(output.line.clone());
            // process_output = process_ouput;
            // process_output_state[output.process_id] = process_output_state[output.process_id]
            // println!("[{}:{}] {}", output.process_id, output.stream, output.line);
        }
    });

    let _ = tui::start(&process_output_state);
}
