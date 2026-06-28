use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(crate) const IPC_SOCKET: &str = "/tmp/rushbeats_mpv.sock";
pub(crate) const MAX_PLAYLISTS: usize = 300;
pub(crate) const MAX_DOWNLOAD_QUEUE: usize = 1000;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Song {
    pub(crate) title: String,
    pub(crate) video_id: String,
    pub(crate) duration: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Playlist {
    pub(crate) name: String,
    pub(crate) filename: String,
    pub(crate) songs: Vec<Song>,
    pub(crate) is_youtube_playlist: bool,
    pub(crate) youtube_playlist_url: String,
    pub(crate) is_shared: bool,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum DownloadStatus {
    Pending,
    Active,
    Completed,
    Failed,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub(crate) video_id: String,
    pub(crate) title: String,
    pub(crate) filename: String,
    pub(crate) playlist_name: String,
    pub(crate) status: DownloadStatus,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum View {
    Search,
    Playlists,
    PlaylistSongs,
    AddToPlaylist,
    Settings,
    NowPlaying,
    Help,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum RepeatMode {
    Off,
    All,
    One,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum CookiesMode {
    Off,
    Auto,
    Manual,
}

pub(crate) struct Config {
    pub(crate) download_path: String,
    pub(crate) seek_step: i32,
    pub(crate) remember_session: bool,
    pub(crate) max_results: i32,
    pub(crate) cookies_mode: CookiesMode,
    pub(crate) cookies_browser: String,
    pub(crate) cookies_file: String,
    pub(crate) shuffle_mode: bool,
    pub(crate) repeat_mode: RepeatMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            download_path: format!("{}/Music/RushBeats", std::env::var("HOME").unwrap_or_default()),
            seek_step: 10,
            remember_session: false,
            max_results: 50,
            cookies_mode: CookiesMode::Off,
            cookies_browser: "firefox".to_string(),
            cookies_file: String::new(),
            shuffle_mode: false,
            repeat_mode: RepeatMode::Off,
        }
    }
}

pub(crate) struct DownloadQueue {
    pub(crate) tasks: Vec<DownloadTask>,
}

pub(crate) struct SearchResult {
    pub(crate) title: String,
    pub(crate) video_id: String,
    pub(crate) duration: i32,
}

pub(crate) struct MusicApp {
    pub(crate) view: View,
    pub(crate) running: bool,

    pub(crate) config: Config,
    pub(crate) config_dir: PathBuf,
    pub(crate) playlists_dir: PathBuf,
    pub(crate) bin_dir: PathBuf,

    pub(crate) search_results: Vec<SearchResult>,
    pub(crate) search_count: usize,
    pub(crate) search_selected: usize,
    pub(crate) search_scroll: usize,
    pub(crate) query: String,
    pub(crate) query_active: bool,

    pub(crate) playlists: Vec<Playlist>,
    pub(crate) playlist_count: usize,
    pub(crate) playlist_selected: usize,
    #[allow(dead_code)]
    pub(crate) playlist_scroll: usize,

    pub(crate) current_playlist_idx: usize,
    pub(crate) playlist_song_selected: usize,
    pub(crate) playlist_song_scroll: usize,

    pub(crate) playing_index: i32,
    pub(crate) playing_from_playlist: bool,
    pub(crate) playing_playlist_idx: i32,
    pub(crate) paused: bool,
    pub(crate) playback_started: u64,
    pub(crate) current_title: String,
    pub(crate) shuffle_order: Vec<usize>,
    pub(crate) shuffle_idx: usize,

    pub(crate) add_song_title: String,
    pub(crate) add_song_video_id: String,
    pub(crate) add_song_duration: i32,
    pub(crate) add_to_playlist_selected: usize,

    pub(crate) settings_selected: usize,
    pub(crate) settings_editing: bool,
    pub(crate) settings_edit_buf: String,
    pub(crate) settings_edit_cursor: usize,

    pub(crate) mpv_connected: bool,
    pub(crate) mpv_fd: Option<UnixStream>,
    pub(crate) mpv_pid: Option<u32>,

    pub(crate) ytdlp_available: bool,
    pub(crate) mpv_available: bool,
    pub(crate) ffmpeg_available: bool,
    pub(crate) js_runtime_available: bool,
    pub(crate) ytdlp_updating: bool,
    pub(crate) yt_blocked: bool,
    pub(crate) ytdlp_update_status: String,

    pub(crate) download_queue: DownloadQueue,
    pub(crate) download_spinner_idx: usize,

    pub(crate) status_message: String,
    pub(crate) status_timer: u64,

    pub(crate) url_input_active: bool,
    pub(crate) url_buffer: String,
}

impl MusicApp {
    pub fn new() -> Self {
        let config_dir = get_config_dir();
        let playlists_dir = config_dir.join("playlists");
        let bin_dir = config_dir.join("bin");

        fs::create_dir_all(&playlists_dir).ok();
        fs::create_dir_all(&bin_dir).ok();

        let mut app = Self {
            view: View::Search,
            running: true,
            config: Config::default(),
            config_dir: config_dir.clone(),
            playlists_dir,
            bin_dir,

            search_results: Vec::new(),
            search_count: 0,
            search_selected: 0,
            search_scroll: 0,
            query: String::new(),
            query_active: false,

            playlists: Vec::new(),
            playlist_count: 0,
            playlist_selected: 0,
            playlist_scroll: 0,

            current_playlist_idx: 0,
            playlist_song_selected: 0,
            playlist_song_scroll: 0,

            playing_index: -1,
            playing_from_playlist: false,
            playing_playlist_idx: -1,
            paused: false,
            playback_started: 0,
            current_title: String::new(),
            shuffle_order: Vec::new(),
            shuffle_idx: 0,

            add_song_title: String::new(),
            add_song_video_id: String::new(),
            add_song_duration: 0,
            add_to_playlist_selected: 0,

            settings_selected: 0,
            settings_editing: false,
            settings_edit_buf: String::new(),
            settings_edit_cursor: 0,

            mpv_connected: false,
            mpv_fd: None,
            mpv_pid: None,

            ytdlp_available: false,
            mpv_available: false,
            ffmpeg_available: false,
            js_runtime_available: false,
            ytdlp_updating: false,
            yt_blocked: false,
            ytdlp_update_status: String::new(),

            download_queue: DownloadQueue {
                tasks: Vec::new(),
            },
            download_spinner_idx: 0,

            status_message: String::new(),
            status_timer: 0,

            url_input_active: false,
            url_buffer: String::new(),
        };

        app.load_config();
        app.load_playlists();
        app.check_dependencies();
        app.load_download_queue();

        if app.config.remember_session {
            app.restore_session();
        }

        app
    }

    pub fn run_tui(&mut self, terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
        self.start_mpv();
        let mut last_tick = SystemTime::now();

        while self.running {
            terminal.draw(|frame| self.draw(frame))?;

            if last_tick.elapsed().unwrap_or_default().as_millis() >= 100 {
                self.tick();
                last_tick = SystemTime::now();
            }

            if !self.handle_events()? {
                break;
            }
        }

        self.shutdown_mpv();
        if self.config.remember_session {
            self.save_session();
        }
        self.save_download_queue();
        Ok(())
    }

    fn tick(&mut self) {
        self.download_spinner_idx = (self.download_spinner_idx + 1) % 4;
        self.check_track_end();

        if self.status_timer > 0 {
            self.status_timer -= 1;
            if self.status_timer == 0 {
                self.status_message.clear();
            }
        }
    }

    pub(crate) fn set_status(&mut self, msg: &str) {
        self.status_message = msg.to_string();
        self.status_timer = 30;
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.playing_index >= 0
    }

    pub(crate) fn shuffle_playlist(&mut self, playlist_idx: usize) {
        let count = self.playlists[playlist_idx].songs.len();
        self.shuffle_order = (0..count).collect();
        let mut rng = rand::thread_rng();
        use rand::seq::SliceRandom;
        self.shuffle_order.shuffle(&mut rng);
        self.shuffle_idx = 0;
    }

    pub(crate) fn format_duration(secs: i32) -> String {
        let m = secs / 60;
        let s = secs % 60;
        format!("{}:{:02}", m, s)
    }

    pub(crate) fn sanitize_filename(s: &str) -> String {
        let s: String = s
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' || c == '.' { c } else { '_' })
            .collect();
        let s = s.trim().to_string();
        if s.is_empty() { "untitled".to_string() } else { s }
    }
}

fn get_config_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("rushbeats")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".rushbeats")
    } else {
        PathBuf::from(".rushbeats")
    }
}

