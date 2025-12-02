use crossterm::event::{KeyEvent};
// use crossterm::event::{MouseEvent};
use tokio::sync::mpsc;

use crate::ro_cell::RoCell;

static TX: RoCell<mpsc::UnboundedSender<TuiEvent>> = RoCell::new();
static RX: RoCell<mpsc::UnboundedReceiver<TuiEvent>> = RoCell::new();

#[derive(Debug)]
pub enum TuiEvent {
    Key(KeyEvent),
    Render,
    // ProcessMessage { process_id: usize, line: String },
    Quit,
}

impl TuiEvent {
    #[inline]
    pub fn init() {
        let (tx, rx) = mpsc::unbounded_channel();
        TX.init(tx);
        RX.init(rx);
    }

    #[inline]
    pub fn take() -> mpsc::UnboundedReceiver<TuiEvent> {
        RX.drop()
    }

    #[inline]
    pub fn emit(self) {
        TX.send(self).ok();
    }
}
