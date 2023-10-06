mod app;
mod args;
mod event;
mod handler;
mod tui_helper;
mod ui;

use std::process::exit;

use args::{Cli, Parser};
use simple_logger::SimpleLogger;

use crate::app::{App, AppResult};
use crate::event::{Event, EventHandler};
use crate::handler::handle_key_events;
use crate::tui_helper::Tui;
use std::io;
use tui::backend::CrosstermBackend;
use tui::Terminal;

fn main() -> AppResult<()> {
    let mut cli = Cli::parse();

    // Check if we need to print an example
    if cli.example {
        // The `include_str!` macro reads in the file and returns it as a
        // `&'static str` at compile time. Pretty cool.
        println!("\n{}", include_str!("../assets/example.moos"));
        return Ok(());
    }

    let level = match cli.verbose {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };

    let _ = SimpleLogger::new().with_level(level).init();

    if let (Some(_alias), Some(app_name)) = (&cli.alias, &cli.app_name) {
        log::warn!("Both `alias` and positional `app_name` are set. Using `app_name` for both.");
        cli.alias = Some(app_name.clone());
    } else if let Some(alias) = &cli.alias {
        cli.app_name = Some(alias.clone());
    } else if let Some(app_name) = &cli.app_name {
        cli.alias = Some(app_name.clone());
    }
    println!("Args: {:?}", cli);

    let mission = cli.mission_file.clone();
    let name = cli.alias.clone();

    // Need to create the application and call run
    // Move the cli into the application to be used further
    let mut app = App::new(cli);

    //app.run(mission, name)

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }
    }

    // Exit the user interface.
    tui.exit()
}
