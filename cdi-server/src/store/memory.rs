use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

use super::LogStore;
use cdi_shared::log::{LogLine, ProcessInfo, Stream};

// - [ ] Implement remaining trait methods

struct ProcessData {
    info: ProcessInfo,
    logs: VecDeque<LogLine>,
}

pub struct MemoryStore {
    session_id: u64,
    max_lines: usize,
    processes: HashMap<u64, ProcessData>,
}

impl MemoryStore {
    pub fn new(processes: Vec<ProcessInfo>, session_id: u64, max_lines: usize) -> Self {
        let processes = processes
            .iter()
            .map(|info| {
                let data = ProcessData {
                    info: info.clone(),
                    logs: VecDeque::with_capacity(max_lines),
                };

                (info.id, data)
            })
            .collect();

        MemoryStore {
            session_id,
            max_lines,
            processes,
        }
    }
}

impl LogStore for MemoryStore {
    fn append(&mut self, process_id: u64, stream: Stream, content: String) {
        let proc = self.processes.get_mut(&process_id).unwrap();
        let next_id = proc.logs.back().map(|log| log.id + 1).unwrap_or(0);

        proc.logs.push_back(LogLine {
            id: next_id,
            process_id,
            session_id: self.session_id,
            stream,
            content,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        });

        if proc.logs.len() > self.max_lines {
            proc.logs.pop_front();
        }
    }

    fn get_lines(&self, process_id: u64, limit: Option<usize>) -> Vec<&LogLine> {
        let proc = self.processes.get(&process_id).unwrap();
        let start = limit
            .map(|n| proc.logs.len().saturating_sub(n))
            .unwrap_or(0);

        proc.logs.range(start..).collect()
    }

    fn get_lines_since(&self, process_id: u64, since_id: u64) -> Vec<&LogLine> {
        let proc = self.processes.get(&process_id).unwrap();
        let start = proc
            .logs
            .binary_search_by(|log| log.id.cmp(&since_id))
            .unwrap_or(0);

        proc.logs.range(start..).collect()
    }

    fn get_process(&self, process_id: u64) -> &ProcessInfo {
        &self.processes.get(&process_id).unwrap().info
    }

    fn get_processes(&self) -> Vec<&ProcessInfo> {
        self.processes.values().map(|value| &value.info).collect()
    }

    // fn set_process_status(
    //     &mut self,
    //     process_id: u64,
    //     status: cdi_shared::log::ProcessStatus,
    //     exit_code: Option<i32>,
    // ) {
    // }
}
