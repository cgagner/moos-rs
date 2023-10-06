use crate::app::{App, AppResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') if key_event.modifiers == KeyModifiers::CONTROL => {
            app.quit();
        }
        // Counter handlers
        KeyCode::Down => {
            app.increment_counter();
        }
        KeyCode::Up => {
            app.decrement_counter();
        }
        KeyCode::Char('s') | KeyCode::Char('S') => app.toggle_source_column(),
        KeyCode::Char('c') | KeyCode::Char('C') => app.toggle_community_column(),
        KeyCode::Char('t') | KeyCode::Char('T') => app.toggle_time_column(),
        //KeyCode::Char('a') | KeyCode::Char('A') => app.toggle_aux_column(),
        // Other handlers you could add here.
        _ => {}
    }
    Ok(())
}
