use crate::error::PlayerError;
use crate::song::{Playlist, Song};
use rodio::{self, OutputStream, Sink};
use std::{fmt::Display, sync::mpsc::Sender};

pub struct Player {
    queue: Vec<Song>,
    current_song: u32,
    sink: Sink,
    _output_stream: OutputStream,
    communicater: Sender<PlayerAction>,
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

        Self {
            queue,
            current_song: 0,
            _output_stream,
            sink,
            communicater: sender,
        }
    }

    pub fn add_track(&mut self, song: Song) -> Result<u32, PlayerError> {
        self.queue.push(song);
        if self.sink.empty() {
            if self.current_song != 0 {
                return self.next_track();
            } else {
                return self.play(true);
            }
        }
        Ok(self.queue.len() as u32)
    }

    pub fn clear_tracks(&mut self) {
        self.queue.clear();
    }

    pub fn remove_track(&mut self, song_id: usize) -> Result<String, PlayerError> {
        if song_id == 0 || song_id > self.queue.len() {
            return Err(PlayerError::IndexOutOfBounds);
        }

        let removed_song = self.queue.remove(song_id - 1);

        if self.queue.is_empty() {
            self.sink.clear();
            return Ok(format!(
                "Removed {} from queue! It's Empty Now!",
                removed_song.song_name
            ));
        }

        if self.current_song as usize > song_id - 1 {
            self.current_song -= 1;
        }

        self.play(true).map(|index| {
            format!(
                "Removed {}, Now Playing {} @ {}",
                removed_song.song_name,
                self.current_song_name(),
                index
            )
        })
    }

    pub fn add_playlist(&mut self, mut playlist: Playlist) -> Result<u32, PlayerError> {
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
            let song = self.queue.get(self.current_song as usize).unwrap();
            self.sink.clear();
            self.sink.append(song.get_source().unwrap());
            self.sink.play();
        } else {
            self.sink.play();
        }
        Ok(self.current_song)
    }

    pub fn toggle_player(&self) -> String {
        if self.sink.is_paused() {
            self.sink.play();
            "player resumed".to_string()
        } else {
            self.sink.pause();
            "player paused".to_string()
        }
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
        if index >= self.queue.len() {
            return Err(PlayerError::IndexOutOfBounds);
        }
        self.current_song = index as u32;
        self.play(true)
    }

    pub fn get_song_detail(&self, index: usize) -> Result<String, PlayerError> {
        if index >= self.queue.len() {
            Err(PlayerError::IndexOutOfBounds)
        } else {
            let song = self.queue.get(index).unwrap();
            Ok(song.song_name.clone())
        }
    }

    pub fn get_queue(&self) -> Vec<&String> {
        self.queue.iter().map(|song| &song.song_name).collect()
    }

    pub fn get_queue_ids(&self) -> Vec<u32> {
        self.queue.iter().map(|song| song.song_id).collect()
    }

    pub fn current_song(&self) -> u32 {
        self.current_song
    }

    pub fn current_song_name(&self) -> String {
        self.queue
            .get(self.current_song as usize)
            .unwrap()
            .song_name
            .clone()
    }

    pub fn is_sink_empty(&self) -> bool {
        self.sink.empty()
    }

    pub fn is_last(&self) -> bool {
        (self.current_song) as usize == self.queue.len()
    }
}
