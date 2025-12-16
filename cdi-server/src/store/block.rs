// #### File: `block.rs` (NEW)
// - [ ] `LogLineMeta` struct (per-line, ~40 bytes)
//     - `id: u64`
//     - `timestamp: u128`
//     - `stream: Stream`
//     - `content_start: u32` (offset into string_data)
//     - `content_len: u32`
// - [ ] `Block` struct
//     - `process_id: u64` (block header - shared by all lines)
//     - `session_id: u64` (block header - shared by all lines)
//     - `string_data: String` (concatenated log content)
//     - `lines: Vec<LogLineMeta>`
// - [ ] `Block::new()` with pre-allocated capacity
//     - `BLOCK_LINE_CAP = 256`
//     - `BLOCK_STRING_CAP = 64KB`
// - [ ] `Block::push(meta, content)` - append line
// - [ ] `Block::is_full()` - check if block should be sealed
// - [ ] `Block::get_content(&LogLineMeta) -> &str`
// - [ ] `Block::iter() -> impl Iterator<Item = (&LogLineMeta, &str)>`
// - [ ] Derive `Clone` for copy-on-write support

use cdi_shared::log::Stream;

const BLOCK_LINE_CAP: usize = 128;
const BLOCK_STRING_INIT_SIZE: usize = 32 * 1024; // 64KB

#[derive(Clone)]
struct LogLineData {
    id: u64,
    timestamp: u128,
    stream: Stream,
    content_start: u32,
    content_len: u32,
}

#[derive(Clone)]
pub struct Block {
    process_id: u64,
    session_id: u64,
    data: String,
    lines: Vec<LogLineData>,
}

impl Block {
    pub fn new(process_id: u64, session_id: u64) -> Block {
        Self {
            process_id,
            session_id,
            data: String::with_capacity(BLOCK_STRING_INIT_SIZE), // 64KB
            lines: Vec::with_capacity(BLOCK_LINE_CAP),
        }
    }

    pub fn push() {}
    pub fn is_full() {}
    pub fn get_content() {}
    // pub fn iter() -> impl Iterator<Item = (&LogLineData, &str)> {}
}

// struct LogLineContext<'a> {}
