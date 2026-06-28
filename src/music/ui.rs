use super::app::{DownloadStatus, MusicApp, RepeatMode, View};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const SPINNER: [char; 4] = ['|', '/', '-', '\\'];

impl MusicApp {
    pub(crate) fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        self.draw_header(frame, chunks[0]);
        self.draw_hints(frame, chunks[1]);
        self.draw_separator(frame, chunks[2]);
        self.draw_content(frame, chunks[3]);
        self.draw_separator(frame, chunks[4]);
        self.draw_status_line(frame, chunks[5]);
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let text = Line::from(vec![
            Span::styled(" RushBeats ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("v0.1.0", Style::default().fg(Color::Gray)),
            Span::raw("  "),
            Span::styled(
                if self.ytdlp_updating { " [yt-dlp updating...]" } else { "" },
                Style::default().fg(Color::Yellow),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(text).style(Style::default().bg(Color::Rgb(30, 30, 30))),
            area,
        );
    }

    fn draw_hints(&self, frame: &mut Frame, area: Rect) {
        let hints = match self.view {
            View::Search => " /s search  Enter play  Space pause  n/p next/prev  f playlists  d download  a add to playlist  S settings  h help  q quit",
            View::Playlists => " Enter open  c create  a import YT playlist  x delete  Escape back  h help  q quit",
            View::PlaylistSongs => " Enter play  d download  r rename  X remove  u sync yt  a add to playlist  Escape back  h help  q quit",
            View::AddToPlaylist => " Enter confirm  Escape cancel",
            View::Settings => " ↑↓ navigate  Enter edit  Escape back",
            View::NowPlaying => " Space pause  n/p next/prev  ←→ seek  s search  f playlists  t jump  h help",
            View::Help => " ↑↓ scroll  any key close",
        };
        frame.render_widget(
            Paragraph::new(Line::styled(hints, Style::default().fg(Color::DarkGray)))
                .style(Style::default().bg(Color::Rgb(20, 20, 20))),
            area,
        );
    }

    fn draw_separator(&self, frame: &mut Frame, area: Rect) {
        let line = "─".repeat(area.width as usize);
        frame.render_widget(
            Paragraph::new(Line::styled(line, Style::default().fg(Color::Rgb(50, 50, 50)))),
            area,
        );
    }

    fn draw_content(&self, frame: &mut Frame, area: Rect) {
        if self.url_input_active {
            self.draw_url_input(frame, area);
            return;
        }

        if !self.query_active && self.query.is_empty() && self.search_results.is_empty() && self.view != View::Playlists && self.view != View::PlaylistSongs && self.view != View::Settings && self.view != View::Help && self.view != View::AddToPlaylist {
            self.draw_welcome(frame, area);
            return;
        }

        match self.view {
            View::Search => self.draw_search_view(frame, area),
            View::Playlists => self.draw_playlists_view(frame, area),
            View::PlaylistSongs => self.draw_playlist_songs_view(frame, area),
            View::AddToPlaylist => self.draw_add_to_playlist_view(frame, area),
            View::Settings => self.draw_settings_view(frame, area),
            View::NowPlaying => self.draw_now_playing_view(frame, area),
            View::Help => self.draw_help_view(frame, area),
        }
    }

    fn draw_welcome(&self, frame: &mut Frame, area: Rect) {
        let lines = vec![
            Line::raw(""),
            Line::styled("  RushBeats", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::raw("  Terminal Music Player"),
            Line::raw(""),
            Line::raw("  Press / or s to search YouTube"),
            Line::raw("  Press f to browse playlists"),
            Line::raw("  Press h for help"),
            Line::raw(""),
            Line::styled(
                format!("  yt-dlp: {}  mpv: {}  ffmpeg: {}  JS: {}",
                    if self.ytdlp_available { "✓" } else { "✗" },
                    if self.mpv_available { "✓" } else { "✗" },
                    if self.ffmpeg_available { "✓" } else { "✗" },
                    if self.js_runtime_available { "✓" } else { "✗" },
                ),
                Style::default().fg(Color::Gray),
            ),
        ];
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn draw_search_view(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(area);

        let query_text = if self.query_active {
            format!("  Search: {}█", self.query)
        } else {
            format!("  Query: {}", self.query)
        };
        frame.render_widget(
            Paragraph::new(Line::styled(&query_text, Style::default().fg(Color::Yellow)))
                .block(Block::default().borders(Borders::ALL)),
            chunks[0],
        );

        if self.yt_blocked {
            frame.render_widget(
                Paragraph::new(Line::styled("  YouTube is blocking requests — configure cookies in Settings (S)", Style::default().fg(Color::Red)))
                    .block(Block::default().borders(Borders::ALL)),
                chunks[1],
            );
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for (i, result) in self.search_results.iter().enumerate() {
            if i >= self.search_count {
                break;
            }
            let prefix = if self.is_playing() && !self.playing_from_playlist && self.playing_index == i as i32 {
                " >"
            } else {
                "  "
            };
            let style = if i == self.search_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let is_downloaded = self.is_downloaded(&result.video_id);
            let dl_mark = if is_downloaded { " [D]" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<3}", prefix, i + 1), style),
                Span::styled(&result.title, style),
                Span::styled(
                    format!("  {}", Self::format_duration(result.duration)),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(dl_mark, Style::default().fg(Color::Green)),
            ]));
        }
        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(format!(" Results ({}) ", self.search_count))),
            chunks[1],
        );
    }

    fn draw_playlists_view(&self, frame: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();
        for (i, pl) in self.playlists.iter().enumerate() {
            if i >= self.playlist_count {
                break;
            }
            let style = if i == self.playlist_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let shared = if pl.is_shared { " [shared]" } else { "" };
            let yt = if pl.is_youtube_playlist { " [YT]" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<3}", i + 1), style),
                Span::styled(&pl.name, style),
                Span::styled(
                    format!("  ({} songs){}{}", pl.songs.len(), yt, shared),
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }
        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(format!(" Playlists ({}) ", self.playlist_count))),
            area,
        );
    }

