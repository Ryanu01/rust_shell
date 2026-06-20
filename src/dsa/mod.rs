mod app;
mod input;
pub(crate) mod sorting;
pub(crate) mod topics;
mod ui;

pub(crate) use topics::TOPICS;

#[derive(Clone)]
pub(crate) struct Step {
    pub(crate) array: Vec<u32>,
    pub(crate) compare: Option<(usize, usize)>,
    pub(crate) swap: Option<(usize, usize)>,
    pub(crate) sorted: Vec<bool>,
    pub(crate) label: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Menu,
    Visualizer,
    Compare,
    InputSize,
    InputValues,
}

pub(crate) struct DsaApp {
    screen: Screen,
    selected: usize,
    selected2: usize,
    topic_count: usize,
    array: Vec<u32>,
    steps: Vec<Step>,
    steps2: Vec<Step>,
    current_step: usize,
    current_step2: usize,
    custom_input: String,
    custom_size: String,
    running: bool,
    compare_picking: bool,
    compare_selected_first: bool,
}

pub fn run() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let res = DsaApp::new().run_tui(&mut terminal);
    ratatui::restore();
    res
}
