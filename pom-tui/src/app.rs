use anyhow::Result;
use pom_server as server;
use pom_server::Process;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::Widget,
};
use tokio::sync::mpsc::Receiver;
// use strum::EnumIter;

pub fn start(process_state: &Vec<Vec<String>>) -> Result<()> {
    let terminal = ratatui::init();
    let app = App {
        state: AppState::default(),
        process_state,
    };
    let app_result = app.run(terminal);

    ratatui::restore();
    app_result
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum AppState {
    #[default]
    Running,
    Quitting,
}

struct App<'a> {
    state: AppState,
    process_state: &'a Vec<Vec<String>>, // selected_tab: SelectedTab,
}

impl App<'_> {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.state == AppState::Running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn handle_events(&mut self) -> anyhow::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('l') | KeyCode::Right => todo!(),
                    KeyCode::Char('q') | KeyCode::Esc => self.quit(),
                    _ => {}
                }
            }
        }

        Ok(())
    }

    pub fn quit(&mut self) {
        self.state = AppState::Quitting;
    }
}

impl Widget for &App<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Min};
        let vertical = Layout::vertical([Length(1), Min(0), Length(1)]);
        let [header_area, inner_area, footer_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Min(0), Length(20)]);
        let [tabs_area, title_area] = horizontal.areas(header_area);

        render_title(title_area, buf);
        for outputs in self.process_state {
            for line in outputs {
                line.render(inner_area, buf);
            }
        }
    }
}

fn render_title(area: Rect, buf: &mut Buffer) {
    "Ratatui Tabs Example".render(area, buf);
}
