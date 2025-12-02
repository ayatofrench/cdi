use anyhow::{Error, Result};
use cdi_shared::log::ProcessInfo;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    Connection,
    process::Process,
    server::{Message, ServerCommand},
};

pub(super) struct Supervisor {
    processes: Vec<ProcessContext>,
    server_conn: Connection,
}

struct ProcessContext {
    info: ProcessInfo,
    conn: Connection,
    handle: JoinHandle<()>,
}

impl Supervisor {
    pub fn start(services: Vec<ProcessInfo>, server_conn: Connection) -> Result<()> {
        let supervisor = Self {
            processes: Vec::with_capacity(services.len()),
            server_conn,
        };

        let _ = tokio::spawn(supervisor.run(services));

        Ok(())
    }

    async fn run(mut self, process_infos: Vec<ProcessInfo>) -> Result<()> {
        // let mut processes: Vec<ProcessContext> = Vec::with_capacity(process_infos.len());
        let processes: Vec<ProcessContext> = process_infos.iter().map(|proc_info| {
            let (supervisor_sender, process_receiver) = mpsc::channel::<Message>(1);
            let (process_sender, supervisor_receiver) = mpsc::channel::<Message>(1);

            let task = Process::start(
                proc_info.clone(),
                self.server_conn.sender.clone(),
                Connection {
                    sender: process_sender,
                    receiver: process_receiver,
                },
            ).unwrap();

            ProcessContext {
                info: proc_info.clone(),
                conn: Connection {
                    sender: supervisor_sender,
                    receiver: supervisor_receiver,
                },
                handle: task,
            }

        }).collect();
        //
        // for (idx, service) in processes.iter().enumerate() {
        //     let (supervisor_sender, process_receiver) = mpsc::channel::<Message>(1);
        //     let (process_sender, supervisor_receiver) = mpsc::channel::<Message>(1);
        //
        //     let task = Process::start(
        //         idx,
        //         service.clone(),
        //         self.server_conn.sender.clone(),
        //         Connection {
        //             sender: process_sender,
        //             receiver: process_receiver,
        //         },
        //     )?;
        //
        //     processes.push(ProcessContext {
        //         id: idx,
        //         conn: Connection {
        //             sender: supervisor_sender,
        //             receiver: supervisor_receiver,
        //         },
        //         handle: task,
        //     })
        // }

        while let Some(msg) = self.server_conn.receiver.recv().await {
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

                        return Ok(());
                    }
                },
                _ => todo!(),
            }
        }

        for proc in processes.iter() {
            if !proc.handle.is_finished() {
                eprintln!("Process {} did not exit", proc.info.id);
            }
        }

        Ok(())
    }
}
