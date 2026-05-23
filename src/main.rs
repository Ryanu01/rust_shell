mod helper;
mod utils;
use helper::ShellCompleter;

use rustyline::history::History;
use rustyline::{CompletionType, Config, EditMode, Editor};
use rustyline::{error::ReadlineError, history::DefaultHistory};

use pathsearch::find_executable_in_path;
use std::cell::RefCell;
use std::collections::HashMap;
#[allow(unused_imports)]
use std::io::{self, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout};
use std::sync::{LazyLock, Mutex};
use std::thread;
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

const BUILTINS: [&str; 8] = [
    "echo", "exit", "type", "pwd", "cd", "complete", "jobs", "history",
];

pub(crate) static COMPLETION_SPEC: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) static JOBS: LazyLock<Mutex<Vec<Job>>> = LazyLock::new(|| Mutex::new(Vec::new()));
fn main() {
    let config = Config::builder()
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();

    let mut last_appended: usize = 0;
    let mut rl = Editor::<ShellCompleter, DefaultHistory>::with_config(config).unwrap();
    rl.set_helper(Some(ShellCompleter {
        last_completed: RefCell::new(String::new()),
    }));

    let hist_file = env::var("HISTFILE").ok();

    if let Some(ref path) = hist_file {
        let _ = rl.load_history(path);
    }

    loop {
        reap_jobs(false, &mut io::stdout().lock());
        match rl.readline("$ ") {
            Ok(line) => {
                rl.add_history_entry(line.as_str()).unwrap();
                read_input(line.trim(), &mut rl, &mut last_appended, &hist_file);
            }
            Err(ReadlineError::Interrupted) => {
                if let Some(ref path) = hist_file {
                    let _ = rl.save_history(path);
                }
                break;
            }
            Err(ReadlineError::Eof) => {
                if let Some(ref path) = hist_file {
                    let _ = rl.save_history(path);
                }
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);

                if let Some(ref path) = hist_file {
                    let _ = rl.save_history(path);
                }
                break;
            }
        }
    }
}

fn read_input(
    cmd: &str,
    rl: &mut Editor<ShellCompleter, DefaultHistory>,
    last_appended: &mut usize,
    hist_file: &Option<String>,
) {
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

    let mut pipe_positions = Vec::new();
    for (i, &s) in slices.iter().enumerate() {
        if s == "|" {
            pipe_positions.push(i);
        }
    }

    if !pipe_positions.is_empty() {
        let mut segments: Vec<Vec<&str>> = Vec::new();
        let mut start = 0;
        for &pos in &pipe_positions {
            segments.push(slices[start..pos].to_vec());
            start = pos + 1;
        }
        segments.push(slices[start..].to_vec());
        run_pipeline(segments);
        return;
    }

    let command = slices[0];

    match command {
        "exit" => cmd_exit(rl, hist_file),
        "echo" => cmd_echo(slices[1..].to_vec(), &mut io::stdout().lock()),
        "type" => cmd_type(slices[1..].to_vec(), &mut io::stdout().lock()),
        "pwd" => cmd_pwd(&mut io::stdout().lock()),
        "cd" => cmd_cd(slices[1..].to_vec(), &mut io::stdout().lock()),
        "complete" => cmd_complete(slices[1..].to_vec(), &mut io::stdout().lock()),
        "jobs" => cmd_jobs(slices[1..].to_vec(), &mut io::stdout().lock()),
        "history" => cmd_history(
            slices[1..].to_vec(),
            rl,
            &mut io::stdout().lock(),
            last_appended,
        ),
        _ => run_external_cmd(slices, background),
    }
}

fn cmd_history(
    args: Vec<&str>,
    rl: &mut Editor<ShellCompleter, DefaultHistory>,
    writer: &mut dyn Write,
    last_appended: &mut usize,
) {
    let history = rl.history();
    let total = history.len();
    if !args.is_empty() && args[0] == "-r" {
        let path = args[1];

        rl.load_history(path).unwrap();
    } else if !args.is_empty() && args[0] == "-a" {
        let path = args[1];

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();

        for entry in rl.history().iter().skip(*last_appended) {
            writeln!(file, "{}", entry).unwrap();
        }

        *last_appended = rl.history().len();
    } else if !args.is_empty() && args[0] == "-w" {
        let path = args[1];
        let mut contents = String::new();
        for records in rl.history().iter() {
            contents.push_str(records);
            contents.push('\n');
        }

        fs::write(path, contents).unwrap();
    } else if !args.is_empty() {
        let history_limit: usize = args[0].parse().unwrap();

        let start = total.saturating_sub(history_limit);

        for (i, record) in rl.history().iter().enumerate().skip(start) {
            writeln!(writer, " {} {}", i + 1, record).unwrap();
        }
    } else {
        for (i, record) in rl.history().iter().enumerate() {
            writeln!(writer, " {} {}", i + 1, record).unwrap();
        }
    }
}

fn cmd_exit(rl: &mut Editor<ShellCompleter, DefaultHistory>, hist_file: &Option<String>) {
    if let Some(path) = hist_file {
        if let Ok(mut file) = fs::File::create(path) {
            for entry in rl.history().iter() {
                writeln!(file, "{}", entry).unwrap();
            }
        }
    }
    process::exit(0);
}
fn reap_jobs(print_running: bool, writer: &mut dyn Write) {
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
            writeln!(
                writer,
                "[{}]{}  {:<24}{}",
                jobs[it].id, symbol, status, jobs[it].command
            )
            .unwrap();
        }

        if status == "Done" {
            jobs.remove(it);
        } else {
            it += 1;
        }
    }
}
#[allow(unused_variables)]
fn cmd_jobs(_args: Vec<&str>, writer: &mut dyn Write) {
    reap_jobs(true, writer);
}
fn cmd_complete(args: Vec<&str>, writer: &mut dyn Write) {
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
            writeln!(writer, "{}", stdout).unwrap();
        } else {
            writeln!(
                writer,
                "complete: {}: no completion specification",
                cmd_name
            )
            .unwrap();
        }
    }
}

