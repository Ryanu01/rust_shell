use super::{App, OutputStyle};
use crate::JOBS;
use ratatui::crossterm::execute;
use ratatui::DefaultTerminal;

impl App {
    pub fn new() -> Self {
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".to_string());
        Self {
            input: String::new(),
            cursor: 0,
            output_lines: Vec::new(),
            scroll_offset: 0,
            history: Vec::new(),
            history_idx: None,
            running: true,
            cwd,
            completions: Vec::new(),
            completion_start: 0,
            mouse_capture: true,
            search_mode: false,
            search_query: String::new(),
            search_match: None,
        }
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let mut terminal = ratatui::init();
        terminal.clear()?;
        let _ = execute!(
            std::io::stdout(),
            ratatui::crossterm::event::EnableMouseCapture
        );
        let res = self.run_tui(&mut terminal);
        let _ = execute!(
            std::io::stdout(),
            ratatui::crossterm::event::DisableMouseCapture
        );
        ratatui::restore();
        res
    }

    fn run_tui(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while self.running {
            self.reap_jobs_tui();
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn reap_jobs_tui(&mut self) {
        let mut jobs = JOBS.lock().unwrap();
        let mut it = 0;
        while it < jobs.len() {
            let status = match jobs[it].child.try_wait() {
                Ok(Some(_)) => "Done",
                Ok(None) => "Running",
                Err(_) => "Error",
            };
            if status == "Done" {
                let cmd = jobs[it].command.clone();
                let id = jobs[it].id;
                self.output_lines
                    .push((format!("[{}] Done  {}", id, cmd), OutputStyle::Plain));
                jobs.remove(it);
            } else {
                it += 1;
            }
        }
    }

    pub(crate) fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub(crate) fn update_cwd(&mut self) {
        self.cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".to_string());
    }
}
