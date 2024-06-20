use std::{fmt::Display, sync::{mpsc::Sender, Arc, Mutex}, thread::{self, JoinHandle}, time::Duration};
use rodio::{self, OutputStream, Sink};
use crate::error::PlayerError;
use crate::song::Song;

pub struct Player {
    queue: Vec<Song>,
    current_song: u32,
    sink: Arc<Mutex<Sink>>,
    _output_stream: OutputStream,
    pub communicater: Sender<PlayerAction>,
    playback_handle: JoinHandle<()>
}

#[derive(Debug)]
pub enum PlayerAction {
    NextSong,
    Playing(String),
    ConnectionMessage(String),
}

impl Display for PlayerAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionMessage(message) => write!(f, "{}", message),
            Self::NextSong => write!(f, "Skipped"),
            Self::Playing(song) => write!(f, "Now Playing {}", song)
        }
    }
}

impl Player {
    pub fn new(sender: Sender<PlayerAction>) -> Self {
        let (_output_stream, output_stream_handle) = OutputStream::try_default().unwrap();
        let queue = Vec::new();
        let sender = sender;
        
        let sink = Sink::try_new(&output_stream_handle).unwrap();
        let sink = Arc::new(Mutex::new(sink));
        let sink_clone = Arc::clone(&sink);

        let sender_clone = sender.clone();
        let playback_handle = thread::spawn(move || {
            Self::handle_playback(sink_clone, sender_clone);
        });
        let player = Self {
            queue,
            current_song: 0,
            _output_stream,
            sink,
            communicater: sender,
            playback_handle
        };
        player
    }

    pub fn add_track(&mut self, song: Song) -> Result<u32, PlayerError> {
        self.queue.push(song);
        if self.sink.lock().unwrap().empty() {
            self.play(true);
        }
        Ok(self.current_song)
    }

    pub fn play(&self, forced: bool) -> Result<u32, PlayerError> {
        let sink = self.sink.lock().unwrap();
        if self.queue.is_empty() {
            return Err(PlayerError::EmptyQueue);
        }
        if forced {
            let song = self.queue.iter().nth(self.current_song as usize).unwrap();
            sink.clear();
            sink.append(song.get_source().unwrap());
        } else {
            sink.play();
        }
        return Ok(self.current_song);
    }

    pub fn next_track(&mut self) -> Result<u32, PlayerError> {
        if (self.current_song + 1) as usize > self.queue.len() {
            Err(PlayerError::IndexOutOfBounds)
        } else {
            self.current_song += 1;
            Ok(self.current_song)
        }
    }

    pub fn get_song_detail(&self, index: u32) -> Result<String, PlayerError> {
        let index = (index - 1) as usize;
        if index >= self.queue.len() {
            Err(PlayerError::IndexOutOfBounds)
        } else {
            let song = self.queue.iter().nth(index).unwrap();
            Ok(song.song_name.clone())
        }
    }

    pub fn get_queue(&self) -> Vec<&String> {
        self.queue
            .iter()
            .map(|song| &song.song_name)
            .collect()
    }

    pub fn current_song(&self) -> u32 {
        self.current_song
    }

    fn handle_playback(sink: Arc<Mutex<Sink>>, sender: Sender<PlayerAction>) {
        loop {
            thread::park();
            loop {
                thread::sleep(Duration::from_secs_f32(0.5));
                let sink = sink.lock().unwrap();
                if sink.empty() {
                    sink.skip_one();
                    sender.send(PlayerAction::NextSong).unwrap();
                    break;
                }
            }
        }
    }
}