use crate::song::{Song, SongLakeChief};
use rodio::{Decoder, OutputStream, Sink};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::{
    fs::File,
    io::{self, BufReader},
    path::PathBuf,
    sync::mpsc::Sender,
};

fn get_source(path: PathBuf) -> Result<Decoder<BufReader<File>>, io::Error> {
    let file = File::open(path).unwrap();
    let buf_reader = BufReader::new(file);

    Ok(Decoder::new(buf_reader).unwrap())
}

pub struct Dj {
    queue: Vec<Song>,
    _output_stream: OutputStream,
    sink: Arc<Mutex<Sink>>,
    lake_chief: SongLakeChief,
    pub current_index: u8,
    communicator: Arc<Mutex<Sender<String>>>,
}

impl Dj {
    pub fn new(sender: Sender<String>, spl_receiver: Receiver<u8>) -> Self {
        let queue = Vec::new();
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Arc::new(Mutex::new(Sink::try_new(&stream_handle).unwrap()));
        let sink_clone = Arc::clone(&sink);

        let sender = Arc::new(Mutex::new(sender));
        let sender_clone = Arc::clone(&sender);
        thread::spawn(move || Dj::manage_playback(sink_clone, sender_clone, spl_receiver));
        Self {
            queue,
            _output_stream: stream,
            sink,
            lake_chief: SongLakeChief::new("song.db").unwrap(),
            current_index: 0,
            communicator: sender,
        }
    }

    pub fn get_queue(&self) -> Vec<&String> {
        self.queue.iter().map(|x| &x.song_name).collect()
    }

    pub fn play(&self, forced: bool) -> String {
        let sink = self.sink.lock().unwrap();
        if self.current_index == 0 {
            return "No Songs to play".to_string();
        }
        if !forced && sink.is_paused() {
            sink.play();
            return "Resumed".to_string();
        }
        sink.clear();
        let current_song = self.queue.get((self.current_index - 1) as usize).unwrap();
        sink.append(get_source(current_song.get_path()).unwrap());
        sink.play();

        format!("Playing {}", current_song.song_name.to_owned())
    }

    pub fn add_track(&mut self, song_name: &str) -> String {
        let song = self.lake_chief.get_song(song_name);
        match song {
            Ok(song) => {
                let song_name = song.song_name.to_owned();
                self.queue.push(song);
                if self.sink.lock().unwrap().empty() {
                    self.current_index += 1;
                    let play_result = self.play(true);
                    return play_result;
                }
                format!("Added {} to Queue", song_name)
            }
            Err(err) => err,
        }
    }

    pub fn skip_track(&mut self) -> String {
        if self.queue.is_empty() {
            return "Dude, add the songs first".to_string();
        }
        if (self.current_index + 1) as usize > self.queue.len() {
            return "Already at the last!".to_string();
        }
        self.sink.lock().unwrap().clear();
        self.current_index += 1;
        self.play(true)
    }

    pub fn prev_track(&mut self) -> String {
        if self.queue.is_empty() {
            return "Dude, add the songs first".to_string();
        }
        if self.current_index == 0 || self.current_index == 1 {
            return "Sry, At the Very First".to_string();
        }
        self.current_index -= 1;
        self.play(true)
    }

    pub fn pause(&self) {
        self.sink.lock().unwrap().pause();
    }

    pub fn scan_songs<S: ToString>(&mut self, path: Option<S>) {
        let mut song_details: Vec<(String, String)> = Vec::new();
            
    }

    pub fn current_song(&self) -> u8 {
        return self.current_index;
    }

    pub fn jump_song(&mut self, index: usize) -> String {
        if index <= self.queue.len() {
            self.current_index = index as u8;
            self.play(true)
        } else {
            "No Song At Given Position!".to_string()
        }
    }

    pub fn pause_play(&self) {
        let sink = self.sink.lock().unwrap();
        if sink.is_paused() {
            self.play(false);
        } else {
            sink.pause();
        }
    }

    pub fn manage_playback(sink: Arc<Mutex<Sink>>, communicator: Arc<Mutex<Sender<String>>>, spl_receiver: Receiver<u8>) {
        loop {
            spl_receiver.recv().unwrap();
            loop {
                thread::sleep(Duration::from_secs_f32(0.5));
                if sink.lock().unwrap().empty() {
                    communicator.lock().unwrap().send("101 Done".to_string()).unwrap();
                    break;
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}