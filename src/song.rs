use rusqlite::{Connection, ErrorCode};
use std::{io, path::PathBuf, sync::mpsc::Sender};

#[derive(Debug)]
pub struct Song {
    pub song_name: String,
    pub song_path: String,
}

impl Song {
    pub fn new<S: AsRef<str> + ToString>(song_name: S, song_path: S) -> Result<Self, io::Error> {
        let path_check = PathBuf::from(song_path.as_ref());
        if !path_check.exists() {
            return Err(io::ErrorKind::NotFound.into());
        }

        Ok(Self {
            song_name: song_name.to_string(),
            song_path: song_path.to_string(),
        })
    }

    fn return_values(&self) -> (&str, &str) {
        (&self.song_name, &self.song_path)
    }

    pub fn get_path(&self) -> PathBuf {
        PathBuf::from(&self.song_path)
    }
}

pub struct SongLakeChief {
    conn: Connection,
}

impl SongLakeChief {
    pub fn new(db_name: &str) -> Result<Self, String> {
        let conn = Connection::open(db_name);
        if let Err(conn) = conn.as_ref() {
            if conn.sqlite_error_code() == Some(ErrorCode::CannotOpen) {
                return Err("Can't Open the Database".to_string());
            }
        }

        let conn = conn.unwrap();
        let create_table = conn.execute(
            "CREATE TABLE IF NOT EXISTS songs(
                song_name TEXT PRIMARY KEY NOT NULL,
                song_path TEXT NOT NULL
            )",
            (),
        );

        match create_table {
            Err(_) => Err("Can't Create Song Lake".to_string()),
            _ => Ok(Self { conn }),
        }
    }

    pub fn batch_insert_songs(
        &mut self,
        song_details: Vec<(String, String)>,
        sender: &mut Sender<String>
    ) -> usize {
        let conn = &mut self.conn;
        let sql_tx = conn.transaction().map_err(|e| e.to_string());
        if let Err(err) = sql_tx.as_ref() {
            sender.send(err.to_owned()).map_err(|_| {}).unwrap();
        }
        let sql_tx = sql_tx.unwrap();
        let mut add_count = 0;
        for (song_name, song_path) in song_details {
            let song = Song::new(song_name, song_path);
            if let Err(_) = song.as_ref() {
                continue;
            }

            let song = song.unwrap();
            let exe_result = sql_tx.execute(
                "INSERT INTO songs (song_name, song_path) 
                values (?1, ?2)",
                song.return_values(),
            );

            if let Err(error) = exe_result {
                if error.sqlite_error_code().unwrap().eq(&ErrorCode::ConstraintViolation) {
                    continue;
                } else {
                    sender.send(format!("Can't Insert Data for {}", song.song_name)).map_err(|_| {}).unwrap();
                }
            } else {
                add_count += 1;
                sender.send(format!("> Found and added `{}`", song.song_name)).map_err(|_| {}).unwrap();
            }
        }

        sql_tx.commit().map_err(|e| sender.send(e.to_string())).unwrap();
        add_count
    }

    pub fn get_song(&self, song_name: &str) -> Result<Song, String> {
        let pattern = format!("%{}%", song_name);

        let mut fetch_query = self
            .conn
            .prepare(
                "SELECT * FROM songs WHERE 
            song_name LIKE ?1",
            )
            .unwrap();
        let fetch_result = fetch_query.query([pattern]);
        if let Err(err) = fetch_result {
            return Err(err.to_string());
        }

        let mut fetch_result = fetch_result.unwrap();
        let best_match = fetch_result.next().unwrap();
        if best_match.is_none() {
            return Err("404 Song Not Found :(".to_string());
        }

        let best_match = best_match.unwrap();

        let song_name: String = best_match.get(0).unwrap();
        let song_path: String = best_match.get(1).unwrap();

        Ok(Song::new(song_name, song_path).unwrap())
    }
}


/*
  pub fn insert_song<S: ToString + AsRef<str>>(
        &self,
        song_name: S,
        song_path: S, 
    ) -> Result<(), String> {
        let song = Song::new(song_name, song_path);
        if let Err(_) = song.as_ref() {
            return Err("Invalid Song Details".to_string());
        }

        let song = song.unwrap();
        let exe_result = self.conn.execute(
            "INSERT INTO songs (song_name, song_path) 
                values (?1, ?2)",
            song.return_values(),
        );

        if let Err(_) = exe_result {
            return Err("Can't Insert Data in Db".to_string());
        }

        Ok(())
    }

*/