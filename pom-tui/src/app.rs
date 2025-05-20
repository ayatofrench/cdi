use anyhow::Result;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, poll},
    layout::{Constraint, Layout, Rect},
    text::{Line, Text},
    widgets::Widget,
};
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

use pom_server::ProcessMessage;

pub async fn run(conn: Receiver<ProcessMessage>) -> Result<()> {
    let terminal = ratatui::init();
    let mut app = App {
        state: AppState::default(),
        conn,
        process_tabs: ProcessTabs::default(),
    };
    for _ in 0..2 {
        app.process_tabs.data.push(vec![]);
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

struct ProcessTabs {
    selected: usize,
    data: Vec<Vec<String>>,
}

impl Default for ProcessTabs {
    fn default() -> Self {
        Self {
            selected: 0,
            data: Vec::new(),
        }
    }
}

struct App {
    state: AppState,
    conn: Receiver<ProcessMessage>,
    process_tabs: ProcessTabs,
}

impl App {
    async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.state == AppState::Running {
            while let Ok(output) = self.conn.try_recv() {
                let process_id: usize = output.process_id.try_into()?;
                self.process_tabs.data[process_id].push(output.line.clone());
            }
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            self.handle_events()?;
        }

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
        if self.process_tabs.selected < self.process_tabs.data.len() - 1 {
            self.process_tabs.selected = self.process_tabs.selected + 1;
        }
    }

    pub fn prev_tab(&mut self) {
        if self.process_tabs.selected > 0 {
            self.process_tabs.selected = self.process_tabs.selected - 1;
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Min};
        let vertical = Layout::vertical([Length(1), Min(0), Length(1)]);
        let [header_area, inner_area, footer_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Min(0), Length(20)]);
        let [tabs_area, title_area] = horizontal.areas(header_area);

        render_title(title_area, buf);
        render_selected_process_tab(self, inner_area, buf);
    }
}

fn render_selected_process_tab(app: &App, area: Rect, buf: &mut Buffer) {
    let lines: Vec<Line> = app.process_tabs.data[app.process_tabs.selected]
        .iter()
        .map(|line| Line::from(line.clone()))
        .collect();
    let text = Text::from(lines);

    text.render(area, buf);
}

fn render_title(area: Rect, buf: &mut Buffer) {
    "Ratatui Tabs Example".render(area, buf);
}
