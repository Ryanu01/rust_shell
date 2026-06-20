use super::{DsaApp, Screen};
use crate::dsa::topics::TOPICS;

impl DsaApp {
    pub fn new() -> Self {
        Self {
            screen: Screen::Menu,
            selected: 0,
            selected2: 0,
            topic_count: TOPICS.len(),
            array: Vec::new(),
            steps: Vec::new(),
            steps2: Vec::new(),
            current_step: 0,
            current_step2: 0,
            custom_input: String::new(),
            custom_size: String::new(),
            running: true,
            compare_picking: false,
            compare_selected_first: false,
        }
    }

    pub fn run_tui(
        &mut self,
        terminal: &mut ratatui::DefaultTerminal,
    ) -> std::io::Result<()> {
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    pub(crate) fn select_topic(&mut self, idx: usize) {
        self.array = DsaApp::generate_random_array(8);
        self.steps = (TOPICS[idx].gen_steps)(&self.array);
        self.current_step = 0;
        self.screen = Screen::Visualizer;
    }

    pub(crate) fn reset_visualizer(&mut self) {
        let topic = &TOPICS[self.selected];
        self.steps = (topic.gen_steps)(&self.array);
        self.current_step = 0;
        self.screen = Screen::Visualizer;
    }

    pub(crate) fn apply_custom_array(&mut self, values: Vec<u32>) {
        self.array = values;
        self.reset_visualizer();
    }

    pub(crate) fn start_compare(&mut self) {
        self.array = DsaApp::generate_random_array(8);
        self.steps = (TOPICS[self.selected].gen_steps)(&self.array);
        self.steps2 = (TOPICS[self.selected2].gen_steps)(&self.array);
        self.current_step = 0;
        self.current_step2 = 0;
        self.compare_picking = false;
        self.compare_selected_first = false;
        self.screen = Screen::Compare;
    }
}
