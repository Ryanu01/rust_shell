use super::{is_executable, App, OutputStyle};
use crate::{
    cmd_cd, cmd_complete, cmd_declare, cmd_echo, cmd_jobs, cmd_pwd, cmd_type, expand_vars,
    is_builtin, JOBS,
};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

impl App {
    pub(crate) fn execute_cmd(&mut self, cmd: &str) {
        let parts = match shell_words::split(cmd) {
            Ok(p) => p,
            Err(e) => {
                self.output_lines
                    .push((format!("rush: parse error: {}", e), OutputStyle::Plain));
                return;
            }
        };

        if parts.is_empty() {
            return;
        }

        let parts = expand_vars(parts);
        let parts: Vec<String> = parts.into_iter().filter(|s| !s.is_empty()).collect();
        let slices: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();

        if slices.is_empty() {
            return;
        }

        if slices.last() == Some(&"&") {
            let mut bg_slices = slices.clone();
            bg_slices.pop();
            self.run_external_bg(bg_slices, &parts.join(" "));
            return;
        }

        let mut pipe_positions = Vec::new();
        for (i, &s) in slices.iter().enumerate() {
            if s == "|" {
                pipe_positions.push(i);
            }
        }

        if !pipe_positions.is_empty() {
            self.run_pipeline_tui(&slices, &pipe_positions);
            return;
        }

        let command = slices[0];

        match command {
            "exit" => self.running = false,
            "clear" => {
                self.output_lines.clear();
                self.scroll_offset = 0;
            }
            "ls" => self.cmd_ls_tui(&slices[1..]),
            "history" => {
                for (i, entry) in self.history.iter().enumerate() {
                    self.output_lines
                        .push((format!(" {} {}", i + 1, entry), OutputStyle::Plain));
                }
            }
            "cd" => {
                let mut buf: Vec<u8> = Vec::new();
                cmd_cd(slices[1..].to_vec(), &mut buf);
                if !buf.is_empty() {
                    self.output_lines.push((
                        String::from_utf8_lossy(&buf).to_string(),
                        OutputStyle::Plain,
                    ));
                }
            }
            _ if is_builtin(command) => {
                let mut buf: Vec<u8> = Vec::new();
                match command {
                    "echo" => cmd_echo(slices[1..].to_vec(), &mut buf),
                    "type" => cmd_type(slices[1..].to_vec(), &mut buf),
                    "pwd" => cmd_pwd(&mut buf),
                    "complete" => cmd_complete(slices[1..].to_vec(), &mut buf),
                    "jobs" => cmd_jobs(slices[1..].to_vec(), &mut buf),
                    "declare" => cmd_declare(slices[1..].to_vec(), &mut buf),
                    _ => {}
                }
                if !buf.is_empty() {
                    self.output_lines.push((
                        String::from_utf8_lossy(&buf).to_string(),
                        OutputStyle::Plain,
                    ));
                }
            }
            _ => self.run_external_tui(&slices),
        }
    }

    fn cmd_ls_tui(&mut self, args: &[&str]) {
        let path = args
            .iter()
            .find(|a| !a.starts_with('-'))
            .map(|s| s.to_string())
            .unwrap_or_else(|| ".".to_string());

        let entries = match fs::read_dir(&path) {
            Ok(e) => e,
            Err(e) => {
                self.output_lines.push((
                    format!("ls: cannot access '{}': {}", path, e),
                    OutputStyle::Plain,
                ));
                return;
            }
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();
        let mut executables = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            match entry.file_type() {
                Ok(t) if t.is_dir() => dirs.push(name),
                Ok(t) if t.is_symlink() => {
                    if let Ok(meta) = entry.metadata() {
                        if meta.is_dir() {
                            dirs.push(name);
                        } else {
                            files.push(name);
                        }
                    } else {
                        files.push(name);
                    }
                }
                _ => {
                    if is_executable(&entry.path()) {
                        executables.push(name);
                    } else {
                        files.push(name);
                    }
                }
            }
        }

        dirs.sort();
        files.sort();
        executables.sort();

        for d in &dirs {
            self.output_lines
                .push((format!("{}/", d), OutputStyle::Directory));
        }
        for f in &files {
            self.output_lines.push((f.clone(), OutputStyle::Plain));
        }
        for e in &executables {
            self.output_lines
                .push((format!("{}*", e), OutputStyle::Executable));
        }
    }

