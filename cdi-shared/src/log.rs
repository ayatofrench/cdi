#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug)]
pub struct LogLine {
    pub id: u64,
    pub process_id: u64,
    pub session_id: u64,
    pub timestamp: u128,
    pub stream: Stream,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Crashed,
}

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub id: u64,
    pub name: String,
    pub command: String,
    pub cwd: Option<String>,
    pub pid: Option<usize>,
    pub status: ProcessStatus,
    pub exit_code: Option<i32>,
}

impl ProcessInfo {
    pub fn new(name: String, command: String, cwd: Option<String>) -> Self {
        let id = Self::compute_id(&name, &command, cwd.as_deref());

        Self {
            id,
            name,
            command,
            cwd,
            pid: None,
            status: ProcessStatus::Stopped,
            exit_code: None,
        }
    }

    fn compute_id(name: &str, command: &str, cwd: Option<&str>) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        command.hash(&mut hasher);
        cwd.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Clone, Debug)]
pub struct SessionInfo {
    pub id: u64,
    pub started_at: u64,
}
