use std::io;

mod error;
mod player;
mod song;
mod song_base;
mod ui;
mod utility;

fn main() -> io::Result<()> {
    let mut terminal = ui::init()?;
    let app_result = ui::App::new().run(&mut terminal);
    ui::restore()?;
    app_result
}
