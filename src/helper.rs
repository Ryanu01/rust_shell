use std::cell::RefCell;
use std::{env, fs};

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

pub struct ShellCompleter {
    pub last_tab: RefCell<bool>
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
        let matches: Vec<Pair> = commands
        .iter()
        .filter(|cmd| cmd.starts_with(word))
        .map(|cmd| Pair {
            display: cmd.to_string(),
            replacement: format!("{} ", cmd.to_string())
        })
        .collect();

    

        if matches.len() > 1 {
            let mut last_tab = self.last_tab.borrow_mut();

            if !* last_tab {

        
                print!("\x07");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                *last_tab = true;

                return Ok((start, vec![]));
            }

            
            let mut name: Vec<String> = matches.iter().map(|m| m.display.clone()).collect();
            
            name.sort();
            name.dedup();
            
            println!();

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

            *last_tab = false;

            return Ok((start, vec![]));
        }

        
        Ok((start, matches))
    }
}