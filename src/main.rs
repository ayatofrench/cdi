use std::process::Stdio;

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc,
};

struct Process {
    process_id: String,
    line: String,
    stream: String,
}

async fn process_handler(cmd: String, args: Vec<String>, sender: mpsc::Sender<Process>) {
    // maybe can do something better here need to look into this
    // Also need to look into why command can be chained after new but not with it.
    // I think I have a general understanding but need to research it.
    let mut command = Command::new(cmd.clone());
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprint!("Failed to spawn {}: {}", cmd.clone(), e);
            return;
        }
    };

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let sender_stdout = sender.clone();
    let process_id_stdout = cmd.clone();
    tokio::spawn(async move {
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            if sender_stdout
                .send(Process {
                    process_id: process_id_stdout.clone(),
                    line,
                    stream: "stream".to_string(),
                })
                .await
                .is_err()
            {
                eprint!("Reciever dropped for stdout of {}", process_id_stdout);
                break;
            }
        }
    });

    let sender_stderr = sender.clone();
    let process_id_stderr = cmd.clone();
    tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            if sender_stderr
                .send(Process {
                    process_id: process_id_stderr.clone(),
                    line,
                    stream: "stream".to_string(),
                })
                .await
                .is_err()
            {
                eprint!("Reciever dropped for stderr of {}", process_id_stderr);
                break;
            }
        }
    });

    match child.wait().await {
        Ok(status) => println!("Process {} exited with status: {}", cmd, status),
        Err(e) => todo!(),
    }
}

#[tokio::main]
async fn main() {
    let commands_to_run = vec![
        ("ls".to_string(), vec!["-la".to_string()]),
        ("sleep".to_string(), vec!["5".to_string()]),
    ];

    let (sender, mut reciever) = mpsc::channel(100);
    for (cmd, args) in commands_to_run {
        tokio::spawn(process_handler(cmd, args, sender.clone()));
    }

    drop(sender);

    while let Some(output) = reciever.recv().await {
        println!("[{}:{}] {}", output.process_id, output.stream, output.line);
    }

    println!("Hello, world!");
}
