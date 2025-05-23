use anyhow::Result;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, poll},
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style, palette::tailwind::SLATE},
    text::{Line, Text},
    widgets::{HighlightSpacing, List, ListState, StatefulWidget, Tabs, Widget},
};
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

use pom_server::ProcessMessage;

const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

pub async fn run(conn: Receiver<ProcessMessage>) -> Result<()> {
    let terminal = ratatui::init();
    let mut app = App {
        state: AppState::default(),
        conn,
        process_tabs: ProcessTabs::default(),
        list_state: ListState::default().with_selected(Some(0)),
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
    list_state: ListState,
}

impl App {
    async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.state == AppState::Running {
            while let Ok(output) = self.conn.try_recv() {
                let process_id: usize = output.process_id.try_into()?;
                self.process_tabs.data[process_id].push(output.line.clone());
            }
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
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
        self.list_state.select_next();
        // if self.process_tabs.selected < self.process_tabs.data.len() - 1 {
        //     self.process_tabs.selected = self.process_tabs.selected + 1;
        // }
    }

    pub fn prev_tab(&mut self) {
        self.list_state.select_previous();
        // if self.process_tabs.selected > 0 {
        //     self.process_tabs.selected = self.process_tabs.selected - 1;
        // }
    }

    fn render_tabs(&mut self, area: Rect, buf: &mut Buffer) {
        let titles = self.process_tabs.data.iter().map(|_| "1");

        let list = List::new(titles)
            // .select(self.process_tabs.selected)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.list_state)
    }

    fn render_selected_process_tab(&self, area: Rect, buf: &mut Buffer) {
        let selected = self.list_state.selected().unwrap_or(0);
        let lines: Vec<Line> = self.process_tabs.data[selected].as_slice()
            [self.process_tabs.data.len().saturating_sub(100)..]
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
