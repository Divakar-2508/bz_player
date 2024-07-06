use std::{fmt::Display, sync::mpsc::Sender};
use rodio::{self, OutputStream, Sink};
use crate::error::PlayerError;
use crate::song::{Playlist, Song};

pub struct Player {
    queue: Vec<Song>,
    current_song: u32,
    sink: Sink,
    _output_stream: OutputStream,
    pub communicater: Sender<PlayerAction>,
}

#[derive(Debug)]
pub enum PlayerAction {
    ConnectionMessage(String),
}

impl Display for PlayerAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionMessage(message) => write!(f, "{}", message),
        }
    }
}

impl Player {
    pub fn new(sender: Sender<PlayerAction>) -> Self {
        let (_output_stream, output_stream_handle) = OutputStream::try_default().unwrap();
        let queue = Vec::new();
        let sender = sender;
        
        let sink = Sink::try_new(&output_stream_handle).unwrap();
    
        let player = Self {
            queue,
            current_song: 0,
            _output_stream,
            sink,
            communicater: sender,
        };
        player
    }

    pub fn add_track(&mut self, song: Song) -> Result<u32, PlayerError> {
        self.queue.push(song);
        if self.sink.empty() {
            self.play(true)?;
        }
        Ok(self.queue.len() as u32)
    }

    // pub fn add_track_multiple(&mut self, songs: Vec<Song>) -> Result<u32, PlayerError> {
        
    // }

    pub fn add_playlist(&mut self,mut playlist: Playlist) -> Result<u32, PlayerError> {
        self.queue.append(&mut playlist.songs);
        if self.sink.empty() {
            self.play(true)?;
        }
        Ok(self.queue.len() as u32)
    }

    pub fn play(&self, forced: bool) -> Result<u32, PlayerError> {
        if self.queue.is_empty() {
            return Err(PlayerError::EmptyQueue);
        }
        if forced {
            let song = self.queue.iter().nth(self.current_song as usize).unwrap();
            self.sink.clear();
            self.sink.append(song.get_source().unwrap());
            self.sink.play();
        } else {
            self.sink.play();
        }
        return Ok(self.current_song);
    }

    pub fn next_track(&mut self) -> Result<u32, PlayerError> {
        if (self.current_song + 1) as usize >= self.queue.len() {
            Err(PlayerError::IndexOutOfBounds)
        } else {
            self.current_song += 1;
            self.play(true)?;
            Ok(self.current_song)
        }
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn jump_track(&mut self, index: usize) -> Result<u32, PlayerError> {
        if index as usize >= self.queue.len() {
            return Err(PlayerError::IndexOutOfBounds);
        }
        self.current_song = index as u32;
        self.play(true)
    }

    pub fn get_song_detail(&self, index: usize) -> Result<String, PlayerError> {
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

    pub fn get_queue_ids(&self) -> Vec<u32> {
        self.queue
            .iter()
            .map(|song| song.song_id)
            .collect()
    }

    pub fn current_song(&self) -> u32 {
        self.current_song
    }

    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }
}