use ratatui::{buffer::Buffer, layout::{Alignment, Rect}, style::{Color, Style, Stylize}, text::Line, widgets::{Block, BorderType, Borders, Paragraph, Widget, Wrap}};

use crate::song::PlaylistActions;

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

pub fn search_song(rect: Rect, buf: &mut Buffer, song_list: &Vec<String>, song_name: &str) {
    let name = format!("SearchSong: {}", song_name);
    let block = render_block(&name);
    
    let lines: Vec<Line> = song_list
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

pub fn playlist(rect: Rect, buf: &mut Buffer, playlist_names: &Vec<String>) {
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