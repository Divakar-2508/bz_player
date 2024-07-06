use ratatui::{buffer::Buffer, layout::{Alignment, Rect}, style::{Color, Style, Stylize}, text::Line, widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap}};

use crate::{error::SongBaseError, song::PlaylistActions};

pub enum UtilityState {
    Playlist(PlaylistActions),
    SearchSong(String),
    Joke,
    Question,
    Help
}

fn render_block<'a>(name: &str) -> Block<'a> {
    Block::default()
        .title(format!(" {} ", name).fg(Color::Red))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White))
}

pub fn render_search_song(rect: Rect, buf: &mut Buffer, song_list: Result<&Vec<(String, u32)>, &SongBaseError>, song_name: &str) {
    let name = format!("SearchSong: {}", song_name);
    let block = render_block(&name);
    
    if song_list.is_err() {
        let line = Line::raw(format!("Can't get the data, but got an error: {:?}", song_list.err().take().unwrap().to_string()));
        Paragraph::new(line)
            .centered()
            .block(block)
            .wrap(Wrap { trim: true })
            .render(rect, buf);
        return; 
    }

    let song_list = song_list.unwrap();
    if song_list.is_empty() {
        let line = vec![Line::raw(""), Line::raw(format!("That's Empty!, gambare gambare"))];
        Paragraph::new(line)
            .centered()
            .block(block)
            .wrap(Wrap { trim: true })
            .render(rect, buf);
        return;
    }

    let mut lines: Vec<Line> = song_list
        .iter()
        .enumerate()
        .map(|(index, song)| Line::raw(format!("{}. {} ({})", index, song.0, song.1)))
        .collect();
    lines.insert(0, Line::raw(""));

    let para = Paragraph::new(lines)
        .left_aligned()
        .block(block)
        .wrap(Wrap { trim: true });

    para.render(rect, buf);
}

pub fn render_playlist_view(rect: Rect, buf: &mut Buffer, playlist_names: &Vec<String>) {
    let block = render_block("Playlist");

    let lines: Vec<Line> = playlist_names
        .iter()
        .enumerate()
        .map(|(index, song)| Line::raw(format!("{}. {}", index, song)))
        .collect();

    let para = Paragraph::new(lines)
        .left_aligned()
        .block(block)
        .wrap(Wrap { trim: true });

    para.render(rect, buf);
}