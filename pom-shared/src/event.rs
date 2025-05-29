use crossterm::event::{KeyEvent, MouseEvent};
use tokio::sync::mpsc;

use crate::ro_cell::RoCell;

static TX: RoCell<mpsc::UnboundedSender<Event>> = RoCell::new();
static RX: RoCell<mpsc::UnboundedReceiver<Event>> = RoCell::new();

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Render,
    ProcessMessage { process_id: usize, line: String },
    Quit,
}

impl Event {
    #[inline]
    pub fn init() {
        let (tx, rx) = mpsc::unbounded_channel();
        TX.init(tx);
        RX.init(rx);
    }

    #[inline]
    pub fn take() -> mpsc::UnboundedReceiver<Event> {
        RX.drop()
    }

    #[inline]
    pub fn emit(self) {
        TX.send(self).ok();
    }
}
