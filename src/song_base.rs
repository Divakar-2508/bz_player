use std::{
    path::PathBuf,
    sync::{mpsc::Sender, Arc, Mutex},
    thread,
};

use crate::{error::SongBaseError, player::PlayerAction, song::Song};
use rusqlite::{Connection, ErrorCode};

pub struct SongBase {
    conn: Arc<Mutex<Connection>>,
    sender: Sender<PlayerAction>,
}

impl SongBase {
    pub fn new(db_name: &str, sender: Sender<PlayerAction>) -> Result<Self, String> {
        let conn = Connection::open(db_name);
        if let Err(conn) = conn.as_ref() {
            if conn.sqlite_error_code() == Some(ErrorCode::CannotOpen) {
                return Err("Can't Open the Database".to_string());
            }
        }

        let conn = conn.unwrap();

        let create_table_songs = conn.execute(
            "CREATE TABLE IF NOT EXISTS songs(
                song_name TEXT PRIMARY KEY NOT NULL,
                song_path TEXT NOT NULL
            )",
            (),
        );

        // let create_table_playlist = conn.execute(
        //     "CREATE TABLE playlists
        //     playlist_id ", params)

        let conn = Arc::new(Mutex::new(conn));
        match create_table_songs {
            Err(_) => Err("Can't Create Song Base".to_string()),
            _ => Ok(Self { conn, sender }),
        }
    }

    pub fn get_song(&self, song_name: String) -> Result<Song, SongBaseError> {
        let pattern = format!("%{}%", song_name);
        // self.sender.send(Pla)
        let binding = self
            .conn
            .lock()
            .unwrap();

        let mut fetch_query = binding
            .prepare(
                "SELECT * FROM songs WHERE 
            song_name LIKE ?1",
            )
            .unwrap();

        //Contains all result
        let mut fetch_result = fetch_query.query([pattern])
            .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;

        //get the first row
        let best_match = fetch_result.next()
            .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?
            .ok_or(SongBaseError::EntryNotFound)?;
     
        let song_name: String = best_match.get("song_name").unwrap();
        let song_path: String = best_match.get("song_path").unwrap();

        Ok(Song::new(song_name, song_path).unwrap())
    }

    // pub fn get_playlist(&self, playlist_name: String) -> Vec<Song> {

    // }

    // const DEFAULT_DIRECTORIES: &str =
    pub fn scan_songs(&self, path: Option<String>) -> Result<String, SongBaseError> {
        let path = match path {
            Some(p) => PathBuf::from(p),
            None => match dirs::audio_dir() {
                Some(p) => p,
                None => return Err(SongBaseError::InvalidPath),
            },
        };

        if !path.exists() {
            return Err(SongBaseError::InvalidPath);
        } else {
            let connection = Arc::clone(&self.conn);
            let sender_clone = self.sender.clone();
            let path_clone = path.clone();

            thread::spawn(move || Self::fetch_songs(path_clone, connection, &sender_clone));
            return Ok(format!("{}", path.to_string_lossy()));
        }
    }

    const INSERT_QUERY: &'static str = "INSERT INTO songs (song_name, song_path) VALUES (?1, ?2)";
    fn fetch_songs(path: PathBuf, conn: Arc<Mutex<Connection>>, sender: &Sender<PlayerAction>) {
        let read_dir = path.read_dir();
        if read_dir.is_err() {
            let message = format!("Can't read dir: {}", path.to_str().unwrap());
            sender
                .send(PlayerAction::ConnectionMessage(message))
                .unwrap();
            return;
        }

        for entry in read_dir.unwrap() {
            if entry.is_err() {
                continue;
            }
            let entry = entry.unwrap();
            let entry_path = entry.path();

            if entry_path.is_dir() {
                let dir_name = entry_path.file_name();
                match dir_name {
                    Some(dir_name) if dir_name != "node_modules" || dir_name != "target" => {
                        let connection = Arc::clone(&conn);
                        Self::fetch_songs(entry_path, connection, sender);
                    }
                    _ => continue,
                }
                continue;
            }

            let file_extension = entry_path.extension();
            match file_extension {
                Some(ext) if matches!(ext.to_str().unwrap(), "ogg" | "wav" | "mp3") => {
                    let file_name = entry_path.file_name();
                    if file_name.is_none() {
                        continue;
                    }
                    let file_name = file_name.unwrap().to_str().unwrap();

                    let execute_query = conn.lock().unwrap().execute(
                        Self::INSERT_QUERY,
                        (file_name, entry_path.to_str().unwrap()),
                    );
                    if let Err(err) = execute_query {
                        if let Some(ErrorCode::ConstraintViolation) = err.sqlite_error_code() {
                            continue;
                        } else {
                            let message = format!("Database error: {}", err.to_string());
                            sender
                                .send(PlayerAction::ConnectionMessage(message))
                                .map_err(|_| {})
                                .unwrap();
                        }
                    } else {
                        let message = format!("Added {}", file_name);
                        sender
                            .send(PlayerAction::ConnectionMessage(message))
                            .map_err(|_| {})
                            .unwrap();
                    }
                }
                _ => continue,
            }
        }
    }
}
