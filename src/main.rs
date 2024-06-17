use std::io;

mod dj;
mod player_ui;
mod song;

fn main() -> io::Result<()> {
    let mut terminal = player_ui::init()?;
    let app_result = player_ui::App::new().run(&mut terminal);
    player_ui::restore()?;
    app_result
}

