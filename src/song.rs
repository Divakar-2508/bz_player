use std::{fs::File, io::BufReader, path::{Path, PathBuf}};

use rodio::Decoder;

use crate::error::SongError;

#[derive(Debug)]
pub struct Song {
    pub song_name: String,
    pub song_path: PathBuf,
}

impl Song {
    pub fn new<S: AsRef<str> + ToString>(song_name: S, song_path: S) -> Result<Self, SongError> {
        let path_check = PathBuf::from(song_path.as_ref());
        if !path_check.exists() {
            return Err(SongError::InvalidSongPath);
        }
        
        if !Self::is_valid_song_path(&path_check) {
            return Err(SongError::InvalidSongFormat);
        }

        Ok(Self {
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

    pub fn return_values(&self) -> (&str, &str) {
        (&self.song_name, &self.song_path.to_str().unwrap())
    }

    pub fn get_path(&self) -> PathBuf {
        self.song_path.clone()
    }

    fn is_valid_song_path(path: &Path) -> bool {
        let extension = path.extension();
        if extension.is_none() {
            return false;
        }   
        match extension.unwrap().to_string_lossy().to_lowercase().as_str() {
            "mp3" | "ogg" | "wav" => true,
            _ => false,
        }
    }
}