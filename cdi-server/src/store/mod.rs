mod block;
mod memory;

use cdi_shared::log::{LogLine, ProcessInfo, ProcessStatus, Stream};

pub trait LogStore {
    fn append(&mut self, process_id: u64, stream: Stream, content: String);
    fn get_lines(&self, process_id: u64, limit: Option<usize>) -> Vec<&LogLine>;
    fn get_lines_since(&self, process_id: u64, since_id: u64) -> Vec<&LogLine>;
    fn get_process(&self, process_id: u64) -> &ProcessInfo;
    fn get_processes(&self) -> Vec<&ProcessInfo>;
    // fn set_process_status(
    //     &mut self,
    //     process_id: u64,
    //     status: ProcessStatus,
    //     exit_code: Option<i32>,
    // );
}
