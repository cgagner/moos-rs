use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::args::{Color as VarColor, Mode, ShowOptions};

/// Renders the user interface widgets.
pub fn render<B: Backend>(app: &mut App, frame: &mut Frame<'_, B>) {
    let mut columns = vec!["Name".to_owned()];

    let use_source = app.columns.contains(&ShowOptions::Source);
    let use_time = app.columns.contains(&ShowOptions::Time);
    let use_community = app.columns.contains(&ShowOptions::Community);

    if use_source {
        columns.push("Source".to_owned());
    }

    if use_time {
        columns.push("Time".to_owned());
    }

    if use_community {
        columns.push("Community".to_owned());
    }

    columns.push("Value".to_owned());

    let size = frame.size();
    let constraints = [Constraint::Length(0), Constraint::Min(0)];
    let layout = Layout::default()
        .constraints(constraints)
        .direction(Direction::Vertical)
        .split(size);

    let table = Table::new(
        app.data
            .iter()
            .skip(app.counter)
            .take(layout[1].height as usize)
            .enumerate()
            .map(|(_i, row)| {
                let color = match app.color_map.get(row.first().unwrap_or(&"").to_owned()) {
                    Some(VarColor::Blue) => Color::Blue,
                    Some(VarColor::Cyan) => Color::Cyan,
                    Some(VarColor::Green) => Color::Green,
                    Some(VarColor::Magenta) => Color::Magenta,
                    Some(VarColor::Red) => Color::Red,
                    _ => Style::default().fg.unwrap_or(Color::Reset),
                };

                let cells: Vec<Cell> = row
                    .iter()
                    .map(|cell| Cell::from(cell.to_string()))
                    .collect();
                Row::new(cells).style(Style::default().fg(color))
            })
            .collect::<Vec<Row>>(),
    )
    .header(
        Row::new(vec!["Name".to_string(), "Age".to_string()]).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .title(columns.join(","))
            .borders(Borders::ALL),
    )
    .widths(&[Constraint::Percentage(50), Constraint::Percentage(50)]);

    // Name, Source, Time, Community, Value

    frame.render_widget(table, layout[1]);
}
