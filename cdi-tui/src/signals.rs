use anyhow::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent, KeyEventKind};
use futures::StreamExt;
use tokio::{
    select,
    sync::{mpsc, oneshot},
};

use cdi_shared::event::Event;

pub(super) struct Signals {
    tx: mpsc::UnboundedSender<(bool, Option<oneshot::Sender<()>>)>,
}

impl Signals {
    pub(super) fn start() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        Self::spawn(rx)?;

        Ok(Self { tx })
    }

    #[inline]
    fn handle_event(event: CrosstermEvent) {
        match event {
            CrosstermEvent::Key(
                key @ KeyEvent {
                    kind: KeyEventKind::Press,
                    ..
                },
            ) => Event::Key(key).emit(),
            _ => {}
        }
    }

    fn spawn(mut rx: mpsc::UnboundedReceiver<(bool, Option<oneshot::Sender<()>>)>) -> Result<()> {
        let mut evt_stream = Some(EventStream::new());

        tokio::spawn(async move {
            loop {
                if let Some(evt) = &mut evt_stream {
                    select! {
                        biased;
                        // Some((state, mut callback)) = rx.recv() => {
                        //     evt_stream = evt_stream.filter(|_| state);
                        //
                        //     callback.take().map(|cb| cb.send(()));
                        // },
                        Some(Ok(e)) = evt.next() => Self::handle_event(e),
                    }
                }
            }
        });

        Ok(())
    }
}
