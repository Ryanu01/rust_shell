use super::{App, OutputStyle};
use crate::utils::{get_file_matches, longest_common_prefix};
use crate::COMPLETION_SPEC;
use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use ratatui::crossterm::execute;
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

impl App {
    pub(crate) fn handle_events(&mut self) -> std::io::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.on_key(key);
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.scroll_offset += 5;
                }
                MouseEventKind::ScrollDown => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(5);
                }
                _ => {}
            },
            Event::Resize(_, _) => {
                self.update_cwd();
            }
            _ => {}
        }
        Ok(())
    }

    fn on_key(&mut self, key: KeyEvent) {
        if key.code != KeyCode::Tab && !self.completions.is_empty() {
            self.completions.clear();
        }

        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers == KeyModifiers::CONTROL {
                    match c {
                        'c' => {
                            self.output_lines.push(("^C".to_string(), OutputStyle::Plain));
                            self.input.clear();
                            self.cursor = 0;
                            self.history_idx = None;
                        }
                        'd' => {
                            if self.input.is_empty() {
                                self.running = false;
                            }
                        }
                        'u' => {
                            self.input.clear();
                            self.cursor = 0;
                        }
                        'l' => {
                            self.output_lines.clear();
                            self.scroll_offset = 0;
                        }
                        'C' => {
                            self.copy_output_to_clipboard();
                        }
                        'V' => {
                            self.paste_from_clipboard();
                        }
                        _ => {}
                    }
                } else if key.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) {
                    match c {
                        'c' | 'C' => self.copy_output_to_clipboard(),
                        'v' | 'V' => self.paste_from_clipboard(),
                        _ => {}
                    }
                } else {
                    self.input.insert(self.cursor, c);
                    self.cursor += 1;
                    self.history_idx = None;
                }
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                }
            }
            KeyCode::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                }
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor < self.input.len() {
                    self.cursor += 1;
                }
            }
            KeyCode::Home | KeyCode::End => {
                if key.code == KeyCode::Home {
                    self.cursor = 0;
                } else {
                    self.cursor = self.input.len();
                }
            }
            KeyCode::PageUp => {
                self.scroll_offset += 10;
            }
            KeyCode::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            KeyCode::Up => {
                if self.history.is_empty() {
                    return;
                }
                let idx = match self.history_idx {
                    None => self.history.len() - 1,
                    Some(i) if i > 0 => i - 1,
                    _ => return,
                };
                self.history_idx = Some(idx);
                self.input = self.history[idx].clone();
                self.cursor = self.input.len();
            }
            KeyCode::Down => match self.history_idx {
                None => {}
                Some(i) if i < self.history.len() - 1 => {
                    self.history_idx = Some(i + 1);
                    self.input = self.history[i + 1].clone();
                    self.cursor = self.input.len();
                }
                _ => {
                    self.history_idx = None;
                    self.input.clear();
                    self.cursor = 0;
                }
            },
            KeyCode::Enter => {
                let cmd = self.input.trim().to_string();
                if !cmd.is_empty() {
                    self.history.push(cmd.clone());
                    self.output_lines.push((cmd.clone(), OutputStyle::Command));
                    self.execute_cmd(&cmd);
                    self.update_cwd();
                    self.output_lines.push((String::new(), OutputStyle::Plain));
                }
                self.input.clear();
                self.cursor = 0;
                self.history_idx = None;
                self.scroll_to_bottom();
            }
            KeyCode::Tab => {
                self.handle_tab();
            }
            KeyCode::F(2) => {
                self.toggle_mouse_capture();
            }
            _ => {}
        }
    }

    fn handle_tab(&mut self) {
        if !self.completions.is_empty() {
            self.completions.clear();
            return;
        }

        let pos = self.cursor;
        let start = self.input[..pos].rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word = &self.input[start..pos];
        let is_command = !self.input[..pos].contains(' ');

        let candidates = if word.is_empty() {
            if is_command {
                self.get_all_commands()
            } else {
                Vec::new()
            }
        } else if is_command {
            self.complete_command(word)
        } else {
            self.complete_arg(word, start)
        };

        if candidates.is_empty() {
            return;
        }

        let lcp = longest_common_prefix(&candidates);

        if candidates.len() == 1 {
            let insert = candidates[0].clone();
            self.input.replace_range(start..pos, &insert);
            self.cursor = start + insert.len();
            return;
        }

        if lcp.len() > word.len() {
            let extra = &lcp[word.len()..];
            self.input.insert_str(pos, extra);
            self.cursor = pos + extra.len();
            self.completions = candidates;
            self.completion_start = start;
            return;
        }

        self.completions = candidates;
        self.completion_start = start;
    }

    fn get_all_commands(&self) -> Vec<String> {
        let mut commands: Vec<String> = vec![
            "echo".into(),
            "exit".into(),
            "type".into(),
            "pwd".into(),
            "cd".into(),
            "complete".into(),
            "jobs".into(),
            "history".into(),
            "declare".into(),
        ];

        if let Ok(path_var) = env::var("PATH") {
            for path in env::split_paths(&path_var) {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            commands.push(name.to_string());
                        }
                    }
                }
            }
        }

        commands.sort();
        commands.dedup();
        commands
    }

    fn complete_command(&self, word: &str) -> Vec<String> {
        self.get_all_commands()
            .into_iter()
            .filter(|cmd| cmd.starts_with(word))
            .collect()
    }

    fn complete_arg(&self, word: &str, word_start: usize) -> Vec<String> {
        let cmd_name = self.input[..self.cursor]
            .split_whitespace()
            .next()
            .unwrap_or("");
        let completer_path = COMPLETION_SPEC.lock().unwrap().get(cmd_name).cloned();

        match completer_path.and_then(|path| {
            let prev_word = self.input[..word_start]
                .trim()
                .split_whitespace()
                .last()
                .unwrap_or("");
            let output = Command::new(&path)
                .arg(cmd_name)
                .arg(word)
                .arg(prev_word)
                .env("COMP_LINE", &self.input)
                .env("COMP_POINT", self.cursor.to_string())
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let stdout = String::from_utf8(output.stdout).ok()?;
            let candidates: Vec<String> = stdout
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && l.starts_with(word))
                .collect();
            if candidates.is_empty() {
                None
            } else {
                Some(candidates)
            }
        }) {
            Some(candidates) => candidates,
            None => get_file_matches(word),
        }
    }

    fn toggle_mouse_capture(&mut self) {
        self.mouse_capture = !self.mouse_capture;
        if self.mouse_capture {
            let _ = execute!(
                std::io::stdout(),
                ratatui::crossterm::event::EnableMouseCapture
            );
            self.output_lines
                .push(("Mouse capture enabled (F2 to toggle)".into(), OutputStyle::Plain));
        } else {
            let _ = execute!(
                std::io::stdout(),
                ratatui::crossterm::event::DisableMouseCapture
            );
            self.output_lines.push((
                "Selection mode: click and drag to select text, Ctrl+Shift+C to copy".into(),
                OutputStyle::Plain,
            ));
        }
    }

    fn set_clipboard_text(text: &str) -> Result<(), String> {
        if let Ok(mut ctx) = arboard::Clipboard::new() {
            return ctx.set_text(text).map_err(|e| format!("arboard: {}", e));
        }
        #[cfg(target_os = "linux")]
        {
            for (cmd, args) in [
                ("wl-copy", &[] as &[&str]),
                ("xclip", &["-selection", "clipboard"] as &[&str]),
                ("xsel", &["-b"] as &[&str]),
            ] {
                if let Ok(mut child) = Command::new(cmd).args(args).stdin(Stdio::piped()).spawn() {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(text.as_bytes());
                        let _ = child.wait();
                        return Ok(());
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                    let _ = child.wait();
                    return Ok(());
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Ok(mut child) = Command::new("clip").stdin(Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                    let _ = child.wait();
                    return Ok(());
                }
            }
        }
        Err("no clipboard tool found (try installing xclip, wl-clipboard, or pbcopy)".into())
    }

    fn get_clipboard_text() -> Result<String, String> {
        if let Ok(mut ctx) = arboard::Clipboard::new() {
            return ctx.get_text().map_err(|e| format!("arboard: {}", e));
        }
        #[cfg(target_os = "linux")]
        {
            for cmd in &["wl-paste", "xclip -selection clipboard -o", "xsel -b"] {
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if let Ok(output) = Command::new(parts[0]).args(&parts[1..]).output() {
                    if output.status.success() {
                        let text = String::from_utf8_lossy(&output.stdout).to_string();
                        if !text.is_empty() {
                            return Ok(text);
                        }
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("pbpaste").output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).to_string();
                    if !text.is_empty() {
                        return Ok(text);
                    }
                }
            }
        }
        Err("no clipboard tool found (try installing xclip, wl-clipboard, or pbpaste)".into())
    }

    fn copy_output_to_clipboard(&mut self) {
        let text: String = self
            .output_lines
            .iter()
            .map(|(line, _)| line.as_str())
            .collect::<Vec<&str>>()
            .join("\n");
        if text.is_empty() {
            return;
        }
        match Self::set_clipboard_text(&text) {
            Ok(()) => self
                .output_lines
                .push(("Copied output to clipboard".into(), OutputStyle::Plain)),
            Err(e) => self
                .output_lines
                .push((format!("Clipboard error: {}", e), OutputStyle::Plain)),
        }
    }

    fn paste_from_clipboard(&mut self) {
        match Self::get_clipboard_text() {
            Ok(text) => {
                for c in text.chars() {
                    self.input.insert(self.cursor, c);
                    self.cursor += 1;
                }
                self.history_idx = None;
            }
            Err(e) => self
                .output_lines
                .push((format!("Clipboard error: {}", e), OutputStyle::Plain)),
        }
    }
}
