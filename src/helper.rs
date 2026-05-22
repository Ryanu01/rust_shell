use std::cell::RefCell;
use std::process::Command;
use std::{env, fs};

use crate::COMPLETION_SPEC;

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

use crate::utils::{get_file_matches, longest_common_prefix};

pub struct ShellCompleter {
    pub last_completed: RefCell<String>,
}

impl Helper for ShellCompleter {}
impl Hinter for ShellCompleter {
    type Hint = String;
}
impl Highlighter for ShellCompleter {}
impl Validator for ShellCompleter {}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mut commands = vec![
            "echo".to_string(),
            "exit".to_string(),
            "type".to_string(),
            "pwd".to_string(),
            "cd".to_string(),
            "complete".to_string(),
            "jobs".to_string(),
            "history".to_string(),
        ];

        if let Ok(path_var) = env::var("PATH") {
            for path in env::split_paths(&path_var) {
                if let Ok(entries) = fs::read_dir(path) {
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

        let start = line[..pos].rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word = &line[start..pos];
        let is_command = !line[..pos].contains(' ');

        let candidates: Vec<String> = if is_command {
            commands
                .into_iter()
                .filter(|cmd| cmd.starts_with(word))
                .collect()
        } else {
            let cmd_name = line[..pos].split_whitespace().next().unwrap_or("");
            let prev_word = line[..start].trim().split_whitespace().last().unwrap_or("");
            let completer_path = COMPLETION_SPEC.lock().unwrap().get(cmd_name).cloned();
            match completer_path.and_then(|path| {
                let output = Command::new(&path)
                    .arg(cmd_name)
                    .arg(word)
                    .arg(prev_word)
                    .env("COMP_LINE", line)
                    .env("COMP_POINT", pos.to_string())
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
        };

        let mut matches: Vec<Pair> = candidates
            .iter()
            .map(|c| Pair {
                display: c.clone(),
                replacement: if c.ends_with('/') {
                    c.clone()
                } else {
                    format!("{} ", c)
                },
            })
            .collect();

        matches.sort_by(|a, b| a.display.cmp(&b.display));
        matches.dedup_by(|a, b| a.display == b.display);

        let names: Vec<String> = matches.iter().map(|m| m.display.clone()).collect();
        let lcp = longest_common_prefix(&names);

        // The line as it would appear after inserting lcp
        let line_after_lcp = format!("{}{}", &line[..start], lcp);

        // No matches
        if matches.is_empty() {
            *self.last_completed.borrow_mut() = String::new();
            return Ok((start, vec![]));
        }

        // Single match — insert it directly
        if matches.len() == 1 {
            *self.last_completed.borrow_mut() = String::new();
            return Ok((start, matches));
        }

        // Multiple matches from here down
        let last = self.last_completed.borrow().clone();

        // LCP goes further than what's typed — complete to LCP
        // e.g. typed "tes", lcp="test/" → insert "test/"
        if lcp.len() > word.len() {
            *self.last_completed.borrow_mut() = line_after_lcp;
            return Ok((
                start,
                vec![Pair {
                    display: lcp.clone(),
                    replacement: lcp,
                }],
            ));
        }

        // LCP == word (nothing further to complete)
        // Check if previous Tab already completed to this exact point
        // We check both the pre-insertion line AND the post-insertion line
        // because rustyline may pass either depending on timing
        let prev_tab_reached_here =
            last == line_after_lcp || last == format!("{}{}", &line[..start], word);

        if prev_tab_reached_here {
            // Second Tab at same position — show list
            println!();

            let col_width = 20;
            let cols = 6;
            for (i, name) in names.iter().enumerate() {
                print!("{:<width$}", name, width = col_width);
                if (i + 1) % cols == 0 {
                    println!();
                }
            }
            // Make sure last row ends with newline even if not full
            if names.len() % cols != 0 {
                println!();
            }

            print!("$ {}", line);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            *self.last_completed.borrow_mut() = String::new();
        } else {
            // First Tab at this position — bell
            print!("\x07");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            *self.last_completed.borrow_mut() = line_after_lcp;
        }

        Ok((start, vec![]))
    }
}