use std::cell::RefCell;
use std::{env, fs};

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

use crate::utils::{get_file_matches, longest_common_prefix};

pub struct ShellCompleter {
    pub last_line: RefCell<String>
}

impl Helper for ShellCompleter {}

impl Hinter for ShellCompleter {
    type Hint = String;
}

impl Highlighter for ShellCompleter {}

impl Validator for ShellCompleter {}
impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete (
        &self, 
        line: &str, 
        pos: usize,
        _: &Context<'_>
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mut commands = vec!["echo".to_string(), "exit".to_string(), "type".to_string(), "pwd".to_string(), "cd".to_string()];

        if let Ok(path_var) = env::var("PATH") {
            for path in env::split_paths(&path_var) {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let file_name = entry.file_name();

                        if let Some(name) = file_name.to_str() {
                            commands.push(name.to_string());
                        }
                    }
                }
            }
        };

        commands.sort();
        commands.dedup();

        let start = line[..pos].rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word = &line[start..pos];

        let is_command = !line[..start].contains(' ');
        let candidates: Vec<String> = if is_command {
            commands.into_iter().filter(|cmd| cmd.starts_with(word)).collect()
        }else {
            get_file_matches(word)
        };
        
        let matches: Vec<Pair> = candidates.iter()
        .map(|candidate| Pair {
            display: candidate.to_string(),
            replacement: format!("{} ", candidate.to_string())
        })
        .collect();

        let names: Vec<String> = matches.iter().map(|m| m.display.clone()).collect();
        
        let lcp = longest_common_prefix(&names);

        if matches.len() > 1 && lcp.len() > word.len() {
            *self.last_line.borrow_mut() =
                format!("{}{}", &line[..start], lcp);

            return Ok((
                start,
                vec![Pair {
                    display: lcp.clone(),
                    replacement: lcp,
                }],
            ));
        }

        if matches.len() > 1 {
            let mut last_line = self.last_line.borrow_mut();

            if *last_line != line  {

        
                print!("\x07");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                *last_line = line.to_string();

                return Ok((start, vec![]));
            }

            println!();
            
            let mut name: Vec<String> = matches.iter().map(|m| m.display.clone()).collect();
            
            name.sort();
            name.dedup();
            

            let col_width = 20;
            let cols = 6;

            for (i, name) in name.iter().enumerate() {

                print!("{:<width$}", name, width = col_width);

                if (i + 1) % cols == 0 {
                    println!();
                }
            }

            println!();
            print!("$ {}", line);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            *last_line = String::new();

            return Ok((start, vec![]));
        }

        *self.last_line.borrow_mut() = String::new();
        
        Ok((start, matches))
    }
}