impl MusicApp {
    pub(crate) fn load_config(&mut self) {
        let path = self.config_dir.join("config.json");
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return,
        };
        if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(v) = cfg.get("download_path").and_then(|v| v.as_str()) {
                self.config.download_path = v.to_string();
            }
            if let Some(v) = cfg.get("seek_step").and_then(|v| v.as_i64()) {
                self.config.seek_step = v as i32;
            }
            if let Some(v) = cfg.get("remember_session").and_then(|v| v.as_bool()) {
                self.config.remember_session = v;
            }
            if let Some(v) = cfg.get("shuffle_mode").and_then(|v| v.as_bool()) {
                self.config.shuffle_mode = v;
            }
            if let Some(v) = cfg.get("repeat_mode").and_then(|v| v.as_i64()) {
                self.config.repeat_mode = match v {
                    1 => RepeatMode::All,
                    2 => RepeatMode::One,
                    _ => RepeatMode::Off,
                };
            }
            if let Some(v) = cfg.get("max_results").and_then(|v| v.as_i64()) {
                self.config.max_results = v as i32;
            }
            if let Some(v) = cfg.get("cookies_mode").and_then(|v| v.as_i64()) {
                self.config.cookies_mode = match v {
                    1 => CookiesMode::Auto,
                    2 => CookiesMode::Manual,
                    _ => CookiesMode::Off,
                };
            }
            if let Some(v) = cfg.get("cookies_browser").and_then(|v| v.as_str()) {
                self.config.cookies_browser = v.to_string();
            }
            if let Some(v) = cfg.get("cookies_file").and_then(|v| v.as_str()) {
                self.config.cookies_file = v.to_string();
            }
        }
    }

    pub(crate) fn save_config(&self) {
        let obj = serde_json::json!({
            "download_path": self.config.download_path,
            "seek_step": self.config.seek_step,
            "remember_session": self.config.remember_session,
            "shuffle_mode": self.config.shuffle_mode,
            "repeat_mode": match self.config.repeat_mode {
                RepeatMode::Off => 0,
                RepeatMode::All => 1,
                RepeatMode::One => 2,
            },
            "max_results": self.config.max_results,
            "cookies_mode": match self.config.cookies_mode {
                CookiesMode::Off => 0,
                CookiesMode::Auto => 1,
                CookiesMode::Manual => 2,
            },
            "cookies_browser": self.config.cookies_browser,
            "cookies_file": self.config.cookies_file,
        });
        let path = self.config_dir.join("config.json");
        if let Ok(content) = serde_json::to_string_pretty(&obj) {
            let _ = fs::write(&path, content);
        }
    }

    pub(crate) fn load_playlists(&mut self) {
        let path = self.config_dir.join("playlists.json");
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => {
                let _ = fs::write(&path, r#"{"playlists":[]}"#);
                return;
            }
        };
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(arr) = val.get("playlists").and_then(|v| v.as_array()) {
                for entry in arr {
                    let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let filename = entry.get("filename").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let pl_path = self.playlists_dir.join(&filename);
                    if let Ok(pl_content) = fs::read_to_string(&pl_path) {
                        if let Ok(pl_val) = serde_json::from_str::<serde_json::Value>(&pl_content) {
                            let mut pl = Playlist {
                                name: pl_val.get("name").and_then(|v| v.as_str()).unwrap_or(&name).to_string(),
                                filename,
                                songs: Vec::new(),
                                is_youtube_playlist: pl_val.get("type").and_then(|v| v.as_str()) == Some("youtube"),
                                youtube_playlist_url: pl_val.get("playlist_url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                is_shared: pl_val.get("is_shared").and_then(|v| v.as_bool()).unwrap_or(false),
                            };
                            if let Some(songs) = pl_val.get("songs").and_then(|v| v.as_array()) {
                                for s in songs {
                                    let title = s.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let video_id = s.get("video_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let duration = s.get("duration").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                    if is_valid_video_id(&video_id) {
                                        pl.songs.push(Song { title, video_id, duration });
                                    }
                                }
                            }
                            if self.playlist_count < MAX_PLAYLISTS {
                                self.playlists.push(pl);
                                self.playlist_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn save_playlists(&self) {
        let index: Vec<serde_json::Value> = self.playlists.iter().map(|pl| {
            serde_json::json!({"name": pl.name, "filename": pl.filename})
        }).collect();
        let index_obj = serde_json::json!({"playlists": index});
        let _ = fs::write(self.config_dir.join("playlists.json"), serde_json::to_string_pretty(&index_obj).unwrap_or_default());

        for pl in &self.playlists {
            let songs: Vec<serde_json::Value> = pl.songs.iter().map(|s| {
                serde_json::json!({"title": s.title, "video_id": s.video_id, "duration": s.duration})
            }).collect();
            let pl_obj = serde_json::json!({
                "name": pl.name,
                "type": if pl.is_youtube_playlist { "youtube" } else { "local" },
                "is_shared": pl.is_shared,
                "playlist_url": pl.youtube_playlist_url,
                "songs": songs,
            });
            let _ = fs::write(
                self.playlists_dir.join(&pl.filename),
                serde_json::to_string_pretty(&pl_obj).unwrap_or_default(),
            );
        }
    }

    pub(crate) fn save_download_queue(&self) {
        let pending: Vec<&DownloadTask> = self.download_queue.tasks.iter()
            .filter(|t| matches!(t.status, DownloadStatus::Pending | DownloadStatus::Failed))
            .collect();
        let tasks: Vec<serde_json::Value> = pending.iter().map(|t| {
            serde_json::json!({
                "video_id": t.video_id,
                "title": t.title,
                "filename": t.filename,
                "playlist": t.playlist_name,
                "status": match t.status {
                    DownloadStatus::Pending => "pending",
                    DownloadStatus::Failed => "failed",
                    _ => "pending",
                },
            })
        }).collect();
        let obj = serde_json::json!({"tasks": tasks});
        let _ = fs::write(self.config_dir.join("download_queue.json"), serde_json::to_string_pretty(&obj).unwrap_or_default());
    }

    pub(crate) fn load_download_queue(&mut self) {
        let path = self.config_dir.join("download_queue.json");
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return,
        };
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(arr) = val.get("tasks").and_then(|v| v.as_array()) {
                for t in arr {
                    let video_id = t.get("video_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let title = t.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let filename = t.get("filename").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let playlist_name = t.get("playlist").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let status_str = t.get("status").and_then(|v| v.as_str()).unwrap_or("pending");
                    let status = match status_str {
                        "failed" => DownloadStatus::Failed,
                        _ => DownloadStatus::Pending,
                    };
                    self.download_queue.tasks.push(DownloadTask {
                        video_id,
                        title,
                        filename,
                        playlist_name,
                        status,
                    });
                }
            }
        }
    }

    pub(crate) fn save_session(&self) {
        if !self.config.remember_session {
            return;
        }
        let obj = serde_json::json!({
            "last_query": self.query,
            "last_playlist_idx": self.current_playlist_idx,
            "last_song_idx": self.playlist_song_selected,
            "was_playing_playlist": self.playing_from_playlist,
        });
        let _ = fs::write(self.config_dir.join("session.json"), serde_json::to_string_pretty(&obj).unwrap_or_default());
    }

    pub(crate) fn restore_session(&mut self) {
        let path = self.config_dir.join("session.json");
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return,
        };
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(q) = val.get("last_query").and_then(|v| v.as_str()) {
                self.query = q.to_string();
                if !self.query.is_empty() {
                    self.search_youtube();
                    self.view = View::Search;
                }
            }
        }
    }

    pub(crate) fn create_playlist(&mut self) {
        if self.playlist_count >= MAX_PLAYLISTS {
            self.set_status("Maximum playlists reached");
            return;
        }
        let name = format!("Playlist {}", self.playlist_count + 1);
        let filename = format!("{}.json", Self::sanitize_filename(&name));
        self.playlists.push(Playlist {
            name,
            filename,
            songs: Vec::new(),
            is_youtube_playlist: false,
            youtube_playlist_url: String::new(),
            is_shared: false,
        });
        self.playlist_count = self.playlists.len();
        self.save_playlists();
        self.set_status("Playlist created");
    }

    pub(crate) fn delete_playlist(&mut self) {
        if self.playlist_selected >= self.playlist_count {
            return;
        }
        let pl = &self.playlists[self.playlist_selected];
        let filename = pl.filename.clone();
        let name = pl.name.clone();
        let dir = Path::new(&self.config.download_path).join(Self::sanitize_filename(&name));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_file(self.playlists_dir.join(&filename));
        self.playlists.remove(self.playlist_selected);
        self.playlist_count = self.playlists.len();
        if self.playlist_selected > 0 && self.playlist_selected >= self.playlist_count {
            self.playlist_selected = self.playlist_count.saturating_sub(1);
        }
        self.save_playlists();
        self.set_status("Playlist deleted");
    }

    pub(crate) fn rename_playlist(&mut self) {
        if self.playlist_selected >= self.playlist_count {
            return;
        }
        let old_filename = self.playlists[self.playlist_selected].filename.clone();
        let new_name = format!("{}_renamed", self.playlists[self.playlist_selected].name);
        let new_filename = format!("{}.json", Self::sanitize_filename(&new_name));
        let _ = fs::rename(
            self.playlists_dir.join(&old_filename),
            self.playlists_dir.join(&new_filename),
        );
        self.playlists[self.playlist_selected].name = new_name;
        self.playlists[self.playlist_selected].filename = new_filename;
        self.save_playlists();
        self.set_status("Playlist renamed");
    }

    pub(crate) fn remove_song_from_playlist(&mut self) {
        if self.current_playlist_idx >= self.playlist_count {
            return;
        }
        let pl = &mut self.playlists[self.current_playlist_idx];
        if self.playlist_song_selected < pl.songs.len() {
            pl.songs.remove(self.playlist_song_selected);
            if self.playlist_song_selected > 0 && self.playlist_song_selected >= pl.songs.len() {
                self.playlist_song_selected = pl.songs.len().saturating_sub(1);
            }
            self.save_playlists();
            self.set_status("Song removed");
        }
    }
}

fn is_valid_video_id(id: &str) -> bool {
    id.len() == 11 && id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}
