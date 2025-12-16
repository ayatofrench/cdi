# Storage & Query Architecture

## Overview

This document describes the architecture for centralized log storage with efficient memory sharing between the storage engine and clients (TUI, HTTP API).

The block-based storage design is inspired by [Tokio's mpsc channel implementation](https://github.com/tokio-rs/tokio/blob/master/tokio/src/sync/mpsc/block.rs), which batches messages into fixed-capacity blocks for memory efficiency and cache locality.

The design follows a database-like separation of concerns:
- **Storage Layer**: Owns data, provides snapshots
- **Query Layer**: Filters/slices snapshots (stateless utilities)
- **Clients**: Orchestrate snapshot acquisition and querying

```
┌─────────────────────────────────────────────────────────────────┐
│                        StoreManager                              │
│                                                                  │
│   MemoryStore                                                    │
│   ├── Process A: ProcessLogs [Block] [Block] [Block]            │
│   ├── Process B: ProcessLogs [Block]                            │
│   └── Process C: ProcessLogs [Block] [Block]                    │
│                                                                  │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           │ StoreHandle (Arc<RwLock<MemoryStore>>)
                           │
              ┌────────────┴────────────┐
              │                         │
              ▼                         ▼
┌─────────────────────┐     ┌─────────────────────┐
│        TUI          │     │    HTTP Server      │
│                     │     │                     │
│ 1. Get snapshot     │     │ 1. Get snapshot     │
│ 2. Query/filter     │     │ 2. Query/filter     │
│ 3. Render           │     │ 3. Serialize JSON   │
│ 4. Drop snapshot    │     │ 4. Drop snapshot    │
└─────────────────────┘     └─────────────────────┘
```

## Design Goals

1. **Memory Efficiency**: Share log data between storage and clients without duplication
2. **Zero-Copy Reads**: Clients work with references into shared data
3. **Non-Blocking Renders**: TUI never blocks on storage operations
4. **Clean Separation**: Storage doesn't know about queries; queries don't know about storage internals
5. **Simple Mental Model**: Snapshot is the interface between storage and querying

## Core Concepts

### Block Storage

Instead of one allocation per log line, we batch lines into blocks (similar to Tokio's mpsc channel):

```
Traditional (many allocations):
┌───┐ ┌───────┐ ┌─┐ ┌────────────┐
│log│ │log    │ │l│ │log         │
└───┘ └───────┘ └─┘ └────────────┘
  ↑       ↑      ↑        ↑
  4 separate malloc calls

Block-based (few allocations):
┌─────────────────────────────────────────────────────┐
│ Arc<Block>                                          │
│ string_data: "log1|log message 2|another log|..."  │
│ lines: [meta, meta, meta, ...]                     │
└─────────────────────────────────────────────────────┘
  ↑
  1 malloc, ~256 lines, contiguous memory
```

Benefits:
- Fewer allocations (1 per ~256 lines instead of 1 per line)
- Better cache locality (contiguous string data)
- Natural eviction unit (drop whole blocks)
- Cheap sharing via `Arc<Block>`

### Copy-on-Write with Arc::make_mut

Blocks are wrapped in `Arc` for sharing. When the store needs to append to a block that's currently shared with a client:

```rust
// In storage append:
let current = Arc::make_mut(self.blocks.back_mut().unwrap());
current.push(line);
```

`Arc::make_mut` behavior:
- If refcount == 1 (no clients holding it): mutates in place, zero cost
- If refcount > 1 (client has snapshot): clones the block, store gets new copy

This means:
- Clients always see consistent snapshots
- No explicit "shared/not shared" state tracking
- Storage can always append without blocking

### Snapshot Lifecycle

Both TUI and HTTP have the same pattern - they need consistent data for a brief window:

```
TUI:
┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
│ Receive  │ → │ Acquire  │ → │  Render  │ → │  Drop    │
│ Render   │   │ Snapshot │   │  (draw)  │   │ Snapshot │
│ Event    │   │          │   │          │   │          │
└──────────┘   └──────────┘   └──────────┘   └──────────┘
     ~0ms          ~0ms         ~1-5ms           ~0ms

HTTP:
┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
│ Receive  │ → │ Acquire  │ → │  Build   │ → │  Drop    │
│ Request  │   │ Snapshot │   │  JSON    │   │ Snapshot │
└──────────┘   └──────────┘   └──────────┘   └──────────┘
     ~0ms          ~0ms         ~1-10ms          ~0ms
```

Snapshot held for milliseconds, then released. Store continues appending independently.

## Data Structures

### Block

Container for batched log lines with string interning within the block:

```rust
const BLOCK_CAP: usize = 128;

pub struct Block {
    // Block header - shared by all lines (saves ~4KB per block)
    pub process_id: u64,
    pub session_id: u64,
    
    // Line storage
    string_data: String,
    lines: Vec<LogLineMeta>,
}

/// Per-line metadata (~40 bytes vs ~56 bytes if we stored process_id/session_id)
pub struct LogLineMeta {
    pub id: u64,
    pub timestamp: u128,
    pub stream: Stream,
    content_start: u32,
    content_len: u32,
}
```

Block seals when line count reaches 128. This provides predictable block sizes
for simpler iteration and reasoning about eviction.

### ProcessLogs

Manages blocks for a single process:

```rust
pub struct ProcessLogs {
    blocks: VecDeque<Arc<Block>>,
    max_blocks: usize,
}
```

Responsibilities:
- Append new lines (using `Arc::make_mut` for copy-on-write)
- Seal full blocks
- Evict oldest blocks when over limit
- Provide snapshot (cheap `Arc::clone` of block references)

### MemoryStore

Top-level store managing all processes:

```rust
pub struct MemoryStore {
    processes: HashMap<u64, ProcessData>,
    session_id: u64,
}

struct ProcessData {
    info: ProcessInfo,
    logs: ProcessLogs,
}
```

### StoreHandle

Shared access wrapper for clients:

```rust
#[derive(Clone)]
pub struct StoreHandle {
    store: Arc<RwLock<MemoryStore>>,
}

impl StoreHandle {
    pub fn snapshot(&self, process_id: u64) -> LogSnapshot { ... }
    pub fn snapshot_all(&self) -> HashMap<u64, LogSnapshot> { ... }
}
```

### LogSnapshot

Immutable view of logs at a point in time:

```rust
pub struct LogSnapshot {
    blocks: Vec<Arc<Block>>,
    process_id: u64,
}

impl LogSnapshot {
    pub fn query(&self) -> LogView<'_> { ... }
}
```

## Query Layer

Stateless utilities that operate on snapshots. The snapshot IS the interface - no middleware needed.

### LogQuery

Builder for filter parameters:

```rust
pub struct LogQuery {
    pub process_id: Option<u64>,
    pub stream: Option<Stream>,
    pub after_id: Option<u64>,
    pub limit: Option<usize>,
}

impl LogQuery {
    pub fn new() -> Self { ... }
    pub fn process(self, id: u64) -> Self { ... }
    pub fn stream(self, stream: Stream) -> Self { ... }
    pub fn after(self, id: u64) -> Self { ... }
    pub fn limit(self, n: usize) -> Self { ... }
}
```

### LogView

Filtered view over a snapshot, returns references:

```rust
pub struct LogView<'a> {
    blocks: &'a [Arc<Block>],
    query: LogQuery,
}

impl<'a> LogView<'a> {
    /// Iterate over matching lines
    /// Note: Access block.process_id and block.session_id directly if needed
    pub fn iter(&self) -> impl Iterator<Item = (&LogLineMeta, &str)> { ... }
    
    /// Get last N matching lines (for TUI viewport)
    pub fn tail(&self, n: usize) -> Vec<(&LogLineMeta, &str)> { ... }
}
```

### Reference Chain

Zero-copy from storage to render:

```
LogSnapshot (owns Vec<Arc<Block>>)
     │
     │ .query()
     ▼
LogView<'a> (borrows &'a [Arc<Block>])
     │
     │ .iter() / .tail()
     ▼
Iterator<Item = (&'a LogLineMeta, &'a str)>
     │
     ▼
  Render / Serialize
```

## Event Flow

### Storage Side (StoreManager)

```rust
impl StoreManager {
    async fn run(mut self, mut events: Receiver<StoreEvent>) {
        while let Some(event) = events.recv().await {
            match event {
                StoreEvent::AppendLog { process_id, stream, content } => {
                    self.store.write().unwrap().append(process_id, stream, content);
                    TuiEvent::Render.emit();
                }
                StoreEvent::ProcessExited { process_id, status, exit_code } => {
                    self.store.write().unwrap().set_status(process_id, status, exit_code);
                    TuiEvent::Render.emit();
                }
            }
        }
    }
}
```

### Client Side (TUI)

```rust
impl App {
    async fn on_render(&mut self, terminal: &mut Terminal<...>) -> Result<()> {
        // 1. Get snapshot (cheap Arc clones)
        let snapshot = self.store_handle.snapshot(self.selected_process_id);
        
        // 2. Query (zero-copy references)
        let lines = snapshot.query().tail(self.viewport_height);
        
        // 3. Render using references
        terminal.draw(|frame| {
            for (meta, content) in &lines {
                // content is &str pointing into Arc<Block>
            }
        })?;
        
        // 4. snapshot dropped here
        Ok(())
    }
}
```

### Client Side (HTTP)

```rust
async fn get_logs(
    State(store_handle): State<StoreHandle>,
    Query(params): Query<LogParams>,
) -> Json<Vec<LogLineDto>> {
    // 1. Get snapshot
    let snapshot = store_handle.snapshot(params.process_id);
    
    // 2. Query and serialize (clone strings for JSON)
    let lines: Vec<LogLineDto> = snapshot
        .query()
        .after(params.after_id.unwrap_or(0))
        .limit(params.limit.unwrap_or(100))
        .iter()
        .map(|(meta, content)| LogLineDto {
            id: meta.id,
            content: content.to_string(),
            // ...
        })
        .collect();
    
    // 3. snapshot dropped
    Json(lines)
}
```

## File Structure

```
cdi-server/src/store/
├── mod.rs          # Public exports
├── block.rs        # Block, LogLineMeta
├── memory.rs       # ProcessLogs, MemoryStore
├── handle.rs       # StoreHandle
├── snapshot.rs     # LogSnapshot
└── query.rs        # LogQuery, LogView

cdi-shared/src/
├── log.rs          # Stream, ProcessStatus, ProcessInfo (already exists)
└── event/
    ├── store.rs    # StoreEvent (already exists)
    └── ui.rs       # TuiEvent (already exists)
```

## Future Considerations

### String Interning

If logs become repetitive (common with structured logging), we can add interning:

```rust
struct StringInterner {
    strings: HashMap<u64, Arc<str>>,
}
```

This would deduplicate identical log messages across the entire store.

### Regex Filtering

The query layer can be extended:

```rust
pub struct LogQuery {
    // ... existing fields
    pub search: Option<String>,
    pub regex: Option<Regex>,
}
```

### Persistent Storage

The `LogStore` trait allows swapping `MemoryStore` for a persistent implementation later if needed.

---

# Communication Architecture

## Overview

This document describes the communication patterns between components. The system uses a hybrid approach:

- **Actor Model**: For supervisor ↔ process communication (commands, state queries, restarts)
- **Event Buses**: For decoupled internal communication (store events, TUI events)
- **Client Protocol**: For external clients (CLI) to query and command the server

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    MAIN PROCESS (dev session)                   │
│                                                                 │
│   ┌─────────┐     ┌─────────────┐     ┌──────────────────┐     │
│   │   TUI   │←───→│   SERVER    │────→│  STORAGE ENGINE  │     │
│   │ (built  │     │             │     │                  │     │
│   │   in)   │     │             │     │                  │     │
│   └─────────┘     └──────┬──────┘     └──────────────────┘     │
│                          │                                      │
│                          │ Unix Socket                          │
└──────────────────────────│──────────────────────────────────────┘
                           │
                      ┌────┴────┐
                      │   CLI   │  (LLM agents use CLI)
                      └─────────┘
```

### Design Principles

1. **TUI is built-in**: Runs in the same process as the server for simplicity and direct storage access
2. **CLI is external**: Connects via Unix socket for one-shot queries (logs, status)
3. **LLM agents use CLI**: No special integration needed - agents invoke CLI commands
4. **Decoupled event flow**: Server emits events, TUI relays them to its internal bus

## Actor Model: Supervisor ↔ Processes

The supervisor and individual processes communicate via dedicated channels (actor model):

```
┌────────────────────────────────────────────────────────────────┐
│                        SUPERVISOR                               │
│                                                                 │
│   ProcessContext {                                              │
│       info: ProcessInfo,                                        │
│       conn: Connection { sender, receiver },                    │
│       handle: JoinHandle<()>,                                   │
│   }                                                             │
│                                                                 │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│   │ Process 1   │  │ Process 2   │  │ Process N   │            │
│   │   Context   │  │   Context   │  │   Context   │            │
│   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘            │
└──────────│────────────────│────────────────│────────────────────┘
           │                │                │
           ▼                ▼                ▼
     ┌──────────┐    ┌──────────┐    ┌──────────┐
     │ Process  │    │ Process  │    │ Process  │
     │  Actor   │    │  Actor   │    │  Actor   │
     └──────────┘    └──────────┘    └──────────┘
```

### Actor Messages

```rust
/// Commands sent to process actors
enum ProcessCommand {
    Shutdown,
    Restart,
    // Future: SendInput(String), etc.
}

/// Queries sent to process actors (if needed)
enum ProcessQuery {
    GetStatus,
}

/// Responses from process actors
enum ProcessResponse {
    Status(ProcessStatus),
}
```

### Why Actor Model Here?

- **Clear ownership**: Each process actor owns its child process handle
- **Graceful shutdown**: Supervisor can send shutdown command, wait for acknowledgment
- **Restart logic**: Supervisor can coordinate restart without race conditions
- **Backpressure**: Bounded channels prevent message buildup

## Event Buses: Internal Communication

Two separate event buses for decoupled communication:

### StoreEvent Bus

**Purpose**: Process actors → Storage Engine

```rust
enum StoreEvent {
    AppendLog { process_id: u64, stream: Stream, content: String },
    ProcessExited { process_id: u64, status: ProcessStatus, exit_code: Option<i32> },
}
```

**Flow**:
```
[Process Actor] --emit()--> [StoreEvent Bus] --recv()--> [StoreManager]
                                                              │
                                                              ▼
                                                        [MemoryStore]
```

### TuiEvent Bus

**Purpose**: Internal TUI coordination (keyboard, render triggers, quit)

```rust
enum TuiEvent {
    Key(KeyEvent),
    Render,
    Quit,
    // Server events get translated and queued here
    ServerEvent(ServerEvent),  // Optional: or handle separately
}
```

**Flow**:
```
[Keyboard Handler] ──┐
[Render Timer] ──────┼──emit()──> [TuiEvent Bus] ──recv()──> [TUI Main Loop]
[Server Relay] ──────┘
```

### Server → TUI Relay

The TUI receives server events and relays them to its internal bus:

```rust
// Server relay task (runs inside TUI)
async fn server_relay(mut server_events: Receiver<ServerEvent>) {
    while let Some(event) = server_events.recv().await {
        // Translate server event to TUI event and queue it
        match event {
            ServerEvent::ProcessStateChanged { .. } => {
                TuiEvent::Render.emit();
            }
            // Most events just trigger a re-render since TUI
            // pulls fresh data from storage on each render
        }
    }
}
```

**Note**: Since TUI reads directly from `StoreHandle` on each render, most server events just trigger a `Render` event rather than carrying data.

## Client Protocol: CLI ↔ Server

External clients (CLI) connect via Unix socket for request/response communication.

### Protocol

```rust
/// Requests from CLI to server
enum ClientRequest {
    // Queries
    ListProcesses,
    GetProcessStatus { process_id: String },
    GetLogs { 
        process_id: String, 
        limit: usize,
        stream: Option<Stream>,  // stdout, stderr, or both
    },
    
    // Commands
    RestartProcess { process_id: String },
    StopProcess { process_id: String },
}

/// Responses from server to CLI
enum ClientResponse {
    Processes(Vec<ProcessInfo>),
    Status(ProcessStatus),
    Logs(Vec<LogLine>),
    Ok,
    Error { message: String },
}
```

### Connection Flow

```
CLI                                    Server
 │                                        │
 │──── connect (unix socket) ────────────>│
 │                                        │
 │──── ClientRequest (JSON) ─────────────>│
 │                                        │
 │<─── ClientResponse (JSON) ─────────────│
 │                                        │
 │──── disconnect ───────────────────────>│
 │                                        │
```

### Example CLI Usage

```bash
# List all processes
$ pom ps
PID  NAME        STATUS   
1    api         running  
2    frontend    running  
3    worker      crashed  

# Get logs for a specific process
$ pom logs api --lines 50

# Get only stderr
$ pom logs api --stderr --lines 20

# Restart a process
$ pom restart worker
```

### Server-Side Handler

```rust
async fn handle_client(stream: UnixStream, store: StoreHandle, supervisor: SupervisorHandle) {
    let request: ClientRequest = read_json(&stream).await?;
    
    let response = match request {
        ClientRequest::ListProcesses => {
            let processes = store.list_processes();
            ClientResponse::Processes(processes)
        }
        ClientRequest::GetLogs { process_id, limit, stream } => {
            let snapshot = store.snapshot(&process_id);
            let logs = snapshot.query()
                .stream(stream)
                .limit(limit)
                .collect();
            ClientResponse::Logs(logs)
        }
        ClientRequest::RestartProcess { process_id } => {
            supervisor.restart(&process_id).await?;
            ClientResponse::Ok
        }
        // ...
    };
    
    write_json(&stream, &response).await?;
}
```

## Summary: Which Pattern Where?

| Communication Path | Pattern | Reason |
|--------------------|---------|--------|
| Supervisor ↔ Process | Actor (channels) | Ownership, graceful shutdown, restart coordination |
| Process → Storage | Event bus | Fire-and-forget, decoupled |
| Storage → TUI | Direct read | Same process, simple |
| Server → TUI (notifications) | Event relay | Trigger re-renders |
| TUI internal | Event bus | Single event loop |
| CLI → Server | Request/Response (socket) | One-shot queries, external process |
