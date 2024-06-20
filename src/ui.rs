use std::{
    io::{self, stdout, Stdout},
    sync::mpsc::{self, Receiver}
};
use crate::{player::{Player, PlayerAction}, song_base::SongBase};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{block::*, *},
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
    Add(String),
    Play,
    Pause,
    NextSong,
    PrevSong,
    Fetch(Option<String>),
    Jump(i32),
    Invalid,
    Empty,
    Exit
}

impl AppActions {
    fn parse_command(command: &str) -> Self {
        let command_splitted: Vec<&str> = command.split_whitespace().collect();
        if command_splitted.is_empty() {
            return AppActions::Empty;
        }

        let main_command = command_splitted.get(0).unwrap().trim().to_lowercase();
        match main_command.as_str() {
            "add" | "pour" => {
                let song_name = command_splitted.get(1);
                if song_name.is_none() {
                    AppActions::Add("*".to_string())
                } else {
                    AppActions::Add(song_name.unwrap().to_owned().to_string())
                }
            },
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
            },
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
            },
            "exit" | "quit" | "out" => AppActions::Exit, 
            _ => AppActions::Invalid,
        }
    }
}

pub struct App {
    exit: bool,
    command: String,
    info: Vec<String>,
    player: Player,
    system_log: Vec<String>,
    receiver: Receiver<PlayerAction>,
    song_base: SongBase
}

impl App {
    pub fn new() -> App {
        let (sender, receiver) = mpsc::channel();
        let sender_clone = sender.clone();
        let player = Player::new(sender);

        let song_base = SongBase::new("song.db", sender_clone).unwrap();
        App {
            exit: false,
            command: String::new(),
            info: Vec::new(),
            player,
            system_log: Vec::new(),
            receiver,
            song_base
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
                    PlayerAction::ConnectionMessage(msg) => self.log_system(msg),
                    _ => (),
                }
            }
            if self.player.is_empty() {
                match self.player.next_track() {
                    Ok(index) => self.log_info(format!("Now Playing: {}", self.player.get_song_detail(index).unwrap())),
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
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event);
            }
            _ => {}
        };
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
            AppActions::Add(song_name) => {
                if song_name == "*" {
                    self.log_info("Specify Song Name".to_string())
                } else {
                    let song = self.song_base.get_song(song_name);
                    match song {
                        Err(err) => self.log_info(err),
                        Ok(song) => {
                            let song_name = song.song_name.clone();
                            match self.player.add_track(song) {
                                Ok(index) => self.log_info(format!("Added {} to queue @ {}", song_name, index + 1)),
                                Err(err) => self.log_info(err),
                            }
                        },
                    }
                }
            }
            AppActions::Play => {
                let play_result = self.player.play(false);
                
                // self.
            },
            AppActions::Empty => {

            }
            AppActions::Pause => todo!(),
            AppActions::NextSong => {
                match self.player.next_track() {
                    Ok(index) => {
                        let next_track_log = self.player.get_song_detail(index)
                            .map(|song_name| format!("Skipped Track, Now Playing: {}", song_name))
                            .unwrap();
                        self.log_info(next_track_log);
                    },
                    Err(err) => self.log_info(err)
                }
            },
            AppActions::PrevSong => todo!(),
            AppActions::Fetch(path) => {
                let return_value = self.song_base.scan_songs(path);
                let log_info = return_value.map(|s| format!("Searching {}", s))
                    .map_err(|err| err.to_string()).unwrap();
                self.log_info(log_info);
            },
            AppActions::Jump(_) => todo!(),
            AppActions::Exit => {
                self.log_info("See Ya! Have a Great Time");
                self.exit = true;
            },
            AppActions::Invalid => self.log_info("Can't dEcIpHeR that, check out Top Right ↗️"),
        }
        self.command.clear();
    }

    fn log_info<S: ToString>(&mut self, message: S) {
        let message = message.to_string();
        if self.info.len() >= 9 {
            self.info.remove(0);
        }
        self.info.push(message);
    }

    fn log_system(&mut self, message: String) {
        if self.system_log.len() >= 8 {
            self.system_log.remove(0);
        }
        self.system_log.push(message);
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

        //Input Box - Lower Layout
        let input_box = Block::default()
            .title(" Command Box ".red())
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(Color::White));

        let input_area = main_layout[main_layout.len() - 1];

        let input_string = Line::raw(&self.command);

        let input_paragraph = Paragraph::new(input_string)
            .block(input_box)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        input_paragraph.render(input_area, buf);

        //Upper Layout
        let upper_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ])
            .split(main_layout[0]);

        //Queue Box - Left InnerLayout
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

        let middle_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Fill(1)])
            .split(upper_layout[1]);

        //thanush Box - middle Top
        let thanush_box = Block::default()
            .title(" Thanush ".fg(Color::Red))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White));

        let thanush_area = middle_layout[0];

        let thanush_lines: Vec<Line> = (0..10).map(|_| Line::raw("Thanush")).collect();
        let thanush_para = Paragraph::new(thanush_lines)
            .block(thanush_box)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        thanush_para.render(thanush_area, buf);

        //Info Box - Middle Bottom
        let info_box = Block::default()
            .title(" Info ".fg(Color::Red))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White))
            .title_alignment(Alignment::Center);

        let info_area = middle_layout[1];
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

        let right_layout = Layout::default()
            .constraints([Constraint::Fill(1), Constraint::Fill(1)])
            .split(upper_layout[2]);

        //Playlist Box - Right Top
        let playlist_box = Block::default()
            .title(" Playlist ".fg(Color::Red))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White));

        let playlist_area = right_layout[0];

        let playlist_para = Paragraph::new("1. Baka Baka")
            .block(playlist_box)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        playlist_para.render(playlist_area, buf);

        //System LogBox - Right Bottom
        let log_box = Block::default()
            .title(" System Log ".fg(Color::Red))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White));

        let log_area = right_layout[1];

        let log_lines: Vec<Line> = self
            .system_log
            .iter()
            .map(|x| Line::raw(x).fg(Color::White))
            .collect();

        let log_para = Paragraph::new(log_lines)
            .block(log_box)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        log_para.render(log_area, buf);
    }
}
