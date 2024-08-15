use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{mpsc::Sender, Arc, Mutex},
    thread,
};

use crate::{
    error::{SongBaseError, SongError},
    player::PlayerAction,
    song::{Playlist, Song},
};
use rusqlite::{Connection, Error as rusqliteError, ErrorCode};

pub struct SongBase {
    conn: Arc<Mutex<Connection>>,
    sender: Sender<PlayerAction>,
}

impl SongBase {
    const INSERT_SONG_QUERY: &'static str =
        "INSERT INTO songs (song_name, song_path) VALUES (?1, ?2)";
    const PLAYLIST_SONG_REL_QUERY: &'static str = "INSERT INTO song_playlist_";
    pub fn init(db_name: &str, sender: Sender<PlayerAction>) -> Result<Self, SongBaseError> {
        let conn = Connection::open(db_name).map_err(|err| {
            if err.sqlite_error_code() == Some(ErrorCode::CannotOpen) {
                SongBaseError::AccessFailed
            } else {
                SongBaseError::DatabaseError(err.to_string())
            }
        })?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS songs(
                song_id INTEGER PRIMARY KEY AUTOINCREMENT,
                song_name TEXT UNIQUE NOT NULL,
                song_path TEXT NOT NULL
            )",
            (),
        )
        .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS playlists(
                playlist_id INTEGER PRIMARY KEY AUTOINCREMENT,
                playlist_name TEXT UNQIUE NOT NULL
            )",
            (),
        )
        .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS playlist_song_link(
                playlist_id INTEGER,
                song_id INTEGER,
                PRIMARY KEY (playlist_id, song_id),
                FOREIGN KEY (playlist_id) REFERENCES playlists(playlist_id) ON DELETE CASCADE,
                FOREIGN KEY (song_id) REFERENCES songs(song_id) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;

        let conn = Arc::new(Mutex::new(conn));
        Ok(Self { conn, sender })
    }

    pub fn find_song_by_name(&self, song_name: String) -> Result<Song, SongBaseError> {
        let pattern = format!("%{}%", song_name);

        let binding = self.conn.lock().unwrap();

        let mut fetch_query = binding
            .prepare(
                "SELECT * FROM songs WHERE 
            song_name LIKE ?1",
            )
            .unwrap();

        //Contains all result
        let mut fetch_result = fetch_query
            .query([pattern])
            .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;

        //get the first row
        let best_match = fetch_result
            .next()
            .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?
            .ok_or(SongBaseError::EntryNotFound)?;

        let song_id: u32 = best_match.get("song_id").unwrap();
        let song_name: String = best_match.get("song_name").unwrap();
        let song_path: String = best_match.get("song_path").unwrap();

        Ok(Song::new(song_id, song_name, song_path).unwrap())
    }

    fn find_song_by_id(&self, song_id: u32) -> Result<Song, SongBaseError> {
        let connection = self.conn.lock().unwrap();

        let mut song_query = connection
            .prepare(
                "  
            SELECT * FROM songs WHERE song_id = ?1",
            )
            .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;

        let song = song_query
            .query_row([song_id], |row| {
                let song_name: String = row.get("song_name")?;
                let song_path: String = row.get("song_path")?;
                Ok(Song::new(song_id, song_name, song_path))
            })
            .map_err(|err| match err {
                rusqliteError::QueryReturnedNoRows => SongBaseError::EntryNotFound,
                _ => SongBaseError::DatabaseError(err.to_string()),
            })?;

        match song {
            Ok(song) => Ok(song),
            Err(err) => {
                if let SongError::InvalidSongPath = err {
                    connection
                        .execute("DELETE FROM songs WHERE song_id = ?1", [song_id])
                        .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;
                    Err(SongBaseError::EntryNotFound)
                } else {
                    Err(SongBaseError::SongError(err))
                }
            }
        }
    }

    const RETRIEVE_ID_QUERY: &'static str =
        "SELECT song_id FROM songs WHERE song_name = ?1 AND song_path = ?2";
    pub fn create_song(
        conn: &mut Connection,
        song_name: &str,
        song_path: &str,
    ) -> Result<u32, SongBaseError> {
        match conn.execute(Self::INSERT_SONG_QUERY, (song_name, song_path)) {
            Err(err) if err.sqlite_error_code() != Some(ErrorCode::ConstraintViolation) => Err(SongBaseError::DatabaseError(err.to_string())),
            _ => {
                let mut query_statement = conn.prepare(Self::RETRIEVE_ID_QUERY)
                    .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;
                let mut query_result = query_statement.query([song_name, song_path])
                    .map_err(|err| SongBaseError::from(err))?;
                
                let song_row = query_result.next().map_err(|err| SongBaseError::from(err))?
                    .unwrap();
                
                Ok(song_row.get("song_id").unwrap())
            },
        }
    }

    pub fn filter_song(&self, song_name: &str) -> Result<Vec<(String, u32)>, SongBaseError> {
        let connection = self.conn.lock().unwrap();

        let search_query = "SELECT song_name, song_id FROM songs 
            WHERE song_name LIKE ?1";
        let pattern = format!("%{}%", song_name);

        let mut prepared_statement = connection
            .prepare(&search_query)
            .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;

        let query_result = prepared_statement
            .query_map([pattern], |row| {
                Ok((row.get("song_name")?, row.get("song_id")?))
            })
            .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;

        let mut songs = Vec::new();
        for song in query_result {
            if let Ok(song) = song {
                songs.push(song);
            } else {
                continue;
            }
        }
        Ok(songs)
    }

    pub fn create_playlist(&self, playlist_name: String) -> Result<u8, SongBaseError> {
        let connection = self.conn.lock().unwrap();
        let mut playlist_create_query = connection.prepare("INSERT INTO playlists (playlist_name) VALUES (?1)")
            .map_err(|err| SongBaseError::from(err))?;

        let mut row = playlist_create_query
            .query([playlist_name])
            .map_err(|err| {
                if err.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) {
                    SongBaseError::NameAlreadyExist
                } else {
                    SongBaseError::DatabaseError(err.to_string())
                }
            })?;
        self.sender.send(PlayerAction::ConnectionMessage(format!("{:?}", row.next()))).unwrap();
        let res = row.next().unwrap().unwrap();
        
        Ok(res.get("playlist_id").unwrap())
    }

    pub fn create_playlist_from_path(
        &self,
        playlist_name: String,
        folder_name: PathBuf,
    ) -> Result<(), SongBaseError> {
        let playlist_id = self.create_playlist(playlist_name)?;
        let mut conn = self.conn.lock().unwrap();
        let mut conn = conn.deref_mut();
        let song_ids: Vec<u32> = folder_name
            .read_dir()
            .map_err(|_| SongBaseError::AccessFailed)?
            .filter(|x| x.is_ok() && Song::is_valid_song_path(&x.as_ref().unwrap().path()))
            .map(|song| {
                let song = song.unwrap();
                self.sender
                    .send(PlayerAction::ConnectionMessage(format!(
                        "Added: {:?}",
                        song.file_name()
                    )))
                    .unwrap();
                Self::create_song(
                    &mut conn,
                    song.file_name().to_string_lossy().deref(),
                    song.path().to_string_lossy().deref(),
                ).unwrap()
            }).collect();
        self.add_playlist_song(playlist_id, song_ids)?;
        Ok(())
    }

    // pub fn get_playlists(&self) -> Result<Vec<String>, SongBaseError> {
    //     let conn = self.conn.lock().unwrap();
        
    // }

    pub fn get_playlist(&self, playlist_id: u8) -> Result<Playlist, SongBaseError> {
        let connection = self.conn.lock().unwrap();

        let playlist_name_query = "SELECT playlist_name FROM playlists
        WHERE playlist_id = ?1";

        let playlist_name: String = connection
            .query_row(&playlist_name_query, [playlist_id], |row| {
                row.get("playlist_name")
            })
            .map_err(|err| {
                if err.sqlite_error_code() == Some(ErrorCode::NotFound) {
                    SongBaseError::EntryNotFound
                } else {
                    SongBaseError::DatabaseError(err.to_string())
                }
            })?;

        let mut playlist_songs_query = connection
            .prepare(
                "SELECT song_id FROM playlist_song_link
            WHERE playlist_id=?1",
            )
            .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;

        let mut playlist = Playlist::new(playlist_name.clone());

        let song_ids = playlist_songs_query
            .query_map([playlist_id], |row| row.get("song_id"))
            .map_err(|err| SongBaseError::DatabaseError(err.to_string()))?;

        for song_id in song_ids {
            if let Ok(song_id) = song_id {
                match self.find_song_by_id(song_id) {
                    Ok(song) => playlist.add_song(song),
                    _ => continue,
                }
            }
        }
        self.sender
            .send(PlayerAction::ConnectionMessage(playlist_name))
            .unwrap();
        Ok(playlist)
    }

    pub fn add_playlist_song(
        &self,
        playlist_id: u8,
        song_ids: Vec<u32>,
    ) -> Result<(), SongBaseError> {
        let connection = self.conn.lock().unwrap();

        let playlist_song_add_query = "
            INSERT INTO playlist_song_relation (song_id, playlist_id) VALUES
            (?1, ?2)";

        for id in song_ids {
            match connection.execute(&playlist_song_add_query, [id, playlist_id as u32]) {
                Err(err) => {
                    if err.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) {
                        continue;
                    } else {
                        return Err(SongBaseError::DatabaseError(err.to_string()));
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

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

            thread::spawn(move || Self::fetch_songs(path_clone, &connection, &sender_clone));
            return Ok(format!("{}", path.to_string_lossy()));
        }
    }

    fn fetch_songs(path: PathBuf, conn: &Arc<Mutex<Connection>>, sender: &Sender<PlayerAction>) {
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
                        Self::fetch_songs(entry_path, &connection, sender);
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
                        Self::INSERT_SONG_QUERY,
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
