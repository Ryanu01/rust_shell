use ratatui::Frame;
use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use std::env;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::utils::{get_file_matches, longest_common_prefix};

use crate::expand_vars;
use crate::{
    COMPLETION_SPEC, JOBS, cmd_cd, cmd_complete, cmd_declare, cmd_echo, cmd_jobs, cmd_pwd,
    cmd_type, is_builtin,
};

#[derive(Clone, Copy, PartialEq)]
enum OutputStyle {
    Plain,
    Command,
    Directory,
    Executable,
}

fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

pub struct App {
    input: String,
    cursor: usize,
    output_lines: Vec<(String, OutputStyle)>,
    scroll_offset: usize,
    history: Vec<String>,
    history_idx: Option<usize>,
    running: bool,
    cwd: String,
    completions: Vec<String>,
    completion_start: usize,
}

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

    fn run_tui(&mut self, terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
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

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

        let (status_area, output_area, input_area) = (chunks[0], chunks[1], chunks[2]);

        // Status bar
        let job_count = JOBS.lock().unwrap().len();
        let status_text = format!(
            " {}  |  jobs: {}  |  [Ctrl+D] exit  |  [Ctrl+L] clear",
            self.cwd, job_count
        );
        let status_para = Paragraph::new(Line::raw(status_text))
            .style(Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 30)));
        frame.render_widget(status_para, status_area);

        // Output pane
        let output_height = output_area.height.saturating_sub(1) as usize;
        let total_lines = self.output_lines.len();

        let scroll = if total_lines > output_height {
            total_lines.saturating_sub(output_height)
        } else {
            0
        };

        let start = if self.scroll_offset > 0 {
            total_lines
                .saturating_sub(output_height)
                .saturating_sub(self.scroll_offset)
        } else {
            scroll
        };
        let start = start.min(total_lines.saturating_sub(1));

        let visible_lines: Vec<Line> = if total_lines == 0 {
            vec![
                Line::raw(""),
                Line::styled(
                    "   ____            __",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    "  / __ \\__  _______/ /_",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    " / /_/ / / / / ___/ __ \\",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    "/ _, _/ /_/ (__  ) / / /",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    "/_/ |_|\\__,_/____/_/ /_/",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Line::raw(""),
                Line::styled(
                    "    Welcome to rush! 🦀 >_",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Line::raw(""),
                Line::raw("  Type a command and press Enter."),
                Line::raw("  Use Tab for completion, PageUp/Down or mouse wheel to scroll."),
            ]
        } else {
            self.output_lines[start..]
                .iter()
                .map(|(text, style)| {
                    match style {
                        OutputStyle::Command => Line::styled(
                            format!("$ {}", text),
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                        OutputStyle::Directory => Line::styled(
                            text.clone(),
                            Style::default().fg(Color::Red),
                        ),
                        OutputStyle::Executable => Line::styled(
                            text.clone(),
                            Style::default().fg(Color::Green),
                        ),
                        OutputStyle::Plain if text.is_empty() => Line::raw(""),
                        OutputStyle::Plain => Line::raw(text.clone()),
                    }
                })
                .collect()
        };

        let output_block = Block::default()
            .borders(Borders::TOP)
            .title(" Output ")
            .style(Style::default().bg(Color::Rgb(20, 20, 20)));
        let output_para = Paragraph::new(visible_lines).block(output_block);
        frame.render_widget(output_para, output_area);

        // Completion popup
        if !self.completions.is_empty() {
            let popup_height = (self.completions.len() as u16).min(8) + 2;
            let popup_area = Rect::new(
                input_area.x,
                input_area.y.saturating_sub(popup_height),
                input_area.width.min(60),
                popup_height,
            );
            let items: Vec<ListItem> = self
                .completions
                .iter()
                .map(|c| {
                    let path = std::path::Path::new(c.trim_end_matches('/'));
                    let style = if path.is_dir() {
                        Style::default().fg(Color::Red)
                    } else if is_executable(path) {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    };
                    ListItem::new(c.as_str()).style(style)
                })
                .collect();
            let popup = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Completions ")
                        .style(Style::default().bg(Color::Rgb(30, 30, 30))),
                )
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            frame.render_widget(popup, popup_area);
        }

        // Input bar
        let prompt = "$ ";
        let input_text = if self.input.is_empty() {
            Line::raw(format!("{} ", prompt))
        } else {
            let before = &self.input[..self.cursor];
            let after = &self.input[self.cursor..];
            Line::from(vec![
                Span::raw(prompt),
                Span::raw(before),
                Span::styled(
                    if after.is_empty() { " " } else { &after[..1] },
                    Style::default()
                        .bg(Color::Rgb(100, 100, 100))
                        .fg(Color::White),
                ),
                Span::raw(if after.is_empty() { "" } else { &after[1..] }),
            ])
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(" Input ")
            .style(Style::default().bg(Color::Rgb(20, 20, 20)));
        let input_para = Paragraph::new(input_text).block(input_block);
        frame.render_widget(input_para, input_area);

        // Set the terminal cursor position at the input area
        let cursor_x = input_area.x + 1 + prompt.len() as u16 + self.cursor as u16;
        let cursor_y = input_area.y + 1;
        frame.set_cursor_position((
            cursor_x.min(input_area.x + input_area.width.saturating_sub(2)),
            cursor_y,
        ));
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
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
        // Hide completions on any key except Tab
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
            _ => {}
        }
    }

    fn handle_tab(&mut self) {
        if !self.completions.is_empty() {
            // Already showing completions — hide on next tab
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
            // Keep showing completions for further narrowing
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

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    fn update_cwd(&mut self) {
        self.cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".to_string());
    }

    fn cmd_ls_tui(&mut self, args: &[&str]) {
        let path = args
            .iter()
            .find(|a| !a.starts_with('-'))
            .map(|s| s.to_string())
            .unwrap_or_else(|| ".".to_string());

        let entries = match fs::read_dir(&path) {
            Ok(e) => e,
            Err(e) => {
                self.output_lines
                    .push((format!("ls: cannot access '{}': {}", path, e), OutputStyle::Plain));
                return;
            }
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();
        let mut executables = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            match entry.file_type() {
                Ok(t) if t.is_dir() => dirs.push(name),
                Ok(t) if t.is_symlink() => {
                    if let Ok(meta) = entry.metadata() {
                        if meta.is_dir() {
                            dirs.push(name);
                        } else {
                            files.push(name);
                        }
                    } else {
                        files.push(name);
                    }
                }
                _ => {
                    if is_executable(&entry.path()) {
                        executables.push(name);
                    } else {
                        files.push(name);
                    }
                }
            }
        }

        dirs.sort();
        files.sort();
        executables.sort();

        for d in &dirs {
            self.output_lines.push((format!("{}/", d), OutputStyle::Directory));
        }
        for f in &files {
            self.output_lines.push((f.clone(), OutputStyle::Plain));
        }
        for e in &executables {
            self.output_lines.push((format!("{}*", e), OutputStyle::Executable));
        }
    }

    fn execute_cmd(&mut self, cmd: &str) {
        let parts = match shell_words::split(cmd) {
            Ok(p) => p,
            Err(e) => {
                self.output_lines
                    .push((format!("rush: parse error: {}", e), OutputStyle::Plain));
                return;
            }
        };

        if parts.is_empty() {
            return;
        }

        let parts = expand_vars(parts);
        let parts: Vec<String> = parts.into_iter().filter(|s| !s.is_empty()).collect();
        let slices: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();

        if slices.is_empty() {
            return;
        }

        if slices.last() == Some(&"&") {
            let mut bg_slices = slices.clone();
            bg_slices.pop();
            self.run_external_bg(bg_slices, &parts.join(" "));
            return;
        }

        let mut pipe_positions = Vec::new();
        for (i, &s) in slices.iter().enumerate() {
            if s == "|" {
                pipe_positions.push(i);
            }
        }

        if !pipe_positions.is_empty() {
            self.run_pipeline_tui(&slices, &pipe_positions);
            return;
        }

        let command = slices[0];

        if command == "exit" {
            self.running = false;
            return;
        }

        if command == "clear" {
            self.output_lines.clear();
            self.scroll_offset = 0;
            return;
        }

        if command == "ls" {
            self.cmd_ls_tui(&slices[1..]);
            return;
        }

        if command == "history" {
            for (i, entry) in self.history.iter().enumerate() {
                self.output_lines
                    .push((format!(" {} {}", i + 1, entry), OutputStyle::Plain));
            }
            return;
        }

        if command == "cd" {
            let mut buf: Vec<u8> = Vec::new();
            cmd_cd(slices[1..].to_vec(), &mut buf);
            if !buf.is_empty() {
                self.output_lines
                    .push((String::from_utf8_lossy(&buf).to_string(), OutputStyle::Plain));
            }
            return;
        }

        if is_builtin(command) {
            let mut buf: Vec<u8> = Vec::new();
            match command {
                "echo" => cmd_echo(slices[1..].to_vec(), &mut buf),
                "type" => cmd_type(slices[1..].to_vec(), &mut buf),
                "pwd" => cmd_pwd(&mut buf),
                "complete" => cmd_complete(slices[1..].to_vec(), &mut buf),
                "jobs" => cmd_jobs(slices[1..].to_vec(), &mut buf),
                "declare" => cmd_declare(slices[1..].to_vec(), &mut buf),
                _ => {}
            }
            if !buf.is_empty() {
                self.output_lines
                    .push((String::from_utf8_lossy(&buf).to_string(), OutputStyle::Plain));
            }
            return;
        }

        self.run_external_tui(&slices);
    }

    fn run_external_tui(&mut self, parts: &[&str]) {
        let command = parts[0];
        let args = &parts[1..];

        let output = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(out) => {
                if !out.stdout.is_empty() {
                    let text = String::from_utf8_lossy(&out.stdout).to_string();
                    for line in text.lines() {
                        self.output_lines.push((line.to_string(), OutputStyle::Plain));
                    }
                }
                if !out.stderr.is_empty() {
                    let text = String::from_utf8_lossy(&out.stderr).to_string();
                    for line in text.lines() {
                        self.output_lines.push((line.to_string(), OutputStyle::Plain));
                    }
                }
                if !out.status.success() && out.stdout.is_empty() && out.stderr.is_empty() {
                    self.output_lines
                        .push((format!("{}: command not found", command), OutputStyle::Plain));
                }
            }
            Err(e) => {
                self.output_lines
                    .push((format!("{}: {}", command, e), OutputStyle::Plain));
            }
        }
    }

    fn run_external_bg(&mut self, parts: Vec<&str>, full_cmd: &str) {
        let command = parts[0];
        let args = &parts[1..];

        match Command::new(command)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                let pid = child.id();
                let mut jobs = JOBS.lock().unwrap();
                let job_id = jobs.len() + 1;
                jobs.push(crate::Job {
                    id: job_id,
                    child,
                    command: full_cmd.to_string(),
                });
                self.output_lines
                    .push((format!("[{}] {}", job_id, pid), OutputStyle::Plain));
            }
            Err(e) => {
                self.output_lines
                    .push((format!("{}: {}", command, e), OutputStyle::Plain));
            }
        }
    }

    fn run_pipeline_tui(&mut self, slices: &[&str], pipe_positions: &[usize]) {
        let mut segments: Vec<Vec<&str>> = Vec::new();
        let mut start = 0;
        for &pos in pipe_positions {
            segments.push(slices[start..pos].to_vec());
            start = pos + 1;
        }
        segments.push(slices[start..].to_vec());

        for segment in &segments {
            if !is_builtin(segment[0]) {
                if Command::new(segment[0])
                    .arg("--invalid-flag-to-test-existence")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .is_err()
                {
                    self.output_lines
                        .push((format!("{}: command not found", segment[0]), OutputStyle::Plain));
                    return;
                }
            }
        }

        let has_builtins = segments.iter().any(|s| is_builtin(s[0]));
        if has_builtins {
            self.run_seq_pipeline_tui(&segments);
        } else {
            self.run_conc_pipeline_tui(&segments);
        }
    }

    fn run_conc_pipeline_tui(&mut self, segments: &[Vec<&str>]) {
        let n = segments.len();
        let mut children = Vec::new();
        let mut prev_stdout: Option<std::process::ChildStdout> = None;

        for (i, seg) in segments.iter().enumerate() {
            let mut cmd = Command::new(seg[0]);
            cmd.args(&seg[1..]);
            if prev_stdout.is_some() {
                cmd.stdin(Stdio::piped());
            }
            // Always pipe stdout in TUI mode to capture output
            cmd.stdout(Stdio::piped());
            match cmd.spawn() {
                Ok(mut child) => {
                    if let Some(mut input) = prev_stdout.take() {
                        if let Some(mut stdin) = child.stdin.take() {
                            std::thread::spawn(move || {
                                let _ = std::io::copy(&mut input, &mut stdin);
                            });
                        }
                    }
                    if i < n - 1 {
                        prev_stdout = child.stdout.take();
                    }
                    children.push(child);
                }
                Err(e) => {
                    self.output_lines
                        .push((format!("{}: {}", seg[0], e), OutputStyle::Plain));
                    return;
                }
            }
        }

        // Wait for all children and collect last output
        if let Some(last) = children.pop() {
            // pipe the last child's stdout if not already piped
            let output = last.wait_with_output().ok();
            for child in &mut children {
                let _ = child.wait();
            }
            if let Some(out) = output {
                if !out.stdout.is_empty() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for line in text.lines() {
                        self.output_lines.push((line.to_string(), OutputStyle::Plain));
                    }
                }
                if !out.stderr.is_empty() {
                    let text = String::from_utf8_lossy(&out.stderr);
                    for line in text.lines() {
                        self.output_lines.push((line.to_string(), OutputStyle::Plain));
                    }
                }
            }
        } else {
            for child in &mut children {
                let _ = child.wait();
            }
        }
    }

    fn run_seq_pipeline_tui(&mut self, segments: &[Vec<&str>]) {
        let n = segments.len();
        let mut prev_output: Option<Vec<u8>> = None;

        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == n - 1;
            let cmd = segment[0];
            let args = segment[1..].to_vec();

            if is_builtin(cmd) {
                let mut buf: Vec<u8> = Vec::new();
                let w: &mut dyn Write = &mut buf;
                match cmd {
                    "echo" => cmd_echo(args, w),
                    "type" => cmd_type(args, w),
                    "pwd" => cmd_pwd(w),
                    "cd" => cmd_cd(args, w),
                    "complete" => cmd_complete(args, w),
                    "jobs" => cmd_jobs(args, w),
                    "declare" => cmd_declare(args, w),
                    _ => {}
                }
                if is_last {
                    if !buf.is_empty() {
                        let text = String::from_utf8_lossy(&buf);
                        for line in text.lines() {
                            self.output_lines.push((line.to_string(), OutputStyle::Plain));
                        }
                    }
                } else {
                    prev_output = Some(buf);
                }
            } else {
                let mut command = Command::new(cmd);
                command.args(&args);

                if prev_output.is_some() {
                    command.stdin(Stdio::piped());
                }
                if !is_last {
                    command.stdout(Stdio::piped());
                }

                match command.spawn() {
                    Ok(mut child) => {
                        if let Some(input) = prev_output.take() {
                            if let Some(mut stdin) = child.stdin.take() {
                                let _ = stdin.write_all(&input);
                            }
                        }

                        if is_last {
                            let output = child.wait_with_output().ok();
                            if let Some(out) = output {
                                if !out.stdout.is_empty() {
                                    let text = String::from_utf8_lossy(&out.stdout);
                                    for line in text.lines() {
                                        self.output_lines.push((line.to_string(), OutputStyle::Plain));
                                    }
                                }
                                if !out.stderr.is_empty() {
                                    let text = String::from_utf8_lossy(&out.stderr);
                                    for line in text.lines() {
                                        self.output_lines.push((line.to_string(), OutputStyle::Plain));
                                    }
                                }
                            }
                        } else {
                            let output = child.wait_with_output().ok();
                            prev_output = output.map(|o| o.stdout);
                        }
                    }
                    Err(e) => {
                        self.output_lines.push((format!("{}: {}", cmd, e), OutputStyle::Plain));
                        return;
                    }
                }
            }
        }
    }
}
