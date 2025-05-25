use anyhow::Result;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, poll},
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style, palette::tailwind::SLATE},
    text::{Line, Text},
    widgets::{HighlightSpacing, List, ListState, StatefulWidget, Widget},
};
use std::time::Duration;

use pom_server::{Connection, server::Message};

const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

pub async fn run(conn: Connection, services: Vec<String>) -> Result<()> {
    let terminal = ratatui::init();
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
    let app_result = app.run(terminal).await;

    ratatui::restore();
    app_result
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
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
    async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.state == AppState::Running {
            while let Ok(msg) = self.conn.receiver.try_recv() {
                match msg {
                    Message::ProcessOutput { process_id, line } => {
                        let tab = &mut self.process_tab_group.tabs[process_id];
                        tab.data.push(line.clone());
                    }
                    _ => {}
                }
            }
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
            self.handle_events()?;
        }

        self.conn
            .sender
            .send(Message::Command(
                pom_server::server::ServerCommand::Shutdown,
            ))
            .await?;

        Ok(())
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
        if poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => self.next_tab(),
                        KeyCode::Char('k') | KeyCode::Up => self.prev_tab(),
                        KeyCode::Char('l') | KeyCode::Right => todo!(),
                        KeyCode::Char('q') | KeyCode::Esc => self.quit(),
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    pub fn quit(&mut self) {
        self.state = AppState::Quitting;
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
            // .select(self.process_tabs.selected)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.list_state)
    }

    fn render_selected_process_tab(&self, area: Rect, buf: &mut Buffer) {
        let selected = self.list_state.selected().unwrap_or(0);
        let tab = &self.process_tab_group.tabs[selected];
        let lines: Vec<Line> = tab.data.as_slice()[tab.data.len().saturating_sub(100)..]
            .to_vec()
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect();
        let text = Text::from(lines);

        text.render(area, buf);
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Min};
        let vertical = Layout::vertical([Min(0), Length(1)]);
        let [inner_area, footer_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Length(20), Min(0)]);
        let [tabs_area, output_area] = horizontal.areas(inner_area);

        self.render_tabs(tabs_area, buf);
        self.render_selected_process_tab(output_area, buf);
    }
}
