use std::{
    io::{self, stdout, Stdout},
    sync::mpsc::{Receiver, Sender}
};

use crate::dj::Dj;
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

pub struct App {
    exit: bool,
    command: String,
    info: Vec<String>,
    player: Dj,
    info_reciever: Receiver<String>,
    system_log: Vec<String>,
    spl_sender: Sender<u8>
}

impl App {
    pub fn new() -> App {
        let (tx, rx) = std::sync::mpsc::channel();
        let (spl_tx, spl_rx) = std::sync::mpsc::channel();
        App {
            exit: false,
            command: String::new(),
            info: Vec::new(),
            player: Dj::new(tx, spl_rx),
            info_reciever: rx,
            system_log: Vec::new(),
            spl_sender: spl_tx,
        }
    }

    pub fn run(&mut self, terminal: &mut Tui) -> io::Result<()> {
        self.command = "Hello".to_string();
        self.info.push("Konnichiwa (◔◡◔)".to_string());
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
            let recv = self.info_reciever.try_recv();
            if let Ok(message) = recv {
                let message = message.trim().to_owned();
                if message.starts_with("105") {
                    self.log_info(message);
                } else if message.starts_with("101") {
                    self.log_info("Message Recieved".to_string());
                    let (_, message) = message.split_at(3);
                    if message.trim() == "Done" {
                        if self.player.len() > self.player.current_index as usize {
                            self.player.skip_track();
                            self.log_info("skippedd".to_string());
                        } else {
                            self.log_info("At Queue End, You there Buddy?!".to_string());
                        }
                    }
                } else {
                    self.log_system(message);
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
        let command = self.command.trim().to_lowercase();
        let command_vec = command.split_whitespace().collect::<Vec<&str>>();
        if command_vec.is_empty() {
            self.log_info("Empty Command ><".to_string());
            return;
        }
        let main_command = command_vec.get(0).unwrap().trim();
        match main_command {
            "quit" | "exit" | "break" => self.exit = true,
            "add" => {
                let song_name = command_vec.get(1..).unwrap().join(" ");
                if song_name.trim().is_empty() {
                    self.log_info("Specify a Song Dooood".to_string());
                } else {
                    let add_result = self.player.add_track(&song_name);
                    self.info.push(add_result);
                }
                self.spl_sender.send(1).unwrap();
            }
            "resume" | "play" if command_vec.get(1).is_none() => {
                let play_result = self.player.play(false);
                self.log_info(play_result);
            }
            "play" | "jump"=> {
                let index = command_vec.get(1);
                if index.is_none() {
                    self.player.play(false);
                    self.log_info("Resumed!".to_string());
                } else {
                    let index = index.unwrap().parse::<usize>();
                    if index.is_err() {
                        self.log_info("Duh! Get the Index Right".to_string());
                    } else {
                        self.info.push(self.player.jump_song(index.unwrap()))
                    }
                }
            },
            "p" => self.player.pause_play(),
            "next" | "skip" => {
                let result = self.player.skip_track();
                self.log_info(result);
            }
            "pause" | "wait" | "hold" => {
                self.player.pause();
                self.log_info("Paused For Ya!".to_string())
            }
            "hello" | "hi" | "yo" => {
                self.log_info("Yoo Wassup (★ ω ★)".to_string());
            }
            "prev" | "back" | "revert" => {
                let result = self.player.prev_track();
                self.log_info(result);
            }
            "scan" | "fetch" => {
                self.player.scan_songs(command_vec.get(1));
            },
            other => self.log_info(format!("{}? What's that tho 😕", other)),
        }
        self.command.clear();
    }

    fn log_info(&mut self, message: String) {
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
                if index == self.player.current_song() as usize - 1 {
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
