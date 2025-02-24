use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem},
    Terminal, Frame,
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::io;
use std::time::Duration;

// Import types from main app
use crate::{AppState, Folder, HistoryItem, FavoriteItem};

pub struct UI {
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
}

impl UI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    pub fn draw(&mut self, app_state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.draw(|f| {
            self.render_ui(f, app_state);
        })?;
        Ok(())
    }

    fn render_ui(&self, f: &mut Frame<CrosstermBackend<std::io::Stdout>>, app_state: &AppState) {
        let size = f.size();
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .split(size);

        if app_state.show_favorites {
            self.render_favorites(f, app_state, chunks[0]);
        } else if app_state.show_history {
            self.render_history(f, app_state, chunks[0]);
        } else {
            self.render_folders(f, app_state, chunks[0]);
        }

        // Additional UI rendering can go here for the right panel
    }

    fn render_favorites(
        &self,
        f: &mut Frame<CrosstermBackend<std::io::Stdout>>,
        app_state: &AppState,
        area: ratatui::layout::Rect,
    ) {
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
        f.render_widget(favorites_list, area);
    }

    fn render_history(
        &self,
        f: &mut Frame<CrosstermBackend<std::io::Stdout>>,
        app_state: &AppState,
        area: ratatui::layout::Rect,
    ) {
        let history_items: Vec<ListItem> = app_state
            .history
            .iter()
            .map(|item| ListItem::new(format!("{} - {}", item.title, item.url)))
            .collect();

        let history_list = List::new(history_items)
            .block(Block::default().borders(Borders::ALL).title("History"));
        f.render_widget(history_list, area);
    }

    fn render_folders(
        &self,
        f: &mut Frame<CrosstermBackend<std::io::Stdout>>,
        app_state: &AppState,
        area: ratatui::layout::Rect,
    ) {
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
        f.render_widget(folder_list, area);
    }

    pub fn handle_events(&self, app_state: &mut AppState) -> Result<bool, Box<dyn std::error::Error>> {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if app_state.show_favorites {
                            if app_state.selected_favorite < app_state.favorites.len().saturating_sub(1) {
                                app_state.selected_favorite += 1;
                            }
                        } else if !app_state.show_history
                            && app_state.selected_folder < app_state.folders.len().saturating_sub(1)
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
                        // Add to favorites action detected
                        if app_state.show_history {
                            return Ok(false); // Signal to main app to handle favorite action
                        }
                    }
                    KeyCode::Char('q') => return Ok(true), // Signal to quit
                    _ => {}
                }
            }
        }
        Ok(false) // Continue running
    }
    
    // Method to check if a favorite action was triggered
    pub fn check_favorite_action(&self, app_state: &AppState) -> bool {
        app_state.show_history // Only allow favorite actions in history view
    }

    pub fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}
