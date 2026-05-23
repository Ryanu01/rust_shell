
# Rush

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

### Variable Expansion

- `$NAME` and `${NAME}` syntax
- Variables stored via `declare Var=value`
- Unset variables expand to an empty string and the empty word is removed from arguments
- Inline expansion within words (e.g., `pre_${VAR}_suffix`)
- Regex-based replacement using the `regex` crate

## Architecture

```
main.rs
├── main()                    ─ REPL loop using rustyline
│   ├── readline()            ─ Read input with prompt "$ "
│   └── read_input()          ─ Parse and dispatch command
│       ├── shell_words::split    ─ Tokenize (respects quotes)
│       ├── expand_vars()         ─ Replace $VAR / ${VAR} with values
│       ├── Filter empty strings  ─ Remove words that became empty
│       ├── Pipeline detection    ─ Split on "|"
│       ├── Builtin dispatch      ─ Match command name
│       └── run_external_cmd()    ─ Spawn child process
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

helper.rs                     ─ rustyline Completer implementation
utils.rs                      ─ File path completion utilities
```

### Global State

All shell state is stored in `LazyLock<Mutex<...>>` statics:

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

## Building & Running

```sh
cargo build --release
cargo run
```
## Want to know more

`Read the code`