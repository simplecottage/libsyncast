use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write, Cursor},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use reqwest::blocking::Client;
use feed_rs::parser;

const FEED_CONF: &str = "feeds.txt";

#[derive(Debug)]
struct RssItem {
    title: String,
    url: String,
    description: String,
}

#[derive(Debug)]
struct Folder {
    name: String,
    feeds: Vec<String>,
}

#[derive(Debug)]
struct AppState {
    folders: Vec<Folder>,
    selected_folder: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app_state = AppState {
        folders: load_folders_conf()?,
        selected_folder: 0,
    };

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    'mainloop: loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(size);

            let folder_items: Vec<ListItem> = app_state
                .folders
                .iter()
                .enumerate()
                .map(|(i, folder)| {
                    let style = if i == app_state.selected_folder {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Span::styled(folder.name.clone(), style))
                })
                .collect();

            let folder_list = List::new(folder_items)
                .block(Block::default().borders(Borders::ALL).title("Folders"));
            f.render_widget(folder_list, chunks[0]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if app_state.selected_folder < app_state.folders.len() - 1 {
                            app_state.selected_folder += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app_state.selected_folder > 0 {
                            app_state.selected_folder -= 1;
                        }
                    }
                    KeyCode::Char('q') => break 'mainloop,
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn load_folders_conf() -> Result<Vec<Folder>, Box<dyn std::error::Error>> {
    let file = match File::open(FEED_CONF) {
        Ok(f) => f,
        Err(_) => {
            let mut f = File::create(FEED_CONF)?;
            f.write_all(b"default_folder:\nhttps://example.com/rss\n")?;
            File::open(FEED_CONF)?
        }
    };

    let reader = BufReader::new(file);
    let mut folders = Vec::new();
    let mut current_folder: Option<Folder> = None;

    for line in reader.lines() {
        let line = line?;
        if line.ends_with(':') {
            if let Some(folder) = current_folder.take() {
                folders.push(folder);
            }
            current_folder = Some(Folder {
                name: line.trim_end_matches(':').to_string(),
                feeds: Vec::new(),
            });
        } else if let Some(folder) = &mut current_folder {
            if !line.trim().is_empty() {
                folder.feeds.push(line);
            }
        }
    }
    if let Some(folder) = current_folder {
        folders.push(folder);
    }

    Ok(folders)
}
