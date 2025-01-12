use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write, Cursor},
    process::Command,
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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
struct Feed {
    url: String,
    items: Vec<RssItem>,
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
    selected_feed: usize,
    selected_item: usize,
    feed_scroll: usize,
    item_scroll: usize,
}

fn main() -> Result<()> {
    let mut feeds = load_feed_conf()?;

    let mut feed_data = Vec::new();
    for feed in &feeds {
        match fetch_and_parse_feed(feed) {
            Ok(parsed) => feed_data.push(parsed),
            Err(e) => eprintln!("error parsing feed {}: {}", feed, e),
        }
    }

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut selected_feed = 0usize;
    let mut selected_item = 0usize;

   'mainloop: loop {
        let current_folder = &mut app_state.folders[app_state.selected_folder];
        let feed_data = &current_folder.feeds;
    
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(70)
                ].as_ref())
                .split(size);
    
            // render folder list
            let folder_items: Vec<ListItem> = app_state.folders.iter()
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
                .block(Block::default().borders(Borders::ALL).title("folders"))
                .highlight_style(Style::default().fg(Color::Green));
            f.render_widget(folder_list, chunks[0]);
    
            // render feeds and items like before...
        })?;
    
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char("UP") => {
                        if app_state.selected_folder < app_state.folders.len() - 1 {
                            app_state.selected_folder += 1;
                            app_state.selected_feed = 0;
                        }
                    }
                    KeyCode::Char("DOWN") => {
                        if app_state.selected_folder > 0 {
                            app_state.selected_folder -= 1;
                            app_state.selected_feed = 0;
                        }
                    }
                    _ => {}
                }
            }
         }
       }
    }
    // exit
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn load_folders_conf() -> Result<Vec<Folder>> {
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
        if line.ends_with(":") {
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

fn save_folders_conf(folders: &[Folder]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(FEED_CONF)?;
    for folder in folders {
        writeln!(file, "{}:", folder.name)?;
        for feed in &folder.feeds {
            writeln!(file, "{}", feed)?;
        }
    }
    Ok(())
}

fn load_feed_conf() -> Result<Vec<String>> {
    let file = match File::open(FEED_CONF) {
        Ok(f) => f,
        Err(_) => {
            let mut f = File::create(FEED_CONF)?;
            f.write_all(b"https://example.com/rss\n")?;
            File::open(FEED_CONF)?
        }
    };

    let reader = BufReader::new(file);
    let mut feeds = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            feeds.push(line);
        }
    }
    Ok(feeds)
}

fn save_feed_conf(feeds: &[String]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(FEED_CONF)?;
    for feed in feeds {
        writeln!(file, "{}", feed)?;
    }
    Ok(())
}

fn fetch_and_parse_feed(url: &str) -> Result<Feed> {
    let resp = Client::new().get(url).send()?;
    let bytes = resp.bytes()?;
    let cursor = Cursor::new(bytes);
    let feed = parser::parse(cursor)?;
    let mut items = Vec::new();

    for entry in feed.entries {
        let title = entry.title.map(|t| t.content).unwrap_or_default();
        let description = entry.summary.map(|t| t.content).unwrap_or_default();
        let mp3_url = entry.links
            .iter()
            .find(|l| l.media_type.as_deref().unwrap_or("").contains("mpeg"))
            .map(|l| l.href.clone())
            .unwrap_or_default();
        items.push(RssItem {
            title,
            url: mp3_url,
            description,
        });
    }

    Ok(Feed {
        url: url.to_string(),
        items,
    })
}
