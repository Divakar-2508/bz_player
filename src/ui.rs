use crate::{
    player::{Player, PlayerAction},
    song::{Playable, PlaylistActions},
    song_base::SongBase,
    utility::{render_search_song, UtilityState},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{
    prelude::*,
    widgets::{block::*, *},
};
use std::{
    io::{self, stdout, Stdout},
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> io::Result<Tui> {
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

pub fn restore() -> io::Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

enum AppActions {
    Add(Playable),
    Play,
    Pause,
    NextSong,
    PrevSong,
    Fetch(Option<String>),
    Jump(i32),
    Invalid,
    Empty,
    Exit,
    Utility(UtilityState),
}

impl AppActions {
    fn parse_command(command: &str) -> Self {
        let command_splitted: Vec<&str> = command.split_whitespace().collect();
        if command_splitted.is_empty() {
            return AppActions::Empty;
        }

        let main_command = command_splitted.get(0).unwrap().trim().to_lowercase();
        match main_command.as_str() {
            "add" | "push" => {
                let args = command_splitted.get(1..);
                match args {
                    Some(args) if *args.get(0).unwrap() == "-p" => {
                        if args.get(1).is_none() {
                            AppActions::Add(Playable::None)
                        } else {
                            let playlist_index = args.get(1).unwrap().parse::<u8>();
                            if let Ok(index) = playlist_index {
                                AppActions::Add(Playable::Playlist(index))
                            } else {
                                AppActions::Add(Playable::Playlist(0))
                            }
                        }
                    }
                    Some(args) if *args.get(0).unwrap() == "-s" => {
                        if args.get(1).is_none() {
                            AppActions::Add(Playable::None)
                        } else {
                            let song_ids: Vec<u32> = args
                                .get(1..)
                                .unwrap()
                                .into_iter()
                                .filter_map(|x| x.parse::<u32>().ok())
                                .collect();
                            AppActions::Add(Playable::SearchSong(song_ids))
                        }
                    }
                    Some(song_name) => {
                        if song_name.get(0).is_none() {
                            AppActions::Add(Playable::None)
                        } else {
                            AppActions::Add(Playable::Song(song_name.join(" ")))
                        }
                    }
                    None => AppActions::Add(Playable::None),
                }
            }
            "play" | "p" => AppActions::Play,
            "next" | "skip" => AppActions::NextSong,
            "pause" | "wait" => AppActions::Pause,
            "fetch" | "scan" => {
                let path = command_splitted.get(1);
                if let Some(path) = path {
                    AppActions::Fetch(Some(path.to_owned().to_string()))
                } else {
                    AppActions::Fetch(None)
                }
            }
            "prev" | "back" | "rollback" => AppActions::PrevSong,
            "jump" => {
                let song_index = command_splitted.get(1);
                if song_index.is_none() {
                    return AppActions::Jump(-1);
                }
                let song_index = song_index.unwrap().parse::<i32>();
                if song_index.is_err() {
                    return AppActions::Jump(-1);
                }
                return AppActions::Jump(song_index.unwrap());
            }
            "exit" | "quit" | "out" => AppActions::Exit,
            "playlist" => {
                let args = command_splitted.get(1..);
                if args.is_none() {
                    return AppActions::Utility(UtilityState::Playlist(PlaylistActions::Show));
                }
                let args = args.unwrap();
                let playlist_subcommand = args.get(0);
                if playlist_subcommand.is_none() {
                    return AppActions::Utility(UtilityState::Playlist(PlaylistActions::Show));
                }
                let playlist_command = match *playlist_subcommand.unwrap() {
                    "show" | "-s" | "s" => PlaylistActions::Show,
                    "create" | "-c" | "c" => match args.get(1..) {
                        None => PlaylistActions::Create(None, None),
                        Some(playlist_create_args) => {
                            if let Some(folder_index) =
                                playlist_create_args.iter().position(|arg| arg == &"-f")
                            {
                                let playlist_name = playlist_create_args[..folder_index].join(" ");
                                let folder = PathBuf::from(
                                    playlist_create_args[(folder_index + 1)..].join(" "),
                                );
                                if folder.exists() {
                                    PlaylistActions::Create(Some(playlist_name), Some(folder))
                                } else {
                                    PlaylistActions::Create(Some(playlist_name), None)
                                }
                            } else {
                                PlaylistActions::Create(Some(playlist_create_args.join(" ")), None)
                            }
                        }
                    },
                    "add" | "-a" | "a" => {
                        let playlist_args = args.get(1..);
                        if playlist_args.is_none() {
                            PlaylistActions::Add(0, None)
                        } else {
                            let args = playlist_args.unwrap();
                            let playlist_id = args.get(0).unwrap().parse::<u8>();
                            if playlist_id.is_err() {
                                PlaylistActions::Add(0, None)
                            } else {
                                let song_names = args.get(1..);
                                if song_names.is_none() {
                                    PlaylistActions::Add(playlist_id.unwrap(), None)
                                } else {
                                    let is_whole = song_names.unwrap().get(0);
                                    if is_whole.is_none() {
                                        PlaylistActions::Add(0, None)
                                    } else if is_whole.unwrap().eq(&"*") {
                                        PlaylistActions::AddAll(playlist_id.ok())
                                    } else {
                                        let song_names: Vec<String> = song_names
                                            .unwrap()
                                            .join(" ")
                                            .split(",")
                                            .map(|s| s.trim().to_owned())
                                            .collect();
                                        PlaylistActions::Add(playlist_id.unwrap(), Some(song_names))
                                    }
                                }
                            }
                        }
                    }
                    "view" | "-v" | "v" => {
                        let playlist_id = args.get(1);
                        if playlist_id.is_none() {
                            PlaylistActions::View(None)
                        } else {
                            let playlist_id = playlist_id.unwrap().parse::<u8>().ok();
                            PlaylistActions::View(playlist_id)
                        }
                    }
                    _ => PlaylistActions::Invalid,
                };
                AppActions::Utility(UtilityState::Playlist(playlist_command))
            }
            "search" => {
                let song_name = command_splitted.get(1..);
                if song_name.is_none() {
                    AppActions::Utility(UtilityState::SearchSong("*".to_string()))
                } else {
                    AppActions::Utility(UtilityState::SearchSong(song_name.unwrap().join(" ")))
                }
            }
            _ => AppActions::Invalid,
        }
    }
}

pub struct App {
    exit: bool,
    command: String,
    info: Vec<String>,
    info_lines: u32,
    player: Player,
    receiver: Receiver<PlayerAction>,
    song_base: SongBase,
    utility_state: UtilityState,
}

impl App {
    pub fn new() -> App {
        let (sender, receiver) = mpsc::channel();
        let sender_clone = sender.clone();
        let player = Player::new(sender);

        let song_base = SongBase::init("song.db", sender_clone).unwrap();
        App {
            exit: false,
            command: String::new(),
            info: Vec::new(),
            info_lines: 1,
            player,
            receiver,
            song_base,
            utility_state: UtilityState::Help,
        }
    }

    pub fn run(&mut self, terminal: &mut Tui) -> io::Result<()> {
        self.command = "Hello".to_string();
        self.info.push("Konnichiwa (◔◡◔)".to_string());
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
            let message = self.receiver.try_recv();
            if let Ok(message) = message {
                match message {
                    PlayerAction::ConnectionMessage(msg) => self.log_info(msg),
                }
            }
            if self.player.is_empty() {
                match self.player.next_track() {
                    Ok(index) => self.log_info(format!(
                        "Now Playing: {}",
                        self.player.get_song_detail(index as usize).unwrap()
                    )),
                    _ => (),
                }
            }
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event);
                }
                _ => {}
            };
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(ch) => self.command.push(ch),
            KeyCode::Enter => self.handle_command(),
            KeyCode::Backspace => {
                self.command.pop();
            }
            _ => {}
        }
    }

    fn handle_command(&mut self) {
        let command = AppActions::parse_command(&self.command);
        match command {
            AppActions::Add(playable) => match playable {
                Playable::None => self.log_info("Specify Song Name".to_string()),
                Playable::Song(song_name) => {
                    let song = self.song_base.find_song_by_name(song_name);
                    match song {
                        Err(err) => self.log_info(err),
                        Ok(song) => {
                            let song_name = song.song_name.clone();
                            match self.player.add_track(song) {
                                Ok(index) => self
                                    .log_info(format!("Added {} to queue @ {}", song_name, index)),
                                Err(err) => self.log_info(err),
                            }
                        }
                    }
                }
                Playable::SearchSong(song_ids) => {
                    if let UtilityState::SearchSong(_) = self.utility_state {
                    } else {
                        self.log_info("Utility Search Song need to be active to use `-s` flag");
                    }
                }
                Playable::Playlist(playlist_id) => {
                    if playlist_id == 0 {
                        self.log_info(
                            "Please Mention the playlist id, use 'help playlist' for more info",
                        );
                        self.command.clear();
                        return;
                    }
                    let playlist = self.song_base.get_playlist(playlist_id);
                    match playlist {
                        Err(err) => self.log_info(err),
                        Ok(playlist) => {
                            let playlist_name = playlist.playlist_name.clone();
                            match self.player.add_playlist(playlist) {
                                Ok(index) => self.log_info(format!(
                                    "Added Playlist {} @ {}",
                                    playlist_name, index
                                )),
                                Err(err) => self.log_info(err),
                            }
                        }
                    }
                }
            },
            AppActions::Play => match self.player.play(false) {
                Ok(_) => self.log_info("Track Resumed"),
                Err(err) => self.log_info(err),
            },
            AppActions::Empty => {
                self.log_info("Please Enter a Command ;)");
            }
            AppActions::Pause => {
                self.player.pause();
                self.log_info("Paused.");
            }
            AppActions::NextSong => match self.player.next_track() {
                Ok(index) => {
                    let next_track_log = self
                        .player
                        .get_song_detail(index as usize)
                        .map(|song_name| format!("Skipped Track, Now Playing: {}", song_name))
                        .unwrap();
                    self.log_info(next_track_log);
                }
                Err(err) => self.log_info(err),
            },
            AppActions::PrevSong => todo!(),
            AppActions::Fetch(path) => {
                let return_value = self.song_base.scan_songs(path);
                let log_info = return_value
                    .map(|s| format!("Searching {}", s))
                    .unwrap_or_else(|err| err.to_string());
                self.log_info(log_info);
            }
            AppActions::Jump(index) => match usize::try_from(index) {
                Ok(index) => match self.player.jump_track(index - 1) {
                    Ok(index) => self.log_info(format!(
                        "Playing {}",
                        self.player.get_song_detail(index as usize).unwrap()
                    )),
                    Err(err) => self.log_info(err),
                },
                Err(_) => self.log_info("Enter a valid index"),
            },
            AppActions::Exit => {
                self.log_info("See Ya! Have a Great Time");
                self.exit = true;
            }
            AppActions::Utility(utility) => match utility {
                UtilityState::Playlist(playlist_command) => {
                    self.log_info(&playlist_command);
                    match playlist_command {
                        PlaylistActions::Show => {}
                        PlaylistActions::Create(playlist_name, folder_name) => {
                            self.log_info(format!("{:?} {:?}", playlist_name, folder_name));
                            if folder_name.is_some() {
                                if playlist_name.is_some() {
                                    if let Err(err) = self.song_base.create_playlist_from_path(
                                        playlist_name.unwrap(),
                                        folder_name.unwrap(),
                                    ) {
                                        self.log_info(err);
                                    }
                                } else {
                                    let folder_name = folder_name.unwrap();
                                    if let Err(err) = self.song_base.create_playlist_from_path(
                                        folder_name
                                            .file_name()
                                            .unwrap()
                                            .to_string_lossy()
                                            .to_string(),
                                        folder_name,
                                    ) {
                                        self.log_info(err);
                                    }
                                }
                            } else {
                                if playlist_name.is_none() {
                                    self.log_info(
                                        "Specify the song name to create, try 'help playlist'",
                                    );
                                } else {
                                    match self.song_base.create_playlist(playlist_name.unwrap()) {
                                        Err(err) => self.log_info(err),
                                        _ => (),
                                    }
                                }
                            }
                        }
                        PlaylistActions::AddAll(id) => {
                            if id.is_none() {
                                self.log_info(
                                    "Please mention playlist id to add to, try 'help playlist'",
                                );
                                self.command.clear();
                                return;
                            }
                            match self
                                .song_base
                                .add_playlist_song(id.unwrap(), self.player.get_queue_ids())
                            {
                                Err(err) => self.log_info(err),
                                _ => (),
                            }
                        }
                        _ => {}
                    }
                }
                _ => self.utility_state = utility,
            },
            AppActions::Invalid => self.log_info("Can't get that, Check out Top Right ↗️"),
        }
        self.command.clear();
    }

    fn log_info<S: ToString>(&mut self, message: S) {
        let message = message.to_string();
        let is_two_lines = |msg: &String| -> bool { msg.len() > 72 };
        if is_two_lines(&message) {
            self.info_lines += 2;
        } else {
            self.info_lines += 1;
        }
        if self.info.len() >= 3 {
            let deleted_message = self.info.remove(0);
            if is_two_lines(&deleted_message) {
                self.info_lines -= 2;
            } else {
                self.info_lines -= 1;
            }
        }
        self.info.push(message);
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Title::from(" BZ_Player ".bold().red());

        let main_block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White));

        main_block.render(area, buf);

        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Fill(1), Constraint::Length(3)].as_ref())
            .split(area);

        //Bottom
        let input_area = main_layout[main_layout.len() - 1];
        command_box(input_area, buf, &self.command);

        //Upper Layout
        let upper_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Length(80)])
            .split(main_layout[0]);

        //Queue Box - Left Full
        let queue_block = Block::default()
            .title(" Play Queue ".fg(Color::Red))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White));

        let queue_area = upper_layout[0];
        let queue_logs: Vec<Line> = self
            .player
            .get_queue()
            .iter()
            .enumerate()
            .map(|(index, song_name)| {
                if index == self.player.current_song() as usize {
                    Line::from(format!("{}. {}", index + 1, song_name)).fg(Color::Green)
                } else {
                    Line::from(format!("{}. {}", index + 1, song_name))
                }
            })
            .collect();

        let queue_para = Paragraph::new(queue_logs)
            .block(queue_block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });
        queue_para.render(queue_area, buf);

        let right_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(3), Constraint::Fill(1)])
            .split(upper_layout[1]);

        //Info Box - right Bottom
        let info_box = Block::default()
            .title(" Info ".fg(Color::Red))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White))
            .title_alignment(Alignment::Center);

        let info_area = right_layout[1];
        let info_lines: Vec<Line> = self
            .info
            .iter()
            .map(|comm| Line::raw(format!("! {}", comm)).fg(Color::Yellow))
            .collect();

        let info_para = Paragraph::new(info_lines)
            .block(info_box)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });
        info_para.render(info_area, buf);

        let top_right_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(1)])
            .split(right_layout[0]);

        let utility_area = top_right_layout[0];
        match &self.utility_state {
            UtilityState::Playlist(playlist_command) => {
                match playlist_command {
                    PlaylistActions::Show => {
                        // let playlist_names = self.song_base.get_playlists();
                        // render_playlist_view(utility_area, buf, playlist_names)
                    }
                    _ => (),
                }
                // let playlists = vec!["baka".to_string(), "smth".to_string(), "peace".to_string()];
                // playlist(top_right_layout[0], buf, &playlists);
                // self.log_info(playlist_command.clone())
            }
            UtilityState::SearchSong(song_name) => {
                let song_list = self.song_base.filter_song(song_name);
                render_search_song(utility_area, buf, song_list.as_ref(), song_name);
            }
            _ => (),
        }

        let help_box = Block::default()
            .title(" Help ".fg(Color::Red))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White))
            .title_alignment(Alignment::Center);

        let help_area = top_right_layout[1];
        let help_lines = "Use the Command At the Bottom :)\n\nFetch [dir]: Scan and add songs in the directory
        Add [song_name]: Append the Song to the queue\nPause/Play/Resume: Self Explanatory\nJump [index]: Skip to the song in the queue
        Next: Advance to next Song\nPrev: Rollback to previous Song\nQuit/Exit: Close the App\nManual: Open up the Help Page";
        let help_lines: Vec<Line> = help_lines
            .lines()
            .into_iter()
            .map(|line| Line::raw(line).fg(Color::Blue))
            .collect();

        let help_para = Paragraph::new(help_lines)
            .block(help_box)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });
        help_para.render(help_area, buf);
    }
}

fn command_box(rect: Rect, buf: &mut Buffer, command: &str) {
    //Input Box - Lower Layout
    let input_box = Block::default()
        .title(" Command Box ".red())
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(Color::White));

    let input_string = Line::raw(command);

    let input_paragraph = Paragraph::new(input_string)
        .block(input_box)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    input_paragraph.render(rect, buf);
}
