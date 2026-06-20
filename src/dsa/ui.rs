use super::{DsaApp, Screen, Step, TOPICS};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

impl DsaApp {
    pub(crate) fn draw(&self, frame: &mut Frame) {
        match self.screen {
            Screen::Menu => self.draw_menu(frame),
            Screen::Visualizer => self.draw_visualizer(frame),
            Screen::Compare => self.draw_compare(frame),
            Screen::InputSize => self.draw_input_size(frame),
            Screen::InputValues => self.draw_input_values(frame),
        }
    }

    fn draw_menu(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

        let title_text = if self.compare_picking {
            if self.compare_selected_first {
                "  rush DSA — Select second algorithm for comparison"
            } else {
                "  rush DSA — Select first algorithm for comparison"
            }
        } else {
            "  rush DSA — Select a sorting algorithm"
        };
        let title = Paragraph::new(Line::styled(
            title_text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        let mut lines = Vec::new();
        for (i, topic) in TOPICS.iter().enumerate() {
            let (prefix, style) = if self.compare_picking && self.compare_selected_first && i == self.selected2 {
                (" 2>", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else if self.compare_picking && !self.compare_selected_first && i == self.selected {
                (" 1>", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else if i == self.selected {
                (" > ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else {
                ("   ", Style::default().fg(Color::White))
            };
            let stable_str = if topic.stable { "Stable" } else { "Unstable" };
            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("{:<20}", topic.name), style),
                Span::styled(
                    format!(" {:<14} {}", topic.time, stable_str),
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }
        let list = Paragraph::new(lines)
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(list, chunks[1]);

        let help_text = if self.compare_picking {
            "  ↑↓ navigate  Enter select  Esc cancel  q quit"
        } else {
            "  ↑↓ navigate  Enter select  c compare mode  q quit"
        };
        let help = Paragraph::new(Line::styled(help_text, Style::default().fg(Color::DarkGray)))
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, chunks[2]);
    }

    fn draw_algo_box(&self, steps: &[Step], step_idx: usize, topic_idx: usize, _label: &str) -> (Vec<Line<'_>>, Vec<Line<'_>>) {
        let step = &steps[step_idx];
        let topic = &TOPICS[topic_idx];
        let total = steps.len();

        let mut grid_lines = Vec::new();
        let mut value_spans = vec![Span::raw(" ")];
        for (i, val) in step.array.iter().enumerate() {
            let (text, color) = if step.sorted.get(i).copied().unwrap_or(false) {
                (format!("[{:>3}] ", val), Color::Green)
            } else if step.compare.map_or(false, |(a, b)| a == i || b == i) {
                (format!("[{:>3}] ", val), Color::Red)
            } else {
                (format!("[{:>3}] ", val), Color::White)
            };
            value_spans.push(Span::styled(text, Style::default().fg(color)));
        }
        grid_lines.push(Line::from(value_spans));

        let mut label_spans = vec![Span::raw(" ")];
        for (i, _) in step.array.iter().enumerate() {
            let (text, color) = if step.swap.map_or(false, |(a, b)| a == i || b == i) {
                (format!(" SWAP "), Color::Yellow)
            } else if step.compare.map_or(false, |(a, b)| a == i || b == i) {
                (format!(" CMP  "), Color::Red)
            } else if step.sorted.get(i).copied().unwrap_or(false) {
                (format!(" {:^4} ", i), Color::Green)
            } else {
                (format!(" {:^4} ", i), Color::DarkGray)
            };
            label_spans.push(Span::styled(text, Style::default().fg(color)));
        }
        grid_lines.push(Line::from(label_spans));

        let info_lines = vec![
            Line::raw(format!(" {}", step.label)),
            Line::raw(format!(" Step {}/{}", step_idx + 1, total)),
            Line::raw(format!(" Comparisons: {}  Swaps: {}",
                count_comparisons(steps, step_idx), count_swaps(steps, step_idx))),
            Line::raw(format!(" {}  |  Space: {}  |  {}",
                topic.time, topic.space, if topic.stable { "Stable" } else { "Unstable" })),
        ];

        (grid_lines, info_lines)
    }

    fn draw_visualizer(&self, frame: &mut Frame) {
        let area = frame.area();
        let topic = &TOPICS[self.selected];
        let step = &self.steps[self.current_step];
        let total = self.steps.len();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(4),
                Constraint::Length(5),
                Constraint::Length(1),
            ])
            .split(area);

        let title = Line::from(vec![
            Span::styled(
                format!("  {}  ", topic.name),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("Step {}/{}", self.current_step + 1, total), Style::default().fg(Color::Gray)),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1)])
            .margin(1)
            .split(chunks[1]);

        let mut grid_lines = Vec::new();
        let mut value_spans = vec![Span::raw(" ")];
        for (i, val) in step.array.iter().enumerate() {
            let (text, color) = if step.sorted.get(i).copied().unwrap_or(false) {
                (format!("[{:>3}] ", val), Color::Green)
            } else if step.compare.map_or(false, |(a, b)| a == i || b == i) {
                (format!("[{:>3}] ", val), Color::Red)
            } else {
                (format!("[{:>3}] ", val), Color::White)
            };
            value_spans.push(Span::styled(text, Style::default().fg(color)));
        }
        grid_lines.push(Line::from(value_spans));

        let mut label_spans = vec![Span::raw(" ")];
        for (i, _) in step.array.iter().enumerate() {
            let (text, color) = if step.swap.map_or(false, |(a, b)| a == i || b == i) {
                (format!(" SWAP "), Color::Yellow)
            } else if step.compare.map_or(false, |(a, b)| a == i || b == i) {
                (format!(" CMP  "), Color::Red)
            } else if step.sorted.get(i).copied().unwrap_or(false) {
                (format!(" {:^4} ", i), Color::Green)
            } else {
                (format!(" {:^4} ", i), Color::DarkGray)
            };
            label_spans.push(Span::styled(text, Style::default().fg(color)));
        }
        grid_lines.push(Line::from(label_spans));

        frame.render_widget(Paragraph::new(grid_lines), inner[0]);

        let info_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[2]);

        let info_lines = vec![
            Line::raw(format!("  {}", step.label)),
            Line::raw(format!("  Comparisons: {}  |  Swaps: {}",
                count_comparisons(&self.steps, self.current_step),
                count_swaps(&self.steps, self.current_step))),
            Line::raw(""),
            Line::raw(format!("  Time: {}  |  Space: {}  |  {}",
                topic.time, topic.space, if topic.stable { "Stable" } else { "Unstable" })),
        ];
        let info_para = Paragraph::new(info_lines)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(info_para, info_chunks[0]);

        let mut desc_lines: Vec<Line> = topic.desc
            .split(' ').collect::<Vec<&str>>().chunks(6)
            .map(|chunk| Line::raw(format!("  {}", chunk.join(" ")))).collect();
        desc_lines.insert(0, Line::styled("  Description:", Style::default().add_modifier(Modifier::BOLD)));
        let desc_para = Paragraph::new(desc_lines)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(desc_para, info_chunks[1]);

        let help = Line::styled(
            "  ← → step  r reset  Enter custom array  m menu  q quit",
            Style::default().fg(Color::DarkGray),
        );
        frame.render_widget(Paragraph::new(help), chunks[3]);
    }

    fn draw_compare(&self, frame: &mut Frame) {
        let area = frame.area();
        let topic1 = &TOPICS[self.selected];
        let topic2 = &TOPICS[self.selected2];

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(5),
                Constraint::Length(1),
            ])
            .split(area);

        let title = Line::from(vec![
            Span::styled("  Compare: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(topic1.name, Style::default().fg(Color::Yellow)),
            Span::raw(" vs "),
            Span::styled(topic2.name, Style::default().fg(Color::Green)),
        ]);
        frame.render_widget(Paragraph::new(title), chunks[0]);

        let algo_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        let (g1, i1) = self.draw_algo_box(&self.steps, self.current_step, self.selected, "A");
        let (g2, i2) = self.draw_algo_box(&self.steps2, self.current_step2, self.selected2, "B");

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(5)])
            .split(algo_chunks[0]);
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(5)])
            .split(algo_chunks[1]);

        let mut left_grid = g1;
        left_grid.insert(0, Line::styled(
            format!("  {} [A]", topic1.name),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(left_grid), left_chunks[0]);

        let mut right_grid = g2;
        right_grid.insert(0, Line::styled(
            format!("  {} [B]", topic2.name),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(right_grid), right_chunks[0]);

        let info1 = Paragraph::new(i1).block(Block::default().borders(Borders::ALL));
        frame.render_widget(info1, left_chunks[1]);
        let info2 = Paragraph::new(i2).block(Block::default().borders(Borders::ALL));
        frame.render_widget(info2, right_chunks[1]);

        let winner = if topic1.time != topic2.time {
            let better = if is_better_complexity(topic1.time, topic2.time) { topic1.name } else { topic2.name };
            format!("  Winner by time complexity: {}", better)
        } else if topic1.stable && !topic2.stable {
            format!("  Winner: {} (stable)", topic1.name)
        } else if !topic1.stable && topic2.stable {
            format!("  Winner: {} (stable)", topic2.name)
        } else {
            "  Both are comparable — tie!".to_string()
        };

        let compare_info = vec![
            Line::raw(format!("  {}: {}  |  Space: {}  |  Steps: {}/{}  |  Comparisons: {}  |  Swaps: {}",
                "A", topic1.time, topic1.space,
                self.current_step + 1, self.steps.len(),
                count_comparisons(&self.steps, self.current_step),
                count_swaps(&self.steps, self.current_step))),
            Line::raw(format!("  {}: {}  |  Space: {}  |  Steps: {}/{}  |  Comparisons: {}  |  Swaps: {}",
                "B", topic2.time, topic2.space,
                self.current_step2 + 1, self.steps2.len(),
                count_comparisons(&self.steps2, self.current_step2),
                count_swaps(&self.steps2, self.current_step2))),
            Line::raw(winner),
        ];
        let compare_para = Paragraph::new(compare_info)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(compare_para, chunks[2]);

        let help = Line::styled(
            "  ← → step  r reset  m menu  q quit",
            Style::default().fg(Color::DarkGray),
        );
        frame.render_widget(Paragraph::new(help), chunks[3]);
    }

    fn draw_input_size(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(area);

        let title = Paragraph::new(Line::styled(
            "  Custom Array Input",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        let input = format!("  Enter array size (2-20): {}", self.custom_size);
        frame.render_widget(
            Paragraph::new(Line::styled(input, Style::default().fg(Color::Yellow)))
                .block(Block::default().borders(Borders::ALL)),
            chunks[1],
        );

        frame.render_widget(
            Paragraph::new(Line::styled(
                "  Type a number  Enter confirm  Esc cancel",
                Style::default().fg(Color::DarkGray),
            ))
            .block(Block::default().borders(Borders::ALL)),
            chunks[2],
        );
    }

    fn draw_input_values(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Custom Array Input", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(format!("  (size: {})", self.custom_size)),
            ]))
            .block(Block::default().borders(Borders::ALL)),
            chunks[0],
        );

        let prompt = format!("  Enter {} space-separated values: {}", self.custom_size, self.custom_input);
        frame.render_widget(
            Paragraph::new(Line::styled(prompt, Style::default().fg(Color::Yellow)))
                .block(Block::default().borders(Borders::ALL)),
            chunks[1],
        );

        frame.render_widget(
            Paragraph::new(Line::styled(
                "  Type numbers separated by spaces  Enter confirm  Esc cancel",
                Style::default().fg(Color::DarkGray),
            ))
            .block(Block::default().borders(Borders::ALL)),
            chunks[2],
        );
    }
}

fn count_comparisons(steps: &[Step], up_to: usize) -> usize {
    steps[..=up_to].iter().filter(|s| s.compare.is_some()).count()
}

fn count_swaps(steps: &[Step], up_to: usize) -> usize {
    steps[..=up_to].iter().filter(|s| s.swap.is_some()).count()
}

fn is_better_complexity(a: &str, b: &str) -> bool {
    let order: &[&str] = &["O(1)", "O(log n)", "O(n)", "O(n log n)", "O(n²)", "O(2ⁿ)", "O(n!)"];
    let rank = |s: &str| order.iter().position(|&o| s.starts_with(o)).unwrap_or(usize::MAX);
    rank(a) < rank(b)
}
