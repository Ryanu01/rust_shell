use pathsearch::find_executable_in_path;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::{env, path::{PathBuf}, process::{self, Command}};

const BUILTINS: [&str; 5] = ["echo", "exit", "type", "pwd", "cd"];

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
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.is_empty() {
        return;
    }

    let command = parts[0];

    match command {
        "exit" => cmd_exit(),
        "echo" => cmd_echo(parts[1..].to_vec()),
        "type" => cmd_type(parts[1..].to_vec()),
        "pwd" => cmd_pwd(),
        "cd" => cmd_cd(parts[1..].to_vec()),
        _ => run_external_cmd(parts),
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

fn cmd_pwd() {
    println!("{}", env::current_dir().unwrap().display());
}

fn run_external_cmd(parts: Vec<&str>) {
    let command = parts[0];

    if find_executable_in_path(command).is_some() {
        Command::new(command)
        .args(&parts[1..])
        .status()
        .unwrap();
    }else {
        println!("{}: command not found", command);
    }
}
fn cmd_cd(args: Vec<&str>) {
    let target = if args.is_empty() {
        "~"
    } else {
        args[0]
    };

    let path: PathBuf = if target == "~" {
        PathBuf::from(env::var("HOME").unwrap())
    } else {
        PathBuf::from(target)
    };

    if path.is_dir() {
        env::set_current_dir(&path).unwrap();
    } else {
        println!("cd: {}: No such file or directory", target);
    }
}