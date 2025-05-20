use anyhow;
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::{self, Receiver},
};

pub struct ProcessMessage {
    pub process_id: i32,
    pub line: String,
    pub stream: String,
}

async fn process_handler(
    process_id: i32,
    cmd: String,
    args: Vec<String>,
    sender: mpsc::Sender<ProcessMessage>,
) {
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
                .send(ProcessMessage {
                    process_id,
                    line,
                    stream: "stream".to_string(),
                })
                .await
                .is_err()
            {
                eprint!("Reciever dropped for stdout of {}", process_id);
                break;
            }
        }
    });

    let sender_stderr = sender.clone();
    // let process_id_stderr = cmd.clone();
    tokio::spawn(async move {
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            if sender_stderr
                .send(ProcessMessage {
                    process_id,
                    line,
                    stream: "stream".to_string(),
                })
                .await
                .is_err()
            {
                eprint!("Reciever dropped for stderr of {}", process_id);
                break;
            }
        }
    });

    match child.wait().await {
        Ok(status) => {
            let exit_msg = format!("Process {} exited with status: {}", cmd, status);
            let _ = sender
                .send(ProcessMessage {
                    process_id,
                    line: exit_msg.to_string(),
                    stream: "stream".to_string(),
                })
                .await;
        }
        Err(e) => todo!(),
    }
}

pub fn start(commands: Vec<(String, Vec<String>)>) -> anyhow::Result<Receiver<ProcessMessage>> {
    let (sender, reciever) = mpsc::channel(100);
    for (idx, value) in commands.iter().enumerate() {
        let (cmd, args) = value.to_owned();
        tokio::spawn(process_handler(
            idx.try_into().unwrap(),
            cmd,
            args,
            sender.clone(),
        ));
    }

    drop(sender);
    Ok(reciever)
}
