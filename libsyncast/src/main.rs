use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
};

mod ui;
use ui::UI;

const FEED_CONF: &str = "feeds.txt";
const HISTORY_FILE: &str = "history.txt";
const FAVORITES_FILE: &str = "favorites.txt";

#[derive(Debug)]
struct RssItem {
    title: String,
    url: String,
    description: String,
}

#[derive(Debug)]
pub struct Folder {
    name: String,
    feeds: Vec<String>,
}

#[derive(Debug)]
pub struct HistoryItem {
    title: String,
    url: String,
}

#[derive(Debug)]
pub struct FavoriteItem {
    title: String,
    url: String,
}

#[derive(Debug)]
pub struct AppState {
    pub folders: Vec<Folder>,
    pub selected_folder: usize,
    pub history: Vec<HistoryItem>,
    pub favorites: Vec<FavoriteItem>,
    pub show_history: bool,
    pub show_favorites: bool,
    pub selected_favorite: usize,
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

    // Initialize UI
    let mut ui = UI::new()?;

    'mainloop: loop {
        // Draw UI based on current state
        ui.draw(&app_state)?;

        // Handle user input
        // Returns true if user wants to quit
        if ui.handle_events(&mut app_state)? {
            break 'mainloop;
        }

        // Handle specific application actions
        if app_state.show_history && ui.handle_favorite_action(&mut app_state)? {
            // Handle adding an item to favorites
            if let Some(history_item) = app_state.history.get(app_state.selected_folder) {
                save_to_favorites(&history_item.title, &history_item.url)?;
                app_state.favorites.push(FavoriteItem {
                    title: history_item.title.clone(),
                    url: history_item.url.clone(),
                });
            }
        }
    }

    // Clean up UI
    ui.cleanup()?;
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

fn save_to_favorites(title: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(FAVORITES_FILE)?;
    
    writeln!(file, "{} {}", title, url)?;
    Ok(())
}
