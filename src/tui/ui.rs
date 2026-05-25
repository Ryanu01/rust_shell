use super::{is_executable, App, OutputStyle};
use crate::JOBS;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

impl App {
    pub(crate) fn draw(&self, frame: &mut Frame) {
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
        let mode_indicator = if self.mouse_capture {
            " [F2] select "
        } else {
            " [SELECT] F2 "
        };
        let mode_style = if self.mouse_capture {
            Style::default()
                .fg(Color::Rgb(150, 150, 150))
                .bg(Color::Rgb(30, 30, 30))
        } else {
            Style::default().fg(Color::Yellow).bg(Color::Rgb(60, 30, 0))
        };
        let status_text = format!(
            " {}  |  jobs: {}  |  [Ctrl+D] exit  |  [Ctrl+L] clear  |  [Ctrl+Shift+C] copy  |  [Ctrl+Shift+V] paste{}",
            self.cwd, job_count, mode_indicator,
        );
        let status_para = Paragraph::new(Line::raw(status_text))
            .style(Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 30)));
        frame.render_widget(status_para, status_area);

        let mode_x = area.width.saturating_sub(mode_indicator.len() as u16 + 1);
        let mode_para = Paragraph::new(Line::raw(mode_indicator)).style(mode_style);
        frame.render_widget(
            mode_para,
            Rect::new(mode_x, 0, area.width.saturating_sub(mode_x), 1),
        );

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
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    "  / __ \\__  _______/ /_",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    " / /_/ / / / / ___/ __ \\",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    "/ _, _/ /_/ (__  ) / / /",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    "/_/ |_|\\__,_/____/_/ /_/",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Line::raw(""),
                Line::styled(
                    "    Welcome to rush!",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Line::raw(""),
                Line::raw("  Type a command and press Enter."),
                Line::raw("  Use Tab for completion, PageUp/Down or mouse wheel to scroll."),
            ]
        } else {
            self.output_lines[start..]
                .iter()
                .map(|(text, style)| match style {
                    OutputStyle::Command => Line::styled(
                        format!("$ {}", text),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                    OutputStyle::Directory => {
                        Line::styled(text.clone(), Style::default().fg(Color::Red))
                    }
                    OutputStyle::Executable => {
                        Line::styled(text.clone(), Style::default().fg(Color::Green))
                    }
                    OutputStyle::Plain if text.is_empty() => Line::raw(""),
                    OutputStyle::Plain => Line::raw(text.clone()),
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
                    Style::default().bg(Color::Rgb(100, 100, 100)).fg(Color::White),
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

        let cursor_x = input_area.x + 1 + prompt.len() as u16 + self.cursor as u16;
        let cursor_y = input_area.y + 1;
        frame.set_cursor_position((
            cursor_x.min(input_area.x + input_area.width.saturating_sub(2)),
            cursor_y,
        ));
    }
}