    fn draw_playlist_songs_view(&self, frame: &mut Frame, area: Rect) {
        if self.current_playlist_idx >= self.playlists.len() {
            return;
        }
        let pl = &self.playlists[self.current_playlist_idx];
        let yt = if pl.is_youtube_playlist { " [YT]" } else { "" };
        let shared = if pl.is_shared { " [shared]" } else { "" };

        let mut lines: Vec<Line> = Vec::new();
        for (i, song) in pl.songs.iter().enumerate() {
            let prefix = if self.is_playing() && self.playing_from_playlist
                && self.playing_playlist_idx == self.current_playlist_idx as i32
                && self.playing_index == i as i32
            {
                " >"
            } else {
                "  "
            };
            let style = if i == self.playlist_song_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let is_downloaded = self.is_downloaded(&song.video_id);
            let dl_mark = if is_downloaded { " [D]" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("{}{:<3}", prefix, i + 1), style),
                Span::styled(&song.title, style),
                Span::styled(
                    format!("  {}", Self::format_duration(song.duration)),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(dl_mark, Style::default().fg(Color::Green)),
            ]));
        }

        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(format!(" {} {}{} ({} songs) ", pl.name, yt, shared, pl.songs.len()))),
            area,
        );
    }

    fn draw_add_to_playlist_view(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Adding to playlist: ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.add_song_title, Style::default().fg(Color::White)),
            ])),
            chunks[0],
        );

        let mut lines: Vec<Line> = Vec::new();
        for (i, pl) in self.playlists.iter().enumerate() {
            let style = if i == self.add_to_playlist_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<3}", i + 1), style),
                Span::styled(&pl.name, style),
                Span::styled(format!("  ({} songs)", pl.songs.len()), Style::default().fg(Color::Gray)),
            ]));
        }
        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Select Playlist ")),
            chunks[1],
        );
    }

    fn draw_settings_view(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<(&str, String)> = vec![
            ("Download Path", self.config.download_path.clone()),
            ("Seek Step", format!("{}s", self.config.seek_step)),
            ("Remember Session", if self.config.remember_session { "ON".into() } else { "OFF".into() }),
            ("Shuffle Mode", if self.config.shuffle_mode { "ON".into() } else { "OFF".into() }),
            ("Repeat Mode", match self.config.repeat_mode {
                RepeatMode::Off => "OFF",
                RepeatMode::All => "ALL",
                RepeatMode::One => "ONE",
            }.into()),
            ("Max Results", format!("{}", self.config.max_results)),
            ("Cookies Mode", match self.config.cookies_mode {
                super::app::CookiesMode::Off => "Off",
                super::app::CookiesMode::Auto => "Auto from browser",
                super::app::CookiesMode::Manual => "Manual file",
            }.into()),
            ("Browser", self.config.cookies_browser.clone()),
            ("Cookies File", self.config.cookies_file.clone()),
        ];

        let mut lines: Vec<Line> = Vec::new();
        for (i, (label, value)) in items.iter().enumerate() {
            let style = if i == self.settings_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let val_style = if i == self.settings_selected && self.settings_editing {
                Style::default().fg(Color::Cyan).bg(Color::Rgb(40, 40, 40))
            } else {
                Style::default().fg(Color::Gray)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<20}", label), style),
                Span::raw(" "),
                if self.settings_editing && i == self.settings_selected {
                    Span::styled(format!("{}█", self.settings_edit_buf), val_style)
                } else {
                    Span::styled(value.clone(), val_style)
                },
            ]));
        }
        frame.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" Settings ")),
            area,
        );
    }

    fn draw_now_playing_view(&self, frame: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::raw(""),
            Line::styled("  Now Playing", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::raw(""),
        ];

        if self.is_playing() {
            lines.push(Line::styled(
                format!("  {}", self.current_title),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::raw(""));
            let status = if self.paused { " [PAUSED]" } else { " [PLAYING]" };
            let shuffle = if self.config.shuffle_mode { " [SHUFFLE]" } else { "" };
            let repeat = match self.config.repeat_mode {
                RepeatMode::Off => "",
                RepeatMode::All => " [REPEAT:ALL]",
                RepeatMode::One => " [REPEAT:ONE]",
            };
            lines.push(Line::from(vec![
                Span::styled(status, Style::default().fg(Color::Green)),
                Span::styled(shuffle, Style::default().fg(Color::Yellow)),
                Span::styled(repeat, Style::default().fg(Color::Yellow)),
            ]));
        } else {
            lines.push(Line::styled("  No track playing", Style::default().fg(Color::Gray)));
            lines.push(Line::raw(""));
            lines.push(Line::styled("  Press / or s to search, then press Enter to play", Style::default().fg(Color::DarkGray)));
        }

        lines.push(Line::raw(""));
        lines.push(Line::raw("  Space: Play/Pause  n: Next  p: Prev  ←→: Seek"));
        lines.push(Line::raw("  s: Search  f: Playlists  q: Quit"));

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn draw_url_input(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        frame.render_widget(
            Paragraph::new(Line::styled(
                "  Import YouTube Playlist",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )).block(Block::default().borders(Borders::ALL)),
            chunks[0],
        );

        let prompt = format!("  Paste playlist URL: {}", self.url_buffer);
        frame.render_widget(
            Paragraph::new(Line::styled(
                if self.url_buffer.is_empty() { prompt + "█" } else { prompt },
                Style::default().fg(Color::Yellow),
            )).block(Block::default().borders(Borders::ALL)),
            chunks[1],
        );

        frame.render_widget(
            Paragraph::new(Line::styled(
                "  Enter confirm  Esc cancel",
                Style::default().fg(Color::DarkGray),
            )).block(Block::default().borders(Borders::ALL)),
            chunks[2],
        );
    }

    fn draw_help_view(&self, frame: &mut Frame, area: Rect) {
        let help_lines = vec![
            Line::styled("  RushBeats Key Bindings", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Line::raw(""),
            Line::styled("  Playback", Style::default().fg(Color::Yellow)),
            Line::raw("  / or s    Search YouTube"),
            Line::raw("  Enter     Play selected"),
            Line::raw("  Space     Pause/Resume"),
            Line::raw("  n         Next track"),
            Line::raw("  p         Previous track"),
            Line::raw("  x         Stop"),
            Line::raw("  R         Toggle shuffle"),
            Line::raw("  L         Cycle repeat (OFF→ALL→ONE)"),
            Line::raw("  ← →      Seek backward/forward"),
            Line::raw("  t         Jump to time"),
            Line::raw(""),
            Line::styled("  Navigation", Style::default().fg(Color::Yellow)),
            Line::raw("  ↑ ↓       Move selection"),
            Line::raw("  PgUp/Dn   Page up/down"),
            Line::raw("  g/G       Jump to start/end"),
            Line::raw("  Esc       Back"),
            Line::raw(""),
            Line::styled("  Playlists", Style::default().fg(Color::Yellow)),
            Line::raw("  f         Open playlists"),
            Line::raw("  c         Create playlist"),
            Line::raw("  e         Rename playlist"),
            Line::raw("  X         Remove song"),
            Line::raw("  x         Delete playlist"),
            Line::raw("  d         Download"),
            Line::raw("  u         Sync YouTube playlist"),
            Line::raw(""),
            Line::styled("  Other", Style::default().fg(Color::Yellow)),
            Line::raw("  S         Settings"),
            Line::raw("  h/?       Help"),
            Line::raw("  q         Quit"),
        ];
        frame.render_widget(
            Paragraph::new(help_lines)
                .block(Block::default().borders(Borders::ALL).title(" Help ")),
            area,
        );
    }

    fn draw_status_line(&self, frame: &mut Frame, area: Rect) {
        let spinner = SPINNER[self.download_spinner_idx % 4];

        let mut spans = Vec::new();

        if self.is_playing() {
            let status = if self.paused { " [PAUSED]" } else { "" };
            let shuffle = if self.config.shuffle_mode { " [SHUFFLE]" } else { "" };
            let repeat = match self.config.repeat_mode {
                RepeatMode::Off => "",
                RepeatMode::All => " [REPEAT:ALL]",
                RepeatMode::One => " [REPEAT:ONE]",
            };
            spans.push(Span::styled(
                format!(" Now Playing: {}{}{}{}", self.current_title, status, shuffle, repeat),
                Style::default().fg(Color::Green),
            ));
        } else {
            spans.push(Span::styled(" Not playing", Style::default().fg(Color::Gray)));
        }

        if !self.status_message.is_empty() {
            spans.push(Span::styled(
                format!("  | {} ", self.status_message),
                Style::default().fg(Color::Yellow),
            ));
        }

        let active = self.download_queue.tasks.iter().filter(|t| matches!(t.status, DownloadStatus::Active)).count();
        let pending = self.download_queue.tasks.iter().filter(|t| matches!(t.status, DownloadStatus::Pending)).count();
        if active > 0 || pending > 0 {
            let total = active + pending;
            spans.push(Span::styled(
                format!("  [{}] {}/{}", spinner, active, total),
                Style::default().fg(Color::Cyan),
            ));
        }
        if !self.ytdlp_update_status.is_empty() {
            spans.push(Span::styled(
                format!("  [yt-dlp: {}]", self.ytdlp_update_status),
                Style::default().fg(Color::Magenta),
            ));
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Rgb(30, 30, 30))),
            area,
        );
    }

    fn is_downloaded(&self, video_id: &str) -> bool {
        self.download_queue.tasks.iter().any(|t| t.video_id == video_id && matches!(t.status, DownloadStatus::Completed))
    }
}
