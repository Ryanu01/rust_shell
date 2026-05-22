mod helper;
mod utils;
use helper::ShellCompleter;

use rustyline::{CompletionType, Config, EditMode, Editor};
use rustyline::{error::ReadlineError, history::DefaultHistory};

use pathsearch::find_executable_in_path;
use std::cell::RefCell;
use std::collections::HashMap;
#[allow(unused_imports)]
use std::io::{self, Write};
use std::process::Child;
use std::sync::{LazyLock, Mutex};
use std::{
    env,
    fs::{self, File, OpenOptions},
    path::PathBuf,
    process::{self, Command, Stdio},
};

struct Job {
    id: usize,
    child: Child,
    command: String,
}

const BUILTINS: [&str; 7] = ["echo", "exit", "type", "pwd", "cd", "complete", "jobs"];

pub(crate) static COMPLETION_SPEC: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) static JOBS: LazyLock<Mutex<Vec<Job>>> = LazyLock::new(|| Mutex::new(Vec::new()));
fn main() {
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();

    let mut rl = Editor::<ShellCompleter, DefaultHistory>::with_config(config).unwrap();

    rl.set_helper(Some(ShellCompleter {
        last_completed: RefCell::new(String::new()),
    }));

    loop {
        reap_jobs(false);
        match rl.readline("$ ") {
            Ok(line) => {
                read_input(line.trim());
            }
            Err(ReadlineError::Interrupted) => break,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}

fn read_input(cmd: &str) {
    let parts = shell_words::split(cmd).unwrap();

    let slices: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
    if slices.is_empty() {
        return;
    }

    let mut background = false;
    let mut slices = slices;

    if slices.last() == Some(&"&") {
        background = true;
        slices.pop();
    }

    let command = slices[0];

    match command {
        "exit" => cmd_exit(),
        "echo" => cmd_echo(slices[1..].to_vec()),
        "type" => cmd_type(slices[1..].to_vec()),
        "pwd" => cmd_pwd(),
        "cd" => cmd_cd(slices[1..].to_vec()),
        "complete" => cmd_complete(slices[1..].to_vec()),
        "jobs" => cmd_jobs(slices[1..].to_vec()),
        _ => run_external_cmd(slices, background),
    }
}

fn cmd_exit() {
    process::exit(0);
}
#[allow(unused_variables)]
fn cmd_jobs(_args: Vec<&str>) {
    reap_jobs(true);
}
fn reap_jobs(print_running: bool) {
    let mut jobs = JOBS.lock().unwrap();

    let mut it = 0;

    while it < jobs.len() {
        let status = match jobs[it].child.try_wait() {
            Ok(Some(_)) => "Done",
            Ok(None) => "Running",
            Err(_) => "Error",
        };

        let symbol = if it == jobs.len() - 1 {
            "+"
        } else if it == jobs.len() - 2 {
            "-"
        } else {
            " "
        };

        // automatic reaping only prints Done jobs
        // jobs builtin prints everything
        if status == "Done" || print_running {
            println!(
                "[{}]{}  {:<24}{}",
                jobs[it].id, symbol, status, jobs[it].command
            );
        }

        if status == "Done" {
            jobs.remove(it);
        } else {
            it += 1;
        }
    }
}
fn cmd_complete(args: Vec<&str>) {
    let mut map = COMPLETION_SPEC.lock().unwrap();
    let mut i = 0;
    let mut cmd = None;
    let mut delete_cmd = None;
    while i < args.len() {
        match args[i] {
            "-p" => {
                if i + 1 < args.len() {
                    cmd = Some(args[i + 1])
                }
                i += 2;
                continue;
            }

            "-C" => {
                if i + 2 < args.len() {
                    if let (Some(cmd_path), Some(cmd)) = (args.get(i + 1), args.get(i + 2)) {
                        map.insert(cmd.to_string(), cmd_path.to_string());
                    }
                }
                i += 3;
                continue;
            }
            "-r" => {
                if i + 1 < args.len() {
                    delete_cmd = Some(args[i + 1]);
                }
                i += 2;
                continue;
            }
            _ => (),
        }

        i += 1;
    }

    if let Some(cmd_name) = delete_cmd {
        if let Some(_cmd_path) = map.get(cmd_name) {
            map.remove(cmd_name);
        }
    }

    if let Some(cmd_name) = cmd {
        if let Some(cmd_path) = map.get(cmd_name) {
            let stdout = format!("complete -C '{cmd_path}' {}", cmd_name);
            println!("{}", stdout);
        } else {
            println!("complete: {}: no completion specification", cmd_name);
        }
    }
}

fn cmd_echo(args: Vec<&str>) {
    let mut output = Vec::new();
    let mut stdout_redirect = None;
    let mut stderr_redirect = None;
    let mut append = false;

    let mut i = 0;

    while i < args.len() {
        match args[i] {
            ">" | "1>" => {
                if i + 1 < args.len() {
                    stdout_redirect = Some(args[i + 1]);
                }
                break;
            }

            "2>" | "2>>" => {
                if i + 1 < args.len() {
                    stderr_redirect = Some(args[i + 1]);
                }

                i += 2;
                continue;
            }

            ">>" | "1>>" => {
                if i + 1 < args.len() {
                    stdout_redirect = Some(args[i + 1]);
                    append = true;
                }
                break;
            }
            arg => output.push(arg),
        }

        i += 1;
    }

    let text = output.join(" ");

    if let Some(file) = stderr_redirect {
        fs::write(file, "").unwrap();
    }

    if append {
        match stdout_redirect {
            Some(file) => {
                let mut file_append = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file)
                    .unwrap();
                file_append
                    .write_all(format!("{}\n", text).as_bytes())
                    .unwrap();
            }

            None => {
                println!("{}", text);
            }
        }
    } else {
        match stdout_redirect {
            Some(file) => {
                fs::write(file, format!("{}\n", text)).unwrap();
            }

            None => {
                println!("{}", text);
            }
        }
    }
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

fn run_external_cmd(parts: Vec<&str>, background: bool) {
    let command = parts[0];
    let mut append = false;
    if find_executable_in_path(command).is_none() {
        println!("{}: command not found", command);
        return;
    }

    let mut args = Vec::new();
    let mut output_redirect: Option<&str> = None;
    let mut err_redirect: Option<&str> = None;
    let mut i = 1;
    while i < parts.len() {
        match parts[i] {
            ">" | "1>" => {
                if i + 1 < parts.len() {
                    output_redirect = Some(parts[i + 1]);
                }
                break;
            }

            "2>" => {
                if i + 1 < parts.len() {
                    err_redirect = Some(parts[i + 1])
                }

                i += 2;
                continue;
            }

            ">>" | "1>>" => {
                if i + 1 < parts.len() {
                    output_redirect = Some(parts[i + 1]);
                    append = true;
                }
                break;
            }

            "2>>" => {
                if i + 1 < parts.len() {
                    err_redirect = Some(parts[i + 1])
                }
                append = true;
                i += 2;
                continue;
            }
            arg => args.push(arg),
        }
        i += 1;
    }

    let mut cmd = Command::new(command);
    cmd.args(&args);

    /*
     * instead of printing output to terminal put it in some file
     */
    if let Some(file_name) = err_redirect {
        let file = if append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_name)
                .unwrap()
        } else {
            File::create(file_name).unwrap()
        };
        cmd.stderr(Stdio::from(file));
    }

    if let Some(file_name) = output_redirect {
        let file = if append {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_name)
                .unwrap()
        } else {
            File::create(file_name).unwrap()
        };

        cmd.stdout(Stdio::from(file));
    }

    if background {
        let mut child = cmd.spawn().unwrap();
        let pid = child.id();
        let mut jobs = JOBS.lock().unwrap();

        let job_id = jobs.len() + 1;

        jobs.push(Job {
            id: job_id,
            child,
            command: parts.join(" "),
        });

        println!("[{}] {}", job_id, pid);
    } else {
        cmd.status().unwrap();
    }
}

fn cmd_cd(args: Vec<&str>) {
    let target = if args.is_empty() { "~" } else { args[0] };

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