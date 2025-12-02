use anyhow;
use cdi_config::Service;
use cdi_shared::log::ProcessInfo;
// use cdi_shared::event::Event;
// use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    // select,
    sync::mpsc::{self, Receiver, Sender},
    // task::JoinHandle,
    // time,
};

use crate::supervisor::Supervisor;

// use super::utils;

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

// struct ProcessMetadata {
//     id: usize,
//     conn: Connection,
//     handle: JoinHandle<()>,
// }
//
// async fn kill_gracefully(child: &Child) -> std::io::Result<()> {
//     unsafe {
//         let rc = libc::kill(-(child.id().unwrap() as pid_t), libc::SIGTERM);
//
//         if rc == -1 {
//             Err(std::io::Error::last_os_error())
//         } else {
//             Ok(())
//         }
//     }
// }

// async fn process_handler(
//     process_id: usize,
//     service: Service,
//     sender: mpsc::Sender<Message>,
//     mut conn: Connection,
// ) {
//     if let Some((cmd, args)) = utils::split_command_into_parts(&service.cmd) {
//         let mut command = Command::new(cmd.clone());
//
//         // if cmd == "uvicorn" {
//         //     command.process_group(0);
//         // }
//
//         command
//             .args(args)
//             .process_group(0)
//             .stdin(Stdio::null())
//             .stdout(Stdio::piped())
//             .stderr(Stdio::piped());
//
//         if let Some(cwd) = service.cwd {
//             let canonical = std::fs::canonicalize(cwd).unwrap();
//             command.current_dir(canonical);
//         }
//
//         let mut child = match command.spawn() {
//             Ok(c) => c,
//             Err(e) => {
//                 eprint!("Failed to spawn {}: {}", "", e);
//                 return;
//             }
//         };
//
//         let stdout = child.stdout.take().expect("Failed to capture stdout");
//         let stderr = child.stderr.take().expect("Failed to capture stderr");
//
//         let mut stdout_reader = BufReader::new(stdout).lines();
//         let mut stderr_reader = BufReader::new(stderr).lines();
//
//         tokio::spawn(async move {
//             while let Ok(Some(line)) = stdout_reader.next_line().await {
//                 Event::ProcessMessage { process_id, line }.emit();
//             }
//         });
//
//         tokio::spawn(async move {
//             while let Ok(Some(line)) = stderr_reader.next_line().await {
//                 Event::ProcessMessage { process_id, line }.emit();
//             }
//         });
//
//         loop {
//             select! {
//                 biased;
//
//                 Some(msg) = conn.receiver.recv() => {
//                     match msg {
//                         Message::Command(c) => match c {
//                             ServerCommand::Shutdown => match kill_gracefully(&child).await {
//                                 Ok(_) => {
//                                     let exit_code = child.wait().await.unwrap().code();
//                                     eprintln!("Process {} killed with status {:?}", cmd, exit_code);
//                                     return;
//                                 },
//                                 Err(_) => todo!()
//                             }
//
//                         }
//                         _ => todo!()
//                     }
//
//                 }
//
//                 result = child.wait() => {
//                     match result {
//                         Ok(status) => {
//                             let exit_msg = format!("Process {} exited with status: {}", cmd, status);
//                             Event::ProcessMessage {process_id, line: exit_msg.to_string()}.emit();
//
//                             return;
//                         },
//                         Err(_e) => todo!(),
//                     }
//                 }
//
//             }
//         }
//     }
// }

// async fn supervisor(services: Vec<Service>, mut server_conn: Connection) {
//     let mut processes: Vec<ProcessMetadata> = Vec::with_capacity(services.len());
//     for (idx, service) in services.iter().enumerate() {
//         let (supervisor_sender, process_receiver) = mpsc::channel::<Message>(1);
//         let (process_sender, supervisor_receiver) = mpsc::channel::<Message>(1);
//
//         let task = tokio::spawn(process_handler(
//             idx,
//             service.clone(),
//             server_conn.sender.clone(),
//             Connection {
//                 sender: process_sender,
//                 receiver: process_receiver,
//             },
//         ));
//
//         processes.push(ProcessMetadata {
//             id: idx,
//             conn: Connection {
//                 sender: supervisor_sender,
//                 receiver: supervisor_receiver,
//             },
//             handle: task,
//         })
//     }
//
//     while let Some(msg) = server_conn.receiver.recv().await {
//         match msg {
//             Message::Command(cmd) => match cmd {
//                 ServerCommand::Shutdown => {
//                     for proc in processes.iter() {
//                         let _ = proc
//                             .conn
//                             .sender
//                             .send(Message::Command(ServerCommand::Shutdown))
//                             .await;
//                     }
//
//                     return;
//                 }
//             },
//             _ => todo!(),
//         }
//     }
//
//     for proc in processes.iter() {
//         if !proc.handle.is_finished() {
//             eprintln!("Process {} did not exit", proc.id);
//         }
//     }
// }

pub fn serve(services: Vec<ProcessInfo>) -> anyhow::Result<Connection> {
    let (server_sender, client_receiver) = mpsc::channel::<Message>(100);
    let (client_sender, server_receiver) = mpsc::channel::<Message>(1);

    let _ = Supervisor::start(
        services,
        Connection {
            sender: server_sender.clone(),
            receiver: server_receiver,
        },
    );

    drop(server_sender);
    Ok(Connection {
        sender: client_sender,
        receiver: client_receiver,
    })
}
