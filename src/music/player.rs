use super::app::{
    CookiesMode, DownloadStatus, DownloadTask, MusicApp, Playlist, RepeatMode, SearchResult, Song,
    IPC_SOCKET, MAX_DOWNLOAD_QUEUE, MAX_PLAYLISTS,
};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

impl MusicApp {
    pub(crate) fn check_dependencies(&mut self) {
        self.ytdlp_available = which("yt-dlp").is_some();
        self.mpv_available = which("mpv").is_some();
        self.ffmpeg_available = which("ffmpeg").is_some() && which("ffprobe").is_some();
        self.js_runtime_available =
            which("deno").is_some() || which("node").is_some() || which("bun").is_some()
                || self.bin_dir.join("deno").exists();
    }

    pub(crate) fn search_youtube(&mut self) {
        if !self.ytdlp_available || self.query.is_empty() {
            return;
        }

        let mut cmd = Command::new("yt-dlp");
        cmd.args(["--flat-playlist", "--quiet", "--no-warnings"]);
        self.append_cookie_args(&mut cmd);
        cmd.arg("--print").arg("%(title)s|||%(id)s|||%(duration)s");
        cmd.arg(format!("ytsearch{}:{}", self.config.max_results, self.query));
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = match cmd.output() {
            Ok(o) => o,
            Err(_) => {
                self.set_status("yt-dlp search failed");
                return;
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if stderr.contains("Sign in to confirm") || stderr.contains("bot") {
            self.yt_blocked = true;
        }

        self.search_results.clear();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("ERROR") || line.starts_with("WARNING") {
                continue;
            }
            let parts: Vec<&str> = line.split("|||").collect();
            if parts.len() < 3 {
                continue;
            }
            let video_id = parts[1].trim();
            if video_id.len() < 5 || video_id.len() > 20 {
                continue;
            }
            let duration: i32 = parts[2].trim().parse().unwrap_or(0);
            self.search_results.push(SearchResult {
                title: parts[0].trim().to_string(),
                video_id: video_id.to_string(),
                duration,
            });
        }
        self.search_count = self.search_results.len();
        self.search_selected = 0;
        self.search_scroll = 0;
        self.set_status(&format!("Found {} results", self.search_count));
    }

    fn append_cookie_args(&self, cmd: &mut Command) {
        match self.config.cookies_mode {
            CookiesMode::Off => {}
            CookiesMode::Auto => {
                cmd.arg("--cookies-from-browser");
                cmd.arg(&self.config.cookies_browser);
            }
            CookiesMode::Manual => {
                if !self.config.cookies_file.is_empty() {
                    cmd.arg("--cookies");
                    cmd.arg(&self.config.cookies_file);
                }
            }
        }
    }

    pub(crate) fn start_mpv(&mut self) {
        if !self.mpv_available {
            return;
        }

        let socket_path = IPC_SOCKET;
        let _ = std::fs::remove_file(socket_path);

        let local_ytdlp = self.bin_dir.join("yt-dlp");
        let ytdlp_path = if local_ytdlp.exists() {
            local_ytdlp.to_string_lossy().to_string()
        } else {
            "yt-dlp".to_string()
        };

        let child = match Command::new("mpv")
            .args([
                "--no-video",
                "--idle=yes",
                "--force-window=no",
                "--really-quiet",
                &format!("--input-ipc-server={}", socket_path),
                "--ytdl-format=bestaudio[ext=m4a]/bestaudio[ext=webm]/bestaudio",
                &format!("--script-opts=ytdl_hook-ytdl_path={}", ytdlp_path),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return,
        };

        self.mpv_pid = Some(child.id());
        drop(child);

        for _ in 0..100 {
            if Path::new(socket_path).exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        if Path::new(socket_path).exists() {
            match UnixStream::connect(socket_path) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(Duration::from_millis(100))).ok();
                    self.mpv_fd = Some(stream);
                    self.mpv_connected = true;
                    self.mpv_send_command(r#"{"command":["observe_property",1,"eof-reached"]}"#);
                }
                Err(_) => {
                    self.mpv_connected = false;
                }
            }
        }
    }

    pub(crate) fn mpv_send_command(&mut self, cmd: &str) {
        if let Some(ref mut stream) = self.mpv_fd {
            let msg = format!("{}\n", cmd);
            let _ = stream.write_all(msg.as_bytes());
            let _ = stream.flush();
        }
    }

    fn mpv_read_response(&mut self) -> Option<String> {
        let mut buf = vec![0u8; 4096];
        if let Some(ref mut stream) = self.mpv_fd {
            match stream.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let s = String::from_utf8_lossy(&buf[..n]).to_string();
                    Some(s)
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub(crate) fn check_track_end(&mut self) {
        if !self.is_playing() || self.mpv_fd.is_none() {
            return;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if now < self.playback_started + 3 {
            let _ = self.mpv_read_response();
            return;
        }

        while let Some(resp) = self.mpv_read_response() {
            if resp.contains("\"event\":\"end-file\"") && (resp.contains("\"reason\":\"eof\"") || resp.contains("\"reason\":\"error\"")) {
                self.play_next();
                return;
            }
            if resp.contains("\"event\":\"property-change\"") && resp.contains("\"name\":\"eof-reached\"") && resp.contains("\"data\":true") {
                self.play_next();
                return;
            }
        }
    }

    pub(crate) fn play_search_result(&mut self, idx: usize) {
        if idx >= self.search_results.len() {
            return;
        }
        self.playing_from_playlist = false;
        self.playing_index = idx as i32;
        self.playing_playlist_idx = -1;
        self.current_title = self.search_results[idx].title.clone();

        let url = format!("https://www.youtube.com/watch?v={}", self.search_results[idx].video_id);
        self.mpv_load(&url);
    }

    pub(crate) fn play_playlist_song(&mut self, pl_idx: usize, song_idx: usize) {
        if pl_idx >= self.playlists.len() || song_idx >= self.playlists[pl_idx].songs.len() {
            return;
        }
        self.playing_from_playlist = true;
        self.playing_playlist_idx = pl_idx as i32;
        self.playing_index = song_idx as i32;
        self.current_title = self.playlists[pl_idx].songs[song_idx].title.clone();

        let video_id = self.playlists[pl_idx].songs[song_idx].video_id.clone();
        let local = self.get_local_file(&self.playlists[pl_idx].name, &video_id);

        if let Some(path) = local {
            self.mpv_load(&path);
        } else {
            let url = format!("https://www.youtube.com/watch?v={}", video_id);
            self.mpv_load(&url);
        }
    }

    fn mpv_load(&mut self, target: &str) {
        let cmd = format!(r#"{{"command":["loadfile","{}","replace"]}}"#, target.replace('\\', "\\\\").replace('"', "\\\""));
        self.mpv_send_command(&cmd);
        self.paused = false;
        self.playback_started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    pub(crate) fn toggle_pause(&mut self) {
        self.mpv_send_command(r#"{"command":["cycle","pause"]}"#);
        self.paused = !self.paused;
    }

    pub(crate) fn stop_playback(&mut self) {
        self.mpv_send_command(r#"{"command":["stop"]}"#);
        self.playing_index = -1;
        self.current_title.clear();
        self.paused = false;
    }

    pub(crate) fn play_next(&mut self) {
        if !self.is_playing() {
            return;
        }

        if self.playing_from_playlist {
            let pl_idx = self.playing_playlist_idx as usize;
            if pl_idx >= self.playlists.len() {
                return;
            }
            let count = self.playlists[pl_idx].songs.len();
            if count == 0 {
                return;
            }

            let next_idx = if self.config.shuffle_mode {
                self.shuffle_idx += 1;
                if self.shuffle_idx >= self.shuffle_order.len() {
                    match self.config.repeat_mode {
                        RepeatMode::Off => {
                            self.stop_playback();
                            return;
                        }
                        RepeatMode::All => {
                            self.shuffle_playlist(pl_idx);
                            self.shuffle_idx = 0;
                        }
                        RepeatMode::One => {
                            self.shuffle_idx = self.shuffle_order.len() - 1;
                        }
                    }
                }
                self.shuffle_order[self.shuffle_idx]
            } else {
                let next = self.playing_index as usize + 1;
                if next >= count {
                    match self.config.repeat_mode {
                        RepeatMode::Off => {
                            self.stop_playback();
                            return;
                        }
                        RepeatMode::All => 0,
                        RepeatMode::One => self.playing_index as usize,
                    }
                } else {
                    next
                }
            };
            self.play_playlist_song(pl_idx, next_idx);
        } else {
            let next = self.playing_index as usize + 1;
            if next >= self.search_results.len() {
                self.stop_playback();
                return;
            }
            self.play_search_result(next);
        }
    }

    pub(crate) fn play_prev(&mut self) {
        if !self.is_playing() {
            return;
        }

        if self.playing_from_playlist {
            let pl_idx = self.playing_playlist_idx as usize;
            if pl_idx >= self.playlists.len() {
                return;
            }
            let count = self.playlists[pl_idx].songs.len();
            if count == 0 {
                return;
            }

            let prev_idx = if self.config.shuffle_mode {
                if self.shuffle_idx > 0 {
                    self.shuffle_idx -= 1;
                }
                self.shuffle_order[self.shuffle_idx]
            } else {
                let prev = self.playing_index as usize;
                if prev == 0 {
                    count - 1
                } else {
                    prev - 1
                }
            };
            self.play_playlist_song(pl_idx, prev_idx);
        } else {
            let prev = self.playing_index as usize;
            if prev > 0 {
                self.play_search_result(prev - 1);
            }
        }
    }

    pub(crate) fn seek(&mut self, forward: bool) {
        let step = self.config.seek_step;
        let sign = if forward { "+" } else { "-" };
        let cmd = format!(r#"{{"command":["seek","{}{}","relative"]}}"#, sign, step);
        self.mpv_send_command(&cmd);
        let dir = if forward { "forward" } else { "backward" };
        self.set_status(&format!("Seek {} {}s", dir, step));
    }

    fn get_local_file(&self, playlist_name: &str, video_id: &str) -> Option<String> {
        let search = format!("[{}].mp3", video_id);
        let dir = Path::new(&self.config.download_path).join(Self::sanitize_filename(playlist_name));
        if dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains(&search) {
                        return Some(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }
        None
    }

    pub(crate) fn start_download(&mut self, idx: usize, from_playlist: bool, pl_idx: usize) {
        if !self.ffmpeg_available {
            self.set_status("ffmpeg not available, cannot download");
            return;
        }

        let (video_id, title, playlist_name) = if from_playlist {
            if pl_idx >= self.playlists.len() || idx >= self.playlists[pl_idx].songs.len() {
                return;
            }
            let song = &self.playlists[pl_idx].songs[idx];
            (song.video_id.clone(), song.title.clone(), self.playlists[pl_idx].name.clone())
        } else if idx < self.search_results.len() {
            let r = &self.search_results[idx];
            (r.video_id.clone(), r.title.clone(), String::new())
        } else {
            return;
        };

        let safe_title = Self::sanitize_filename(&title);
        let filename = format!("{}_{}.mp3", safe_title, video_id);
        let dir = if playlist_name.is_empty() {
            Path::new(&self.config.download_path).to_path_buf()
        } else {
            Path::new(&self.config.download_path).join(Self::sanitize_filename(&playlist_name))
        };

        let dest = dir.join(&filename);
        if dest.exists() {
            self.set_status("File already downloaded");
            return;
        }

        for task in &self.download_queue.tasks {
            if task.video_id == video_id && matches!(task.status, DownloadStatus::Pending | DownloadStatus::Active) {
                self.set_status("Already in download queue");
                return;
            }
        }

        if self.download_queue.tasks.len() >= MAX_DOWNLOAD_QUEUE {
            self.set_status("Download queue full");
            return;
        }

        self.download_queue.tasks.push(DownloadTask {
            video_id,
            title,
            filename,
            playlist_name,
            status: DownloadStatus::Pending,
        });
        self.save_download_queue();
        self.process_download_queue();
        self.set_status("Added to download queue");
    }

    fn process_download_queue(&mut self) {
        let pending: Vec<usize> = self.download_queue.tasks.iter()
            .enumerate()
            .filter(|(_, t)| matches!(t.status, DownloadStatus::Pending))
            .map(|(i, _)| i)
            .collect();

        for idx in pending {
            let task = self.download_queue.tasks[idx].clone();
            let dir = if task.playlist_name.is_empty() {
                Path::new(&self.config.download_path).to_path_buf()
            } else {
                Path::new(&self.config.download_path).join(Self::sanitize_filename(&task.playlist_name))
            };
            let dest = dir.join(&task.filename);

            std::fs::create_dir_all(&dir).ok();

            if dest.exists() {
                self.download_queue.tasks[idx].status = DownloadStatus::Completed;
                self.save_download_queue();
                continue;
            }

            self.download_queue.tasks[idx].status = DownloadStatus::Active;
            self.save_download_queue();

            let url = format!("https://www.youtube.com/watch?v={}", task.video_id);
            let mut cmd = Command::new("yt-dlp");
            self.append_cookie_args(&mut cmd);
            cmd.args([
                "-x", "--audio-format", "mp3",
                "--no-playlist", "--no-warnings",
                "-o", &dest.to_string_lossy(),
                &url,
            ]);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let output = cmd.output();
            match output {
                Ok(out) => {
                    if out.status.success() && dest.exists() {
                        self.download_queue.tasks[idx].status = DownloadStatus::Completed;
                        self.set_status(&format!("Downloaded: {}", task.title));
                    } else {
                        self.download_queue.tasks[idx].status = DownloadStatus::Failed;
                        self.set_status(&format!("Download failed: {}", task.title));
                    }
                }
                Err(_) => {
                    self.download_queue.tasks[idx].status = DownloadStatus::Failed;
                }
            }
            self.save_download_queue();
        }
    }

    pub(crate) fn import_youtube_playlist(&mut self, url: &str) {
        if !self.ytdlp_available {
            self.set_status("yt-dlp not available");
            return;
        }
        if !url.contains("youtube.com/playlist?list=") && !url.contains("youtu.be/playlist?list=") {
            self.set_status("Invalid YouTube playlist URL");
            return;
        }

        let mut cmd = Command::new("yt-dlp");
        self.append_cookie_args(&mut cmd);
        cmd.args([
            "--flat-playlist", "--quiet", "--no-warnings",
            "--playlist-items", "1",
            "--print", "%(playlist_title)s|||%(id)s|||%(duration)s",
            url,
        ]);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = match cmd.output() {
            Ok(o) => o,
            Err(_) => {
                self.set_status("Failed to fetch playlist");
                return;
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next().unwrap_or("").to_string();
        let parts: Vec<&str> = first_line.split("|||").collect();
        let playlist_title = if parts.len() >= 1 && !parts[0].is_empty() {
            parts[0].trim().to_string()
        } else {
            "Imported Playlist".to_string()
        };

        let mut cmd = Command::new("yt-dlp");
        self.append_cookie_args(&mut cmd);
        cmd.args([
            "--flat-playlist", "--quiet", "--no-warnings",
            "--print", "%(title)s|||%(id)s|||%(duration)s",
            url,
        ]);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = match cmd.output() {
            Ok(o) => o,
            Err(_) => {
                self.set_status("Failed to fetch playlist songs");
                return;
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut songs = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("ERROR") || line.starts_with("WARNING") {
                continue;
            }
            let p: Vec<&str> = line.split("|||").collect();
            if p.len() < 3 { continue; }
            let video_id = p[1].trim();
            if video_id.len() < 5 || video_id.len() > 20 { continue; }
            let duration: i32 = p[2].trim().parse().unwrap_or(0);
            songs.push(Song {
                title: p[0].trim().to_string(),
                video_id: video_id.to_string(),
                duration,
            });
        }

        if songs.is_empty() {
            self.set_status("No songs found in playlist");
            return;
        }

        let name = playlist_title;
        let filename = format!("{}.json", Self::sanitize_filename(&name));
        let pl = Playlist {
            name,
            filename,
            songs,
            is_youtube_playlist: true,
            youtube_playlist_url: url.to_string(),
            is_shared: false,
        };

        if self.playlist_count < MAX_PLAYLISTS {
            self.playlists.push(pl);
            self.playlist_count = self.playlists.len();
            self.save_playlists();
            self.set_status(&format!("Imported playlist ({} songs)", self.playlists.last().map(|p| p.songs.len()).unwrap_or(0)));
        } else {
            self.set_status("Maximum playlists reached");
        }
    }

    pub(crate) fn shutdown_mpv(&mut self) {
        self.mpv_send_command(r#"{"command":["quit"]}"#);
        std::thread::sleep(Duration::from_millis(200));

        if let Some(pid) = self.mpv_pid {
            let _ = Command::new("kill").arg(pid.to_string()).output();
        }

        self.mpv_fd = None;
        self.mpv_connected = false;
        let _ = std::fs::remove_file(IPC_SOCKET);
    }
}

fn which(name: &str) -> Option<String> {
    let output = Command::new("which").arg(name).output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}
