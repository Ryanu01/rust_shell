use super::app::{CookiesMode, MusicApp, RepeatMode, Song, View};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

impl MusicApp {
    pub(crate) fn handle_events(&mut self) -> std::io::Result<bool> {
        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    self.on_key(key.code);
                }
                _ => {}
            }
        }
        Ok(true)
    }

    fn on_key(&mut self, code: KeyCode) {
        if self.query_active {
            self.handle_query_key(code);
            return;
        }
        if self.settings_editing {
            self.handle_settings_edit_key(code);
            return;
        }

        if self.url_input_active {
            self.handle_url_input_key(code);
            return;
        }

        match code {
            KeyCode::Char(c) => match c {
                'q' if self.view == View::Search || self.view == View::NowPlaying => self.running = false,
                '/' | 's' if self.view != View::Playlists && self.view != View::PlaylistSongs => {
                    self.query_active = true;
                    self.query.clear();
                    self.view = View::Search;
                }
                ' ' => self.toggle_pause(),
                'n' => self.play_next(),
                'p' => self.play_prev(),
                'x' if self.view != View::Playlists && self.view != View::PlaylistSongs => self.stop_playback(),
                'R' => {
                    self.config.shuffle_mode = !self.config.shuffle_mode;
                    self.set_status(if self.config.shuffle_mode { "Shuffle ON" } else { "Shuffle OFF" });
                }
                'L' => {
                    self.config.repeat_mode = match self.config.repeat_mode {
                        RepeatMode::Off => RepeatMode::All,
                        RepeatMode::All => RepeatMode::One,
                        RepeatMode::One => RepeatMode::Off,
                    };
                    self.set_status(match self.config.repeat_mode {
                        RepeatMode::Off => "Repeat OFF",
                        RepeatMode::All => "Repeat ALL",
                        RepeatMode::One => "Repeat ONE",
                    });
                }
                't' => {
                    self.set_status("Jump to time: enter mm:ss");
                }
                'f' => {
                    self.view = View::Playlists;
                }
                'S' => {
                    self.view = View::Settings;
                    self.settings_selected = 0;
                    self.settings_editing = false;
                }
                'h' | '?' => {
                    self.view = View::Help;
                }
                'd' => self.handle_download_key(),
                'a' => {
                    if self.view == View::Playlists {
                        self.url_input_active = true;
                        self.url_buffer.clear();
                    } else {
                        self.handle_add_to_playlist();
                    }
                }
                'c' if self.view == View::Playlists => self.create_playlist(),
                'e' if self.view == View::Playlists => self.rename_playlist(),
                'x' if self.view == View::Playlists => self.delete_playlist(),
                'X' if self.view == View::PlaylistSongs => self.remove_song_from_playlist(),
                'u' if self.view == View::PlaylistSongs => {
                    self.set_status("YouTube playlist sync not fully implemented");
                }
                'r' if self.view == View::PlaylistSongs => {
                    self.set_status("Rename: not implemented in this view");
                }
                _ => {}
            },
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Left => self.seek(false),
            KeyCode::Right => {
                self.seek(true);
            }
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Home => self.go_home(),
            KeyCode::End => self.go_end(),
            KeyCode::Esc => self.handle_escape(),
            KeyCode::Backspace => {
                if self.view == View::Search && !self.query.is_empty() {
                    self.query.pop();
                    self.search_youtube();
                }
            }
            _ => {}
        }
    }

    fn handle_query_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.query.push(c);
                self.search_youtube();
            }
            KeyCode::Enter => {
                self.query_active = false;
                self.search_youtube();
            }
            KeyCode::Backspace => {
                self.query.pop();
                if !self.query.is_empty() {
                    self.search_youtube();
                }
            }
            KeyCode::Esc => {
                self.query_active = false;
            }
            _ => {}
        }
    }

    fn handle_settings_edit_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                self.commit_settings_edit();
                self.settings_editing = false;
            }
            KeyCode::Esc => {
                self.settings_editing = false;
            }
            KeyCode::Char(c) => {
                self.settings_edit_buf.insert(self.settings_edit_cursor, c);
                self.settings_edit_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.settings_edit_cursor > 0 {
                    self.settings_edit_cursor -= 1;
                    self.settings_edit_buf.remove(self.settings_edit_cursor);
                }
            }
            KeyCode::Delete => {
                if self.settings_edit_cursor < self.settings_edit_buf.len() {
                    self.settings_edit_buf.remove(self.settings_edit_cursor);
                }
            }
            KeyCode::Left => {
                if self.settings_edit_cursor > 0 {
                    self.settings_edit_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.settings_edit_cursor < self.settings_edit_buf.len() {
                    self.settings_edit_cursor += 1;
                }
            }
            KeyCode::Home => self.settings_edit_cursor = 0,
            KeyCode::End => self.settings_edit_cursor = self.settings_edit_buf.len(),
            _ => {}
        }
    }

    fn commit_settings_edit(&mut self) {
        match self.settings_selected {
            0 => self.config.download_path = self.settings_edit_buf.clone(),
            1 => {
                if let Ok(n) = self.settings_edit_buf.parse::<i32>() {
                    self.config.seek_step = n.clamp(1, 300);
                }
            }
            2 => self.config.remember_session = !self.config.remember_session,
            3 => self.config.shuffle_mode = !self.config.shuffle_mode,
            4 => {
                self.config.repeat_mode = match self.config.repeat_mode {
                    RepeatMode::Off => RepeatMode::All,
                    RepeatMode::All => RepeatMode::One,
                    RepeatMode::One => RepeatMode::Off,
                };
            }
            5 => {
                if let Ok(n) = self.settings_edit_buf.parse::<i32>() {
                    self.config.max_results = n.clamp(10, 150);
                }
            }
            6 => {
                self.config.cookies_mode = match self.config.cookies_mode {
                    CookiesMode::Off => CookiesMode::Auto,
                    CookiesMode::Auto => CookiesMode::Manual,
                    CookiesMode::Manual => CookiesMode::Off,
                };
            }
            7 => {
                let browsers = ["firefox", "chrome", "chromium", "brave", "edge", "safari", "opera", "vivaldi"];
                if let Some(pos) = browsers.iter().position(|&b| b == self.config.cookies_browser) {
                    let next = (pos + 1) % browsers.len();
                    self.config.cookies_browser = browsers[next].to_string();
                }
            }
            8 => self.config.cookies_file = self.settings_edit_buf.clone(),
            _ => {}
        }
        self.save_config();
    }

    fn handle_enter(&mut self) {
        match self.view {
            View::Search => {
                if self.search_selected < self.search_count {
                    self.play_search_result(self.search_selected);
                    self.view = View::NowPlaying;
                }
            }
            View::Playlists => {
                if self.playlist_selected < self.playlist_count {
                    self.current_playlist_idx = self.playlist_selected;
                    self.playlist_song_selected = 0;
                    self.playlist_song_scroll = 0;
                    self.view = View::PlaylistSongs;
                }
            }
            View::PlaylistSongs => {
                if self.playlist_song_selected < self.playlists[self.current_playlist_idx].songs.len() {
                    self.play_playlist_song(self.current_playlist_idx, self.playlist_song_selected);
                }
            }
            View::AddToPlaylist => {
                if self.add_to_playlist_selected < self.playlist_count {
                    let song = Song {
                        title: self.add_song_title.clone(),
                        video_id: self.add_song_video_id.clone(),
                        duration: self.add_song_duration,
                    };
                    self.playlists[self.add_to_playlist_selected].songs.push(song);
                    self.save_playlists();
                    self.set_status("Added to playlist");
                    self.view = View::PlaylistSongs;
                }
            }
            View::Settings => {
                let _ = self.settings_selected;
                self.settings_editing = true;
                self.settings_edit_buf = match self.settings_selected {
                    0 => self.config.download_path.clone(),
                    1 => self.config.seek_step.to_string(),
                    5 => self.config.max_results.to_string(),
                    8 => self.config.cookies_file.clone(),
                    _ => String::new(),
                };
                self.settings_edit_cursor = self.settings_edit_buf.len();
                if self.settings_selected == 2 || self.settings_selected == 3 || self.settings_selected == 4 || self.settings_selected == 6 || self.settings_selected == 7 {
                    self.commit_settings_edit();
                }
            }
            _ => {}
        }
    }

    fn handle_escape(&mut self) {
        if self.query_active {
            self.query_active = false;
            return;
        }
        if self.settings_editing {
            self.settings_editing = false;
            return;
        }
        match self.view {
            View::Search => {}
            View::Playlists => self.view = View::Search,
            View::PlaylistSongs => self.view = View::Playlists,
            View::NowPlaying => self.view = View::Search,
            View::AddToPlaylist => self.view = View::PlaylistSongs,
            View::Settings => self.view = View::Search,
            View::Help => self.view = View::Search,
        }
    }

    fn handle_download_key(&mut self) {
        match self.view {
            View::Search => {
                if self.search_selected < self.search_count {
                    self.start_download(self.search_selected, false, 0);
                }
            }
            View::PlaylistSongs => {
                if self.playlist_song_selected < self.playlists[self.current_playlist_idx].songs.len() {
                    self.start_download(self.playlist_song_selected, true, self.current_playlist_idx);
                }
            }
            _ => {}
        }
    }

    fn handle_add_to_playlist(&mut self) {
        let (title, video_id, duration) = match self.view {
            View::Search if self.search_selected < self.search_count => {
                let r = &self.search_results[self.search_selected];
                (r.title.clone(), r.video_id.clone(), r.duration)
            }
            View::PlaylistSongs if self.current_playlist_idx < self.playlists.len()
                && self.playlist_song_selected < self.playlists[self.current_playlist_idx].songs.len() =>
            {
                let s = &self.playlists[self.current_playlist_idx].songs[self.playlist_song_selected];
                (s.title.clone(), s.video_id.clone(), s.duration)
            }
            _ => return,
        };

        if self.playlist_count == 0 {
            self.set_status("No playlists — create one with 'c' in playlist view");
            return;
        }

        self.add_song_title = title;
        self.add_song_video_id = video_id;
        self.add_song_duration = duration;
        self.add_to_playlist_selected = 0;
        self.view = View::AddToPlaylist;
    }

    fn move_up(&mut self) {
        match self.view {
            View::Search => {
                if self.search_selected > 0 {
                    self.search_selected -= 1;
                }
            }
            View::Playlists => {
                if self.playlist_selected > 0 {
                    self.playlist_selected -= 1;
                }
            }
            View::PlaylistSongs => {
                if self.playlist_song_selected > 0 {
                    self.playlist_song_selected -= 1;
                }
            }
            View::AddToPlaylist => {
                if self.add_to_playlist_selected > 0 {
                    self.add_to_playlist_selected -= 1;
                }
            }
            View::Settings => {
                if self.settings_selected > 0 {
                    self.settings_selected -= 1;
                }
            }
            _ => {}
        }
    }

    fn move_down(&mut self) {
        match self.view {
            View::Search => {
                if self.search_selected + 1 < self.search_count {
                    self.search_selected += 1;
                }
            }
            View::Playlists => {
                if self.playlist_selected + 1 < self.playlist_count {
                    self.playlist_selected += 1;
                }
            }
            View::PlaylistSongs => {
                if self.playlist_song_selected + 1 < self.playlists[self.current_playlist_idx].songs.len() {
                    self.playlist_song_selected += 1;
                }
            }
            View::AddToPlaylist => {
                if self.add_to_playlist_selected + 1 < self.playlist_count {
                    self.add_to_playlist_selected += 1;
                }
            }
            View::Settings => {
                if self.settings_selected + 1 < 9 {
                    self.settings_selected += 1;
                }
            }
            _ => {}
        }
    }

    fn page_up(&mut self) {
        match self.view {
            View::Search => {
                self.search_selected = self.search_selected.saturating_sub(10);
            }
            View::Playlists => {
                self.playlist_selected = self.playlist_selected.saturating_sub(10);
            }
            View::PlaylistSongs => {
                self.playlist_song_selected = self.playlist_song_selected.saturating_sub(10);
            }
            _ => {}
        }
    }

    fn page_down(&mut self) {
        match self.view {
            View::Search => {
                self.search_selected = (self.search_selected + 10).min(self.search_count.saturating_sub(1));
            }
            View::Playlists => {
                self.playlist_selected = (self.playlist_selected + 10).min(self.playlist_count.saturating_sub(1));
            }
            View::PlaylistSongs => {
                if self.current_playlist_idx < self.playlists.len() {
                    let max = self.playlists[self.current_playlist_idx].songs.len().saturating_sub(1);
                    self.playlist_song_selected = (self.playlist_song_selected + 10).min(max);
                }
            }
            _ => {}
        }
    }

    fn go_home(&mut self) {
        match self.view {
            View::Search => self.search_selected = 0,
            View::Playlists => self.playlist_selected = 0,
            View::PlaylistSongs => self.playlist_song_selected = 0,
            _ => {}
        }
    }

    fn handle_url_input_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.url_buffer.push(c);
            }
            KeyCode::Enter => {
                let url = self.url_buffer.trim().to_string();
                self.url_input_active = false;
                if !url.is_empty() {
                    self.import_youtube_playlist(&url);
                }
            }
            KeyCode::Backspace => {
                self.url_buffer.pop();
            }
            KeyCode::Esc => {
                self.url_input_active = false;
                self.url_buffer.clear();
            }
            _ => {}
        }
    }

    fn go_end(&mut self) {
        match self.view {
            View::Search => self.search_selected = self.search_count.saturating_sub(1),
            View::Playlists => self.playlist_selected = self.playlist_count.saturating_sub(1),
            View::PlaylistSongs => {
                if self.current_playlist_idx < self.playlists.len() {
                    self.playlist_song_selected = self.playlists[self.current_playlist_idx].songs.len().saturating_sub(1);
                }
            }
            _ => {}
        }
    }
}
