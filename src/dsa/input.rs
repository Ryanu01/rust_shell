use super::{DsaApp, Screen};
use rand::Rng;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

impl DsaApp {
    pub(crate) fn handle_events(&mut self) -> std::io::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                match self.screen {
                    Screen::Menu => self.handle_menu_key(key.code),
                    Screen::Visualizer => self.handle_visualizer_key(key.code),
                    Screen::Compare => self.handle_compare_key(key.code),
                    Screen::InputSize => self.handle_input_size_key(key.code),
                    Screen::InputValues => self.handle_input_values_key(key.code),
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_menu_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.selected + 1 < self.topic_count {
                    self.selected += 1;
                }
            }
            KeyCode::Enter => {
                if self.compare_picking {
                    if !self.compare_selected_first {
                        self.selected2 = self.selected;
                        self.compare_selected_first = true;
                    } else {
                        self.start_compare();
                    }
                } else {
                    self.select_topic(self.selected);
                }
            }
            KeyCode::Char('c') => {
                self.compare_picking = !self.compare_picking;
                self.compare_selected_first = false;
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.compare_picking {
                    self.compare_picking = false;
                    self.compare_selected_first = false;
                } else {
                    self.running = false;
                }
            }
            _ => {}
        }
    }

    fn handle_visualizer_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Right | KeyCode::Char(' ') => {
                if self.current_step + 1 < self.steps.len() {
                    self.current_step += 1;
                }
            }
            KeyCode::Left => {
                if self.current_step > 0 {
                    self.current_step -= 1;
                }
            }
            KeyCode::Char('r') => {
                self.array = DsaApp::generate_random_array(8);
                self.reset_visualizer();
            }
            KeyCode::Char('m') => {
                self.screen = Screen::Menu;
            }
            KeyCode::Enter => {
                self.custom_size.clear();
                self.custom_input.clear();
                self.screen = Screen::InputSize;
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
            }
            _ => {}
        }
    }

    fn handle_compare_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Right | KeyCode::Char(' ') => {
                if self.current_step + 1 < self.steps.len() {
                    self.current_step += 1;
                }
                if self.current_step2 + 1 < self.steps2.len() {
                    self.current_step2 += 1;
                }
            }
            KeyCode::Left => {
                if self.current_step > 0 {
                    self.current_step -= 1;
                }
                if self.current_step2 > 0 {
                    self.current_step2 -= 1;
                }
            }
            KeyCode::Char('r') => {
                self.array = DsaApp::generate_random_array(8);
                self.steps = (super::TOPICS[self.selected].gen_steps)(&self.array);
                self.steps2 = (super::TOPICS[self.selected2].gen_steps)(&self.array);
                self.current_step = 0;
                self.current_step2 = 0;
            }
            KeyCode::Char('m') => {
                self.compare_picking = false;
                self.compare_selected_first = false;
                self.screen = Screen::Menu;
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
            }
            _ => {}
        }
    }

    fn handle_input_size_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.custom_size.push(c);
            }
            KeyCode::Backspace => {
                self.custom_size.pop();
            }
            KeyCode::Enter => {
                let size: usize = self.custom_size.parse().unwrap_or(0);
                if size >= 2 && size <= 20 {
                    self.custom_input.clear();
                    self.screen = Screen::InputValues;
                }
            }
            KeyCode::Esc => {
                self.custom_size.clear();
                self.screen = Screen::Visualizer;
            }
            _ => {}
        }
    }

    fn handle_input_values_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) if c.is_ascii_digit() || c == ' ' => {
                self.custom_input.push(c);
            }
            KeyCode::Backspace => {
                self.custom_input.pop();
            }
            KeyCode::Enter => {
                let size: usize = self.custom_size.parse().unwrap_or(0);
                let values: Vec<u32> = self
                    .custom_input
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if values.len() == size {
                    self.custom_size.clear();
                    self.custom_input.clear();
                    self.apply_custom_array(values);
                }
            }
            KeyCode::Esc => {
                self.custom_size.clear();
                self.custom_input.clear();
                self.screen = Screen::Visualizer;
            }
            _ => {}
        }
    }
}

impl DsaApp {
    pub(crate) fn generate_random_array(size: usize) -> Vec<u32> {
        let mut rng = rand::thread_rng();
        (0..size).map(|_| rng.gen_range(1..=99)).collect()
    }
}