    fn run_external_tui(&mut self, parts: &[&str]) {
        let command = parts[0];
        let args = &parts[1..];

        let output = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(out) => {
                if !out.stdout.is_empty() {
                    let text = String::from_utf8_lossy(&out.stdout).to_string();
                    for line in text.lines() {
                        self.output_lines
                            .push((line.to_string(), OutputStyle::Plain));
                    }
                }
                if !out.stderr.is_empty() {
                    let text = String::from_utf8_lossy(&out.stderr).to_string();
                    for line in text.lines() {
                        self.output_lines
                            .push((line.to_string(), OutputStyle::Plain));
                    }
                }
                if !out.status.success() && out.stdout.is_empty() && out.stderr.is_empty() {
                    self.output_lines.push((
                        format!("{}: command not found", command),
                        OutputStyle::Plain,
                    ));
                }
            }
            Err(e) => {
                self.output_lines
                    .push((format!("{}: {}", command, e), OutputStyle::Plain));
            }
        }
    }

    fn run_external_bg(&mut self, parts: Vec<&str>, full_cmd: &str) {
        let command = parts[0];
        let args = &parts[1..];

        match Command::new(command)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                let pid = child.id();
                let mut jobs = JOBS.lock().unwrap();
                let job_id = jobs.len() + 1;
                jobs.push(crate::Job {
                    id: job_id,
                    child,
                    command: full_cmd.to_string(),
                });
                self.output_lines
                    .push((format!("[{}] {}", job_id, pid), OutputStyle::Plain));
            }
            Err(e) => {
                self.output_lines
                    .push((format!("{}: {}", command, e), OutputStyle::Plain));
            }
        }
    }

    fn run_pipeline_tui(&mut self, slices: &[&str], pipe_positions: &[usize]) {
        let mut segments: Vec<Vec<&str>> = Vec::new();
        let mut start = 0;
        for &pos in pipe_positions {
            segments.push(slices[start..pos].to_vec());
            start = pos + 1;
        }
        segments.push(slices[start..].to_vec());

        for segment in &segments {
            if !is_builtin(segment[0]) {
                if Command::new(segment[0])
                    .arg("--invalid-flag-to-test-existence")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .is_err()
                {
                    self.output_lines.push((
                        format!("{}: command not found", segment[0]),
                        OutputStyle::Plain,
                    ));
                    return;
                }
            }
        }

        let has_builtins = segments.iter().any(|s| is_builtin(s[0]));
        if has_builtins {
            self.run_seq_pipeline_tui(&segments);
        } else {
            self.run_conc_pipeline_tui(&segments);
        }
    }

    fn run_conc_pipeline_tui(&mut self, segments: &[Vec<&str>]) {
        let n = segments.len();
        let mut children = Vec::new();
        let mut prev_stdout: Option<std::process::ChildStdout> = None;

        for (i, seg) in segments.iter().enumerate() {
            let mut cmd = Command::new(seg[0]);
            cmd.args(&seg[1..]);
            if prev_stdout.is_some() {
                cmd.stdin(Stdio::piped());
            }
            cmd.stdout(Stdio::piped());
            match cmd.spawn() {
                Ok(mut child) => {
                    if let Some(mut input) = prev_stdout.take() {
                        if let Some(mut stdin) = child.stdin.take() {
                            std::thread::spawn(move || {
                                let _ = std::io::copy(&mut input, &mut stdin);
                            });
                        }
                    }
                    if i < n - 1 {
                        prev_stdout = child.stdout.take();
                    }
                    children.push(child);
                }
                Err(e) => {
                    self.output_lines
                        .push((format!("{}: {}", seg[0], e), OutputStyle::Plain));
                    return;
                }
            }
        }

        if let Some(last) = children.pop() {
            let output = last.wait_with_output().ok();
            for child in &mut children {
                let _ = child.wait();
            }
            if let Some(out) = output {
                if !out.stdout.is_empty() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for line in text.lines() {
                        self.output_lines
                            .push((line.to_string(), OutputStyle::Plain));
                    }
                }
                if !out.stderr.is_empty() {
                    let text = String::from_utf8_lossy(&out.stderr);
                    for line in text.lines() {
                        self.output_lines
                            .push((line.to_string(), OutputStyle::Plain));
                    }
                }
            }
        } else {
            for child in &mut children {
                let _ = child.wait();
            }
        }
    }

    fn run_seq_pipeline_tui(&mut self, segments: &[Vec<&str>]) {
        let n = segments.len();
        let mut prev_output: Option<Vec<u8>> = None;

        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == n - 1;
            let cmd = segment[0];
            let args = segment[1..].to_vec();

            if is_builtin(cmd) {
                let mut buf: Vec<u8> = Vec::new();
                let w: &mut dyn Write = &mut buf;
                match cmd {
                    "echo" => cmd_echo(args, w),
                    "type" => cmd_type(args, w),
                    "pwd" => cmd_pwd(w),
                    "cd" => cmd_cd(args, w),
                    "complete" => cmd_complete(args, w),
                    "jobs" => cmd_jobs(args, w),
                    "declare" => cmd_declare(args, w),
                    _ => {}
                }
                if is_last {
                    if !buf.is_empty() {
                        let text = String::from_utf8_lossy(&buf);
                        for line in text.lines() {
                            self.output_lines
                                .push((line.to_string(), OutputStyle::Plain));
                        }
                    }
                } else {
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

                match command.spawn() {
                    Ok(mut child) => {
                        if let Some(input) = prev_output.take() {
                            if let Some(mut stdin) = child.stdin.take() {
                                let _ = stdin.write_all(&input);
                            }
                        }

                        if is_last {
                            let output = child.wait_with_output().ok();
                            if let Some(out) = output {
                                if !out.stdout.is_empty() {
                                    let text = String::from_utf8_lossy(&out.stdout);
                                    for line in text.lines() {
                                        self.output_lines
                                            .push((line.to_string(), OutputStyle::Plain));
                                    }
                                }
                                if !out.stderr.is_empty() {
                                    let text = String::from_utf8_lossy(&out.stderr);
                                    for line in text.lines() {
                                        self.output_lines
                                            .push((line.to_string(), OutputStyle::Plain));
                                    }
                                }
                            }
                        } else {
                            let output = child.wait_with_output().ok();
                            prev_output = output.map(|o| o.stdout);
                        }
                    }
                    Err(e) => {
                        self.output_lines
                            .push((format!("{}: {}", cmd, e), OutputStyle::Plain));
                        return;
                    }
                }
            }
        }
    }
}
