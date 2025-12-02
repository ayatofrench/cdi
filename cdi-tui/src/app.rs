use ::crossterm::event::{KeyCode as CTKeyCode, KeyEvent, KeyEventKind as KEK};
use ansi_to_tui::IntoText;
use anyhow::Result;
use ratatui::{
    Terminal,
    buffer::Buffer,
    crossterm::{
        self,
        event::{self, Event, KeyCode, KeyEventKind, poll},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Layout, Rect},
    prelude::CrosstermBackend,
    style::{ self, Modifier, Style, palette::tailwind::{SLATE, YELLOW}, },
    text::{Line, Text},
    widgets::{
        Block, Borders, HighlightSpacing, List, ListState, Paragraph, StatefulWidget, Widget, Wrap,
    },
};
use std::io;
use std::time::Duration;
use tokio::time;

use crate::signals::{self, Signals};
use cdi_server::{Connection, server::Message};
use cdi_shared::event::ui::TuiEvent;

const SELECTED_STYLE: Style = Style::new().fg(YELLOW.c600).add_modifier(Modifier::BOLD);

pub async fn run(conn: Connection, services: Vec<String>) -> Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen,)?;

    let term_backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(term_backend)?;

    let mut app = App {
        state: AppState::default(),
        conn,
        process_tab_group: ProcessTabGroup::default(),
        list_state: ListState::default().with_selected(Some(0)),
    };
    for service in services {
        app.process_tab_group.tabs.push(ProcessTab {
            name: service,
            data: vec![],
        })
    }

    terminal.clear()?;

    app.start_loop(&mut terminal).await?;

    terminal.clear()?;
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;

    println!("Cleaning up resources");

    app.conn
        .sender
        .send(Message::Command(
            cdi_server::server::ServerCommand::Shutdown,
        ))
        .await?;

    time::sleep(time::Duration::from_secs(1)).await;

    Ok(())
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum AppState {
    #[default]
    Running,
    Quitting,
}

struct ProcessTab {
    name: String,
    data: Vec<String>,
}

impl Default for ProcessTab {
    fn default() -> Self {
        Self {
            data: vec![],
            name: String::default(),
        }
    }
}

struct ProcessTabGroup {
    // selected: usize,
    tabs: Vec<ProcessTab>,
}

impl Default for ProcessTabGroup {
    fn default() -> Self {
        Self {
            // selected: 0,
            tabs: Vec::new(),
        }
    }
}

struct App {
    state: AppState,
    conn: Connection,
    process_tab_group: ProcessTabGroup,
    list_state: ListState,
}

impl App {
    async fn start_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::StdoutLock<'_>>>,
    ) -> Result<()> {
        let (mut evt_rx, _signals) = (TuiEvent::take(), Signals::start()?);
        self.render(terminal)?;

        let mut events_proccessed = 0;
        let mut events = Vec::with_capacity(200);
        while evt_rx.recv_many(&mut events, 50).await > 0 {
            for event in events.drain(..) {
                events_proccessed += 1;
                self.disptach(event)?;
            }

            if self.state == AppState::Quitting {
                break;
            }

            if events_proccessed >= 50 {
                events_proccessed = 0;
                self.render(terminal)?;
            } else if let Ok(event) = evt_rx.try_recv() {
                events.push(event);
                // self.render(terminal)?;
            } else {
                events_proccessed = 0;
                self.render(terminal)?;
            }
        }

        Ok(())
    }

    #[inline]
    fn disptach(&mut self, event: TuiEvent) -> Result<()> {
        match event {
            TuiEvent::Key(key) => self.dispatch_key(key),
            // TuiEvent::ProcessMessage { process_id, line } => {
            //     let tab = &mut self.process_tab_group.tabs[process_id];
            //     tab.data.push(line.clone());
            // }
            _ => {} // TuiEvent::Render => self.render(sel)
        }

        Ok(())
    }

    #[inline]
    fn dispatch_key(&mut self, key: KeyEvent) {
        if key.kind == KEK::Press {
            match key.code {
                CTKeyCode::Char('j') | CTKeyCode::Down => self.next_tab(),
                CTKeyCode::Char('k') | CTKeyCode::Up => self.prev_tab(),
                CTKeyCode::Char('l') | CTKeyCode::Right => todo!(),
                CTKeyCode::Char('q') | CTKeyCode::Esc => self.quit(),
                _ => {}
            }
        }
    }

    fn render(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::StdoutLock<'_>>>,
    ) -> Result<()> {
        terminal.draw(|frame| {
            use Constraint::{Length, Min};
            let vertical = Layout::vertical([Min(0), Length(1)]);
            let [inner_area, _footer_area] = vertical.areas(frame.area());

            let horizontal = Layout::horizontal([Length(20), Min(0)]);
            let [tabs_area, output_area] = horizontal.areas(inner_area);

            self.render_tabs(tabs_area, frame.buffer_mut());
            self.render_selected_process_tab(output_area, frame.buffer_mut());
        })?;

        Ok(())
    }

    pub fn quit(&mut self) {
        self.state = AppState::Quitting;
        TuiEvent::Quit.emit();
    }

    pub fn next_tab(&mut self) {
        self.list_state.select_next();
    }

    pub fn prev_tab(&mut self) {
        self.list_state.select_previous();
    }

    fn render_tabs(&mut self, area: Rect, buf: &mut Buffer) {
        let titles = self
            .process_tab_group
            .tabs
            .iter()
            .map(|tab| tab.name.clone());

        let list = List::new(titles)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always)
            .block(Block::new().borders(Borders::RIGHT));

        StatefulWidget::render(list, area, buf, &mut self.list_state)
    }

    fn render_selected_process_tab(&self, area: Rect, buf: &mut Buffer) {
        let selected = self.list_state.selected().unwrap_or(0);
        let tab = &self.process_tab_group.tabs[selected];
        let raw_text: Vec<Text> = tab.data.as_slice()
            [tab.data.len().saturating_sub(area.rows().count())..]
            .to_vec()
            .iter()
            .map(|line| line.into_text().unwrap())
            .collect();

        let mut line_idx = 0;

        for text in raw_text.as_slice() {
            for line in text.iter() {
                buf.set_line(area.x, line_idx, line, area.width);
                line_idx += 1;
            }
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Min};
        let vertical = Layout::vertical([Min(0), Length(1)]);
        let [inner_area, _footer_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Length(20), Min(0)]);
        let [tabs_area, output_area] = horizontal.areas(inner_area);

        self.render_tabs(tabs_area, buf);
        self.render_selected_process_tab(output_area, buf);
    }
}
