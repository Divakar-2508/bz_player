use std::{fs::File, io::BufReader, path::{Path, PathBuf}};

use rodio::Decoder;

use crate::error::SongError;

#[derive(Debug)]
pub enum Playable {
    Song(String),
    Playlist(u8),
    None,
    Whole
}

impl std::fmt::Display for PlaylistActions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaylistActions::Show => write!(f, "Playlist Show"),
            PlaylistActions::View(playlist_id) => write!(f, "Playlist View {:?}", playlist_id),
            PlaylistActions::Create(playlist_name, path) => write!(f, "Playlist Create {:?} {:?}", playlist_name, path),
            PlaylistActions::Add(id, songs) => write!(f, "Playlist Add {:?} {:?}", id, songs),
            PlaylistActions::AddAll(id) => write!(f, "Playlist add * {:?}", id),
            PlaylistActions::Invalid => write!(f, "Playlist Invalid"),
        }
    }
}


pub enum PlaylistActions {
    Show,
    View(Option<u8>),
    Create(Option<String>, Option<PathBuf>),
    Add(u8, Option<Vec<String>>),
    AddAll(Option<u8>),
    Invalid,
}

#[derive(Debug)]
pub struct Song {
    pub song_id: u32,
    pub song_name: String,
    pub song_path: PathBuf,
}

impl Song {
    pub fn new<S: AsRef<str> + ToString>(id: u32, song_name: S, song_path: S) -> Result<Self, SongError> {
        let path_check = PathBuf::from(song_path.as_ref());
        if !path_check.exists() {
            return Err(SongError::InvalidSongPath);
        }
        
        if !Self::is_valid_song_path(&path_check) {
            return Err(SongError::InvalidSongFormat);
        }

        Ok(Self {
            song_id: id,
            song_name: song_name.to_string(),
            song_path: path_check,
        })
    }

    pub fn get_source(&self) -> Result<Decoder<BufReader<File>>, SongError> {
        let file = File::open(self.song_path.as_path());
        if file.is_err() {
            return Err(SongError::SongAccessError)
        }
        let reader = BufReader::new(file.unwrap());

        Ok(Decoder::new(reader).unwrap())
    }

    fn is_valid_song_path(path: &Path) -> bool {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("mp3") | Some("ogg") | Some("wav") => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct Playlist {
    pub playlist_name: String,
    pub songs: Vec<Song>
}

impl Playlist {
    pub fn new<S: ToString>(playlist_name: S) -> Self {
        Self {
            playlist_name: playlist_name.to_string(),
            songs: Vec::new()
        }
    }

    pub fn add_song(&mut self, song: Song) {
        self.songs.push(song)
    }
}