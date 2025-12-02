use tokio::sync::mpsc;

use crate::log::{ProcessStatus, Stream};
use crate::ro_cell::RoCell;

static STORE_TX: RoCell<mpsc::UnboundedSender<StoreEvent>> = RoCell::new();
static STORE_RX: RoCell<mpsc::UnboundedReceiver<StoreEvent>> = RoCell::new();

#[derive(Clone, Debug)]
pub enum StoreEvent {
    AppendLog {
        process_id: u64,
        stream: Stream,
        content: String,
    },
    ProcessExited {
        process_id: u64,
        status: ProcessStatus,
        exit_code: Option<i32>,
    },
}

impl StoreEvent {
    #[inline]
    pub fn init() {
        let (tx, rx) = mpsc::unbounded_channel();
        STORE_TX.init(tx);
        STORE_RX.init(rx);
    }

    #[inline]
    pub fn take() -> mpsc::UnboundedReceiver<StoreEvent> {
        STORE_RX.drop()
    }

    #[inline]
    pub fn emit(self) {
        STORE_TX.send(self).ok();
    }
}
