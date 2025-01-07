use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write, Cursor},
    process::Command,
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span},
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
use anyhow::{Result, anyhow};

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
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(70)
                ].as_ref())
                .split(size);

            let feed_items: Vec<ListItem> = feed_data.iter()
                .enumerate()
                .map(|(i, feed)| {
                    let title_style = if i == selected_feed {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Span::styled(feed.url.to_string(), title_style))
                })
                .collect();

            let feed_list = List::new(feed_items)
                .block(Block::default().borders(Borders::ALL).title("feeds"));

            f.render_widget(feed_list, chunks[0]);

            if let Some(feed) = feed_data.get(selected_feed) {
                let item_list: Vec<ListItem> = feed.items.iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let style = if i == selected_item {
                            Style::default().fg(Color::Green)
                        } else {
                            Style::default()
                        };
                        ListItem::new(Span::styled(item.title.clone(), style))
                    })
                    .collect();

                let items_widget = List::new(item_list)
                    .block(Block::default().borders(Borders::ALL).title("episodes"));

                f.render_widget(items_widget, chunks[1]);
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        break 'mainloop;
                    }
                    KeyCode::Down => {
                        if let Some(feed) = feed_data.get(selected_feed) {
                            if selected_item < feed.items.len().saturating_sub(1) {
                                selected_item += 1;
                            }
                        }
                    }
                    KeyCode::Up => {
                        if selected_item > 0 {
                            selected_item -= 1;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(feed) = feed_data.get(selected_feed) {
                            if let Some(item) = feed.items.get(selected_item) {
                                Command::new("mpv")
                                    .arg(&item.url)
                                    .spawn()
                                    .map_err(|e| eprintln!("failed to play: {}", e))
                                    .ok();
                            }
                        }
                    }
                    _ => {}
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