fn cmd_echo(args: Vec<&str>, writer: &mut dyn Write) {
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
                writeln!(writer, "{}", text).unwrap();
            }
        }
    } else {
        match stdout_redirect {
            Some(file) => {
                fs::write(file, format!("{}\n", text)).unwrap();
            }

            None => {
                writeln!(writer, "{}", text).unwrap();
            }
        }
    }
}

fn cmd_type(args: Vec<&str>, writer: &mut dyn Write) {
    if args.is_empty() {
        return;
    }

    let command = args[0];
    if is_builtin(command) {
        writeln!(writer, "{} is a shell builtin", command).unwrap();
    } else if let Some(path) = find_executable_in_path(command) {
        writeln!(writer, "{} is {}", command, path.display()).unwrap();
    } else {
        writeln!(writer, "{}: not found", command).unwrap();
    }
}

fn is_builtin(cmd: &str) -> bool {
    BUILTINS.contains(&cmd)
}

fn execute_builtin(cmd: &str, args: Vec<&str>, writer: &mut dyn Write) {
    match cmd {
        "echo" => cmd_echo(args, writer),
        "type" => cmd_type(args, writer),
        "pwd" => cmd_pwd(writer),
        "cd" => cmd_cd(args, writer),
        "complete" => cmd_complete(args, writer),
        "jobs" => cmd_jobs(args, writer),
        _ => (),
    }
}

fn run_pipeline(segments: Vec<Vec<&str>>) {
    for segment in &segments {
        if !is_builtin(segment[0]) && find_executable_in_path(segment[0]).is_none() {
            writeln!(
                &mut io::stdout().lock(),
                "{}: command not found",
                segment[0]
            )
            .unwrap();
            return;
        }
    }

    let has_builtins = segments.iter().any(|s| is_builtin(s[0]));
    if has_builtins {
        run_sequential_pipeline(segments);
    } else {
        run_concurrent_external_pipeline(segments);
    }
}

fn run_concurrent_external_pipeline(segments: Vec<Vec<&str>>) {
    let n = segments.len();
    let mut child_stdins: Vec<Option<ChildStdin>> = Vec::new();
    let mut child_stdouts: Vec<Option<ChildStdout>> = Vec::new();
    let mut children: Vec<Child> = Vec::new();

    for (i, seg) in segments.iter().enumerate() {
        let mut cmd = Command::new(seg[0]);
        cmd.args(&seg[1..]);

        if i > 0 {
            cmd.stdin(Stdio::piped());
        }
        if i < n - 1 {
            cmd.stdout(Stdio::piped());
        }

        let mut child = cmd.spawn().unwrap();
        child_stdins.push(if i > 0 { child.stdin.take() } else { None });
        child_stdouts.push(if i < n - 1 { child.stdout.take() } else { None });
        children.push(child);
    }

    let mut threads = Vec::new();
    for i in 0..n - 1 {
        let mut prev_stdout = child_stdouts[i].take().unwrap();
        let mut this_stdin = child_stdins[i + 1].take().unwrap();
        threads.push(thread::spawn(move || {
            let _ = io::copy(&mut prev_stdout, &mut this_stdin);
        }));
    }

    if let Some(last) = children.last_mut() {
        last.wait().unwrap();
    }

    for t in threads {
        let _ = t.join();
    }

    for child in &mut children {
        let _ = child.wait();
    }
}

fn run_sequential_pipeline(segments: Vec<Vec<&str>>) {
    let n = segments.len();
    let mut prev_output: Option<Vec<u8>> = None;

    for (i, segment) in segments.iter().enumerate() {
        let is_last = i == n - 1;
        let cmd = segment[0];
        let args = segment[1..].to_vec();

        if is_builtin(cmd) {
            if is_last {
                execute_builtin(cmd, args, &mut io::stdout().lock());
            } else {
                let mut buf: Vec<u8> = Vec::new();
                execute_builtin(cmd, args, &mut buf);
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

            let mut child = command.spawn().unwrap_or_else(|_| {
                panic!("{}: command not found", cmd);
            });

            if let Some(input) = prev_output.take() {
                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(&input).unwrap();
                }
            }

            if is_last {
                child.wait().unwrap();
            } else {
                let output = child.wait_with_output().unwrap();
                prev_output = Some(output.stdout);
            }
        }
    }
}

fn cmd_pwd(writer: &mut dyn Write) {
    writeln!(writer, "{}", env::current_dir().unwrap().display()).unwrap();
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
        let child = cmd.spawn().unwrap();
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

fn cmd_cd(args: Vec<&str>, writer: &mut dyn Write) {
    let target = if args.is_empty() { "~" } else { args[0] };

    let path: PathBuf = if target == "~" {
        PathBuf::from(env::var("HOME").unwrap())
    } else {
        PathBuf::from(target)
    };

    if path.is_dir() {
        env::set_current_dir(&path).unwrap();
    } else {
        writeln!(writer, "cd: {}: No such file or directory", target).unwrap();
    }
}