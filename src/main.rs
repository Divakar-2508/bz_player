use std::io;

mod player;
mod ui;
mod song_base;
mod error;
mod song;

fn main() -> io::Result<()> {
    let mut terminal = ui::init()?;
    let app_result = ui::App::new().run(&mut terminal);
    ui::restore()?;
    app_result
}

