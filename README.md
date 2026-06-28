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

### RushBeats Music Player

A terminal-based music player that searches YouTube, streams/downloads audio via `mpv` + `yt-dlp`, and manages local playlists. Launch with:

```
rush --music
```

**Features:**

- YouTube search via `yt-dlp` with configurable cookie support (off/auto/manual)
- Audio playback via `mpv` through Unix socket IPC (`/tmp/rushbeats_mpv.sock`)
- Playlist management: create, delete, rename, add/remove songs
- YouTube playlist import by URL (press `a` in playlists view)
- Download queue: download songs as MP3 (requires `ffmpeg`)
- Shuffle mode, repeat modes (OFF / ALL / ONE)
- Persistent config, playlists, download queue, and session in `~/.rushbeats/`
- Settings menu (press `S`): download path, seek step, cookies, browser, max results

**Key bindings:**

| Key | Action |
|-----|--------|
| `/` or `s` | Search YouTube |
| `Enter` | Play selected |
| `Space` | Pause/Resume |
| `n` / `p` | Next / Previous track |
| `x` | Stop playback |
| `R` | Toggle shuffle |
| `L` | Cycle repeat (OFF→ALL→ONE) |
| `←` / `→` | Seek backward/forward |
| `f` | Open playlists |
| `c` | Create playlist |
| `a` (playlists) | Import YouTube playlist |
| `d` | Download song |
| `a` (search/songs) | Add song to playlist |
| `S` | Settings |
| `h` / `?` | Help |
| `q` (home) | Quit |
| `Esc` | Back one view |

**Required runtime dependencies:** `mpv`, `yt-dlp`. Optional: `ffmpeg` + `ffprobe` for downloads.

### DSA Learning Mode

Interactive sorting algorithm visualizer. Launch with:

```
rush --dsa               # start DSA learning mode
rush --learn             # same as --dsa
```

**Visualization:**
- Elements displayed as a colored box grid `[ XX ]`
- Green = sorted position, Red = currently compared, Yellow = being swapped
- Label row shows `CMP`, `SWAP`, or index for each element
- Stats panel tracks comparisons, swaps, steps, and complexity info for 6 sorting algorithms

**Custom array input:**
1. Press `Enter` on the visualizer → prompts for array size (2-20)
2. Enter size, press `Enter` → prompts for space-separated values
3. Enter matching values, press `Enter` → visualizer runs on your data
4. `Esc` at any point cancels back to the visualizer

**Compare mode:**
1. On the topic menu, press `c` to enter compare selection
2. Pick first algorithm (Enter), then second algorithm (Enter)
3. Side-by-side view shows both algorithms running on the same array
4. Bottom bar auto-determines the winner by time complexity

| Key (Visualizer) | Action |
|------------------|--------|
| `←` / `→` | Step backward / forward |
| `Space` | Step forward |
| `r` | Reset with new random array |
| `Enter` | Custom array input |
| `m` | Back to menu |
| `q` / `Esc` | Quit |

| Key (Compare) | Action |
|---------------|--------|
| `←` / `→` | Step both algorithms |
| `Space` | Step forward |
| `r` | Reset with new random array |
| `m` | Back to menu |
| `q` | Quit |

| Key (Menu) | Action |
|------------|--------|
| `↑` / `↓` | Navigate |
| `Enter` | Select algorithm |
| `c` | Toggle compare selection mode |
| `q` / `Esc` | Quit / Cancel |

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

music/                        ─ RushBeats music player (feature = "music")
├── mod.rs                    ─ pub fn run(), module exports
├── app.rs                    ─ MusicApp struct, config, playlists, JSON persistence
├── ui.rs                     ─ Ratatui rendering: search, playlists, settings, help
├── input.rs                  ─ Key handling for all music views
└── player.rs                 ─ mpv IPC, yt-dlp search, download queue, YouTube import

dsa/                          ─ DSA learning mode (feature = "dsa")
├── mod.rs                    ─ Step type, DsaApp state, pub fn run()
├── app.rs                    ─ App state, random array generation, compare setup
├── ui.rs                     ─ Ratatui rendering: menu, visualizer, compare, input screens
├── input.rs                  ─ Key handling for all screens
├── sorting.rs                ─ Step generators for 6 algorithms
└── topics.rs                 ─ Topic definitions with complexity metadata
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
| `rand` | Random array generation for DSA mode |
| `anyhow` / `thiserror` | Error handling |
| `bytes` | Buffer management |
| `ratatui` / `crossterm` | TUI rendering and terminal I/O (feature `tui`) |
| `arboard` | Clipboard access in TUI mode (feature `tui`) |
| `serde` / `serde_json` | Config and playlist persistence (feature `music`) |

## Installation

### Pre-built binary (via GitHub Releases)
## Linux (x86_64)

```sh
curl -LO https://github.com/Ryanu01/rust_shell/releases/latest/download/rush-linux-x86_64
chmod +x rush-linux-x86_64
sudo mv rush-linux-x86_64 /usr/local/bin/rush
```

## Linux (ARM64)

```sh
curl -LO https://github.com/Ryanu01/rust_shell/releases/latest/download/rush-linux-aarch64
chmod +x rush-linux-aarch64
sudo mv rush-linux-aarch64 /usr/local/bin/rush
```

## macOS (Intel)

```sh
curl -LO https://github.com/Ryanu01/rust_shell/releases/latest/download/rush-macos-x86_64
chmod +x rush-macos-x86_64
sudo mv rush-macos-x86_64 /usr/local/bin/rush
```

## Windows (x86_64)

```powershell
curl -LO https://github.com/Ryanu01/rust_shell/releases/latest/download/rush-windows-x86_64.exe
.\rush-windows-x86_64.exe
```
### From source

Requires Rust 1.75+.

```sh
cargo install --git https://github.com/Ryanu01/rust_shell
```

### Build locally

```sh
cargo build --release                     # includes TUI via default features
cargo build --release --no-default-features  # REPL-only, no TUI
cargo build --release --features dsa       # includes TUI + DSA learning mode
cargo build --release --features music     # includes TUI + RushBeats music player
./target/release/rush                     # normal shell
./target/release/rush --dsa               # DSA learning mode
./target/release/rush --music             # RushBeats music player
```
## Want to know more

`Read the code`