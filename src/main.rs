use pathsearch::find_executable_in_path;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::process;

const BUILTINS: [&str; 3] = ["echo", "exit", "type"];

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();

        read_input(command.trim());
    }
}

fn read_input(cmd: &str) {
    let mut parts = cmd.split_whitespace();

    let command = match parts.next() {
        Some(c) => c,
        None => return,
    };

    match command {
        "exit" => cmd_exit(),
        "echo" => cmd_echo(parts.collect()),
        "type" => cmd_type(parts.collect()),
        _ => println!("{}: command not found", cmd),
    }
}

fn cmd_exit() {
    process::exit(0);
}

fn cmd_echo(args: Vec<&str>) {
    println!("{}", args.join(" "));
}

fn cmd_type(args: Vec<&str>) {
    if args.is_empty() {
        return;
    }

    let command = args[0];
    if is_builtin(command) {
        println!("{} is a shell builtin", command);
    } else if let Some(path) = find_executable_in_path(command) {
        println!("{} is {}", command, path.display());
    } else {
        println!("{}: not found", command);
    }
}

fn is_builtin(cmd: &str) -> bool {
    BUILTINS.contains(&cmd)
}