use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

pub struct ShellCompleter;

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
        let commands = ["echo", "exit", "type", "pwd", "cd"];

        let matches = commands
        .iter()
        .filter(|cmd| cmd.starts_with(&line[..pos]))
        .map(|cmd| Pair {
            display: cmd.to_string(),
            replacement: format!("{} ", cmd.to_string())
        })
        .collect();
        
        Ok((0, matches))
    }
}