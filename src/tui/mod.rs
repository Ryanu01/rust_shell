mod app;
mod ui;
mod input;
mod exec;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum OutputStyle {
    Plain,
    Command,
    Directory,
    Executable,
}

pub(crate) fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

pub struct App {
    pub(crate) input: String,
    pub(crate) cursor: usize,
    pub(crate) output_lines: Vec<(String, OutputStyle)>,
    pub(crate) scroll_offset: usize,
    pub(crate) history: Vec<String>,
    pub(crate) history_idx: Option<usize>,
    pub(crate) running: bool,
    pub(crate) cwd: String,
    pub(crate) completions: Vec<String>,
    pub(crate) completion_start: usize,
    pub(crate) mouse_capture: bool,
}
