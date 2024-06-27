use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum SongError {
    InvalidSongPath,
    InvalidSongFormat,
    SongAccessError,
}

impl Display for SongError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::InvalidSongPath => write!(f, "Song Path Cannot be Found"),
            Self::InvalidSongFormat => write!(f, "Only ogg, wav, mp3 are supported"),
            Self::SongAccessError => write!(f, "No Access to Song File"),
        }
    }
}

#[derive(Debug)]
pub enum PlayerError {
    SongError(SongError),
    LastSong,
    EmptyQueue,
    IndexOutOfBounds,
}

impl Display for PlayerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::SongError(song_err) => write!(f, "{}", song_err),
            Self::LastSong => write!(f, "No More Song in the Queue"),
            Self::EmptyQueue => write!(f, "Queue is Empty"),
            Self::IndexOutOfBounds => write!(f, "Given Song Index is Invalid"),
        }
    }
}

#[derive(Debug)]
pub enum SongBaseError {
    EntryNotFound,
    AccessFailed,
    SongError(SongError),
    InvalidPath,
    DatabaseError(String),
}

impl Display for SongBaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::AccessFailed => write!(f, "Cannot Access the Database"),
            Self::EntryNotFound => write!(f, "Entry Not Found"),
            Self::SongError(err) => write!(f, "{}", err),
            Self::InvalidPath => write!(f, "The given path does not exists"),
            Self::DatabaseError(err) => write!(f, "Database error: {}", err),
        }
    }
}
