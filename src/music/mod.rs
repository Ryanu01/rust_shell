mod app;
mod input;
mod player;
mod ui;

pub(crate) use app::MusicApp;

pub fn run() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let res = MusicApp::new().run_tui(&mut terminal);
    ratatui::restore();
    res
}
