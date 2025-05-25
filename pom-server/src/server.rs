use anyhow;
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    select,
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

pub struct Connection {
    pub sender: Sender<Message>,
    pub receiver: Receiver<Message>,
}

pub enum ServerCommand {
    Shutdown,
}

pub enum Message {
    Command(ServerCommand),
    ProcessOutput { process_id: usize, line: String },
}

struct ProcessMetadata {
    id: usize,
    conn: Connection,
    handle: JoinHandle<()>,
}

async fn process_handler(
    process_id: usize,
    cmd: String,
    args: Vec<String>,
    sender: mpsc::Sender<Message>,
    mut conn: Connection,
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
                .send(Message::ProcessOutput { process_id, line })
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
                .send(Message::ProcessOutput { process_id, line })
                .await
                .is_err()
            {
                eprint!("Reciever dropped for stderr of {}", process_id);
                break;
            }
        }
    });

    // kill_signal.is_terminated
    // kill_signal.await

    loop {
        select! {
            biased;


            Some(msg) = conn.receiver.recv() => {
                match msg {
                    Message::Command(c) => match c {
                        ServerCommand::Shutdown => match child.kill().await {
                            Ok(_) => {
                                println!("Process {} killed with status", cmd);
                                return;
                            },
                            Err(_) => todo!()
                        }

                    }
                    _ => todo!()
                }

            }

            result = child.wait() => {
                match result {
                    Ok(status) => {
                        let exit_msg = format!("Process {} exited with status: {}", cmd, status);
                        let _ = sender
                            .send(Message::ProcessOutput {
                                process_id,
                                line: exit_msg.to_string(),
                            })
                            .await;

                        return;
                    },
                    Err(_e) => todo!(),
                    }
            }

        }
    }
}

async fn supervisor(commands: Vec<(String, Vec<String>)>, mut server_conn: Connection) {
    let mut processes: Vec<ProcessMetadata> = Vec::with_capacity(commands.len());
    for (idx, value) in commands.iter().enumerate() {
        let (supervisor_sender, process_receiver) = mpsc::channel::<Message>(1);
        let (process_sender, supervisor_receiver) = mpsc::channel::<Message>(1);

        let (cmd, args) = value.to_owned();
        let task = tokio::spawn(process_handler(
            idx,
            cmd,
            args,
            server_conn.sender.clone(),
            Connection {
                sender: process_sender,
                receiver: process_receiver,
            },
        ));

        processes.push(ProcessMetadata {
            id: idx,
            conn: Connection {
                sender: supervisor_sender,
                receiver: supervisor_receiver,
            },
            handle: task,
        })
    }

    while let Some(msg) = server_conn.receiver.recv().await {
        match msg {
            Message::Command(cmd) => match cmd {
                ServerCommand::Shutdown => {
                    for proc in processes.iter() {
                        let _ = proc
                            .conn
                            .sender
                            .send(Message::Command(ServerCommand::Shutdown))
                            .await;
                    }

                    return;
                }
            },
            _ => todo!(),
        }
    }
}

pub fn start(commands: Vec<(String, Vec<String>)>) -> anyhow::Result<Connection> {
    let (server_sender, client_receiver) = mpsc::channel::<Message>(100);
    let (client_sender, server_receiver) = mpsc::channel::<Message>(1);

    tokio::spawn(supervisor(
        commands,
        Connection {
            sender: server_sender.clone(),
            receiver: server_receiver,
        },
    ));

    drop(server_sender);
    Ok(Connection {
        sender: client_sender,
        receiver: client_receiver,
    })
}
