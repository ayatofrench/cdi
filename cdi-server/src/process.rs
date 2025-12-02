use std::process::Stdio;

use anyhow::Result;
use libc::pid_t;
use cdi_config::Service;
use cdi_shared::{event::{store::StoreEvent, ui::TuiEvent}, log::ProcessInfo};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    select,
    sync::mpsc,
    task::JoinHandle,
};

use crate::{
    Connection,
    server::{Message, ServerCommand},
    utils,
};

pub(super) struct Process {
    info: ProcessInfo
    sender: mpsc::Sender<Message>,
    conn: Connection,
}

impl Process {
    pub fn start(
        process_info: ProcessInfo,
        sender: mpsc::Sender<Message>,
        conn: Connection,
    ) -> Result<JoinHandle<()>> {
        let process = Self {
            info: process_info,
            sender,
            conn,
        };
        let task = tokio::spawn(process.run());

        Ok(task)
    }

    async fn run(mut self) {
        if let Some((cmd, args)) = utils::split_command_into_parts(&self.info.command) {
            let mut command = Command::new(cmd.clone());

            command
                .args(args)
                .process_group(0)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            if let Some(cwd) = self.info.cwd {
                let canonical = std::fs::canonicalize(cwd).unwrap();
                command.current_dir(canonical);
            }

            let mut child = match command.spawn() {
                Ok(c) => c,
                Err(e) => {
                    eprint!("Failed to spawn {}: {}", "", e);
                    return;
                }
            };

            let stdout = child.stdout.take().expect("Failed to capture stdout");
            let stderr = child.stderr.take().expect("Failed to capture stderr");

            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();

            tokio::spawn(async move {
                while let Ok(Some(line)) = stdout_reader.next_line().await {
                    StoreEvent::AppendLog {
                        process_id: self.info.id,
                        content: line,
                        stream: cdi_shared::log::Stream::Stdout,
                    }
                    .emit();
                }
            });

            tokio::spawn(async move {
                while let Ok(Some(line)) = stderr_reader.next_line().await {
                    StoreEvent::AppendLog {
                        process_id: self.info.id,
                        content: line,
                        stream: cdi_shared::log::Stream::Stderr,
                    }
                    .emit();
                }
            });

            loop {
                select! {
                    biased;

                    Some(msg) = self.conn.receiver.recv() => {
                        match msg {
                            Message::Command(c) => match c {
                                ServerCommand::Shutdown => match Self::kill_gracefully(&child).await {
                                    Ok(_) => {
                                        let exit_code = child.wait().await.unwrap().code();
                                        eprintln!("Process {} killed with status {:?}", cmd, exit_code);
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
                                StoreEvent::AppendLog {process_id: self.info.id, content: exit_msg.to_string(), stream: cdi_shared::log::Stream::Stdout,}.emit();

                                return;
                            },
                            Err(_e) => todo!(),
                        }
                    }
                }
            }
        }
    }

    async fn kill_gracefully(child: &Child) -> std::io::Result<()> {
        unsafe {
            let rc = libc::kill(-(child.id().unwrap() as pid_t), libc::SIGTERM);

            if rc == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
}
