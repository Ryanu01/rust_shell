# Rush 🦀 >_

A POSIX-like shell implementation written in Rust.

## Features

### Builtin Commands

| Command      | Description |
|--------------|-------------|
| `echo`       | Print arguments to stdout. Supports `>`, `>>`, `1>`, `1>>`, `2>`, `2>>` redirection. |
| `exit`       | Exit the shell, saving history to `HISTFILE` if set. |
| `type`       | Display whether a command is a shell builtin or an external executable. |
| `pwd`        | Print the current working directory. |
| `cd`         | Change directory. Supports `~` for home directory. |
| `declare`    | Set or display shell variables (`declare Var=value`, `declare -p Var`). |
| `jobs`       | List background jobs with their status (Running/Done). |
| `history`    | View, read (`-r`), append (`-a`), or write (`-w`) command history. |
| `complete`   | Register (`-C`), query (`-p`), or remove (`-r`) tab-completion specifications. |

### External Program Execution

- Runs any executable found in `PATH`
- Supports stdin/stdout/stderr redirection (`>`, `>>`, `2>`, `2>>`)
- Background execution with `&`

### Pipelines

- Connect multiple commands via `|` (pipe)
- Pure-external pipelines run concurrently with threads copying pipe data
- Pipelines involving builtins run sequentially

### Job Control

- Background jobs (`command &`) with job IDs
- Automatic reaping of completed background jobs
- `jobs` builtin to list all tracked jobs

### Tab Completion

- Builtin command names and `PATH` executables for the first word
- File path completion for subsequent words
- Custom completion specs via `complete -C`
- Multi-column display on repeated Tab press

### TUI Mode

A full-screen terminal UI powered by `ratatui` and `crossterm`. Launch automatically (unless `--no-tui` is passed):

```
rush              # starts in TUI mode
rush --no-tui     # falls back to classic rustyline REPL
```

| Key | Action |
|-----|--------|
| `Enter` | Execute command |
| `Tab` | Trigger / cycle completions |
| `Up` / `Down` | Navigate history |
| `Ctrl+D` | Exit (on empty input) |
| `Ctrl+C` | Cancel current input |
| `Ctrl+U` | Clear input line |
| `Ctrl+L` | Clear output pane |
| `Ctrl+Shift+C` | Copy output to clipboard |
| `Ctrl+Shift+V` | Paste from clipboard |
| `PageUp` / `PageDown` | Scroll output up / down |
| `Mouse wheel` | Scroll output |
| `F2` | Toggle mouse capture (for text selection) |

- Colored `$` prompt, output pane, and completion popup
- Directory entries colored red, executables green with `*` suffix
- Status bar showing `cwd`, background job count, and keybindings
- Built-in `ls` command with colorized output

### Variable Expansion

- `$NAME` and `${NAME}` syntax
- Variables stored via `declare Var=value`
- Unset variables expand to an empty string and the empty word is removed from arguments
- Inline expansion within words (e.g., `pre_${VAR}_suffix`)
- Regex-based replacement using the `regex` crate

## Architecture

```
main.rs
├── main()                    ─ Entry: TUI (ratatui) or rustyline REPL
│   ├── tui::App::run()       ─ Full-screen TUI (feature = "tui")
│   │   ├── draw()            ─ Render status bar, output pane, input bar
│   │   ├── handle_events()   ─ Key / mouse / resize dispatch
│   │   └── execute_cmd()     ─ Parse & execute within TUI
│   └── (rustyline fallback)  ─ Classic REPL with "$ " prompt
│       ├── readline()        ─ Read input line
│       └── read_input()      ─ Parse and dispatch command
│           ├── shell_words::split    ─ Tokenize (respects quotes)
│           ├── expand_vars()         ─ Replace $VAR / ${VAR}
│           ├── Filter empty strings  ─ Remove empty words
│           ├── Pipeline detection    ─ Split on "|"
│           ├── Builtin dispatch      ─ Match command name
│           └── run_external_cmd()    ─ Spawn child process
│
├── expand_vars()             ─ Variable expansion logic
├── cmd_declare()             ─ declare builtin
├── cmd_echo()                ─ echo builtin
├── cmd_type()                ─ type builtin
├── cmd_pwd()                 ─ pwd builtin
├── cmd_cd()                  ─ cd builtin
├── cmd_history()             ─ history builtin
├── cmd_complete()            ─ complete builtin
├── cmd_jobs()                ─ jobs builtin
├── cmd_exit()                ─ exit builtin
├── run_external_cmd()        ─ External command execution
├── run_pipeline()            ─ Pipeline orchestrator
│   ├── run_concurrent_external_pipeline()  ─ Multi-threaded pipe
│   └── run_sequential_pipeline()           ─ Sequential (builtins)
└── reap_jobs()               ─ Background job reaper

tui/                          ─ TUI mode modules (feature = "tui")
├── app.rs                    ─ App state, run loop, job reaping
├── ui.rs                     ─ Ratatui rendering (status, output, input)
├── input.rs                  ─ Keyboard/mouse input, completions, clipboard
└── exec.rs                   ─ Command execution within TUI

helper.rs                     ─ rustyline Completer implementation
utils.rs                      ─ File path completion utilities
```

### Global State

All shell state is stored in `LazyLock<Mutex<...>>` statics (shared between TUI and REPL modes):

- `STORE` — `HashMap<String, String>` for shell variables (`declare`)
- `COMPLETION_SPEC` — `HashMap<String, String>` mapping command names to completer executables
- `JOBS` — `Vec<Job>` tracking background child processes

### Expansion Flow

1. Raw input line is split into words by `shell_words::split` (handles quoting)
2. Each word is scanned with the regex `\$\{[A-Za-z_][A-Za-z0-9_]*\}|\$[A-Za-z_][A-Za-z0-9_]*`
3. Matches are replaced with values from `STORE` (or empty string if unset)
4. Words that become empty after expansion are removed from the argument list
5. The resulting word list is passed to builtins or external commands

## Dependencies

| Crate | Purpose |
|-------|---------|
| `rustyline` | Readline/REPL with history and tab completion |
| `shell-words` | POSIX shell word splitting (quote-aware) |
| `pathsearch` | `PATH` lookup for executables |
| `regex` | Variable name pattern matching in expansion |
| `anyhow` / `thiserror` | Error handling |
| `bytes` | Buffer management |
| `ratatui` / `crossterm` | TUI rendering and terminal I/O (feature `tui`) |
| `arboard` | Clipboard access in TUI mode (feature `tui`) |

## Installation

### Pre-built binary (via GitHub Releases)

**Linux (x86_64 / ARM64)**
```sh
curl -LO https://github.com/Ryanu01/rush/releases/latest/download/rush.gz
gunzip rush.gz && chmod +x rush && sudo mv rush /usr/local/bin/
```

**macOS (Intel / Apple Silicon)**
```sh
curl -LO https://github.com/Ryanu01/rush/releases/latest/download/rush.gz
gunzip rush.gz && chmod +x rush && sudo mv rush /usr/local/bin/
```

**Windows (x86_64)**
```powershell
curl -LO https://github.com/Ryanu01/rush/releases/latest/download/rush.exe.gz
gunzip rush.exe.gz
.\rush.exe
```

### From source

Requires Rust 1.75+.

```sh
cargo install --git https://github.com/Ryanu01/rush
```

### Build locally

```sh
cargo build --release                     # includes TUI via default features
cargo build --release --no-default-features  # REPL-only, no TUI
./target/release/rush
```
## Want to know more

`Read the code`