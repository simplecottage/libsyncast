use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
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

const FEED_CONF: &str = "feeds.txt";
const HISTORY_FILE: &str = "history.txt";

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
struct HistoryItem {
    title: String,
    url: String,
}

#[derive(Debug)]
struct FavoriteItem {
    title: String,
    url: String,
}

#[derive(Debug)]
struct AppState {
    folders: Vec<Folder>,
    selected_folder: usize,
    history: Vec<HistoryItem>,
    favorites: Vec<FavoriteItem>,
    show_history: bool,
    show_favorites: bool,
    selected_favorite: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app_state = AppState {
        folders: load_folders_conf()?,
        selected_folder: 0,
        history: load_history()?,
        favorites: load_favorites()?,
        show_history: false,
        show_favorites: false,
        selected_favorite: 0,
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

            if app_state.show_favorites {
                let favorite_items: Vec<ListItem> = app_state
                    .favorites
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let style = if i == app_state.selected_favorite {
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };
                        ListItem::new(Span::styled(format!("{} - {}", item.title, item.url), style))
                    })
                    .collect();

                let favorites_list = List::new(favorite_items)
                    .block(Block::default().borders(Borders::ALL).title("Favorites"));
                f.render_widget(favorites_list, chunks[0]);
            } else if app_state.show_history {
                let history_items: Vec<ListItem> = app_state
                    .history
                    .iter()
                    .map(|item| ListItem::new(format!("{} - {}", item.title, item.url)))
                    .collect();

                let history_list = List::new(history_items)
                    .block(Block::default().borders(Borders::ALL).title("History"));
                f.render_widget(history_list, chunks[0]);
            } else {
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
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if app_state.show_favorites {
                            if app_state.selected_favorite < app_state.favorites.len() - 1 {
                                app_state.selected_favorite += 1;
                            }
                        } else if !app_state.show_history
                            && app_state.selected_folder < app_state.folders.len() - 1
                        {
                            app_state.selected_folder += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app_state.show_favorites {
                            if app_state.selected_favorite > 0 {
                                app_state.selected_favorite -= 1;
                            }
                        } else if !app_state.show_history && app_state.selected_folder > 0 {
                            app_state.selected_folder -= 1;
                        }
                    }
                    KeyCode::Char('h') => {
                        app_state.show_history = !app_state.show_history;
                        app_state.show_favorites = false;
                    }
                    KeyCode::Char('F') => {
                        app_state.show_favorites = !app_state.show_favorites;
                        app_state.show_history = false;
                    }
                    KeyCode::Char('f') => {
                        if app_state.show_history {
                            if let Some(history_item) = app_state.history.get(app_state.selected_folder) {
                                save_to_favorites(&history_item.title, &history_item.url)?;
                                app_state.favorites.push(FavoriteItem {
                                    title: history_item.title.clone(),
                                    url: history_item.url.clone(),
                                });
                            }
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

fn load_history() -> Result<Vec<HistoryItem>, Box<dyn std::error::Error>> {
    let file = File::open(HISTORY_FILE).unwrap_or_else(|_| File::create(HISTORY_FILE).unwrap());
    let reader = BufReader::new(file);
    Ok(reader
        .lines()
        .filter_map(|line| {
            if let Ok(line) = line {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    Some(HistoryItem {
                        title: parts[0].to_string(),
                        url: parts[1].to_string(),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect())
}

fn load_favorites() -> Result<Vec<FavoriteItem>, Box<dyn std::error::Error>> {
    let file = File::open(FAVORITES_FILE).unwrap_or_else(|_| File::create(FAVORITES_FILE).unwrap());
    let reader = BufReader::new(file);
    Ok(reader
        .lines()
        .filter_map(|line| {
            if let Ok(line) = line {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    Some(FavoriteItem {
                        title: parts[0].to_string(),
                        url: parts[1].to_string(),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect())
}
