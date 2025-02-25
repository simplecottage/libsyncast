use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, BorderType, List, ListItem, Paragraph, Tabs, Wrap},
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

// Define UI colors for a cohesive theme
const ACTIVE_TAB_COLOR: Color = Color::Rgb(252, 152, 103);    // Coral accent color
const INACTIVE_TAB_COLOR: Color = Color::DarkGray;
const SELECTED_ITEM_COLOR: Color = Color::Rgb(252, 152, 103); // Coral accent
const BORDER_COLOR: Color = Color::DarkGray;
const TITLE_COLOR: Color = Color::White;
const TEXT_COLOR: Color = Color::Gray;
const HIGHLIGHT_COLOR: Color = Color::Rgb(252, 152, 103);     // Coral accent

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
        
        // Create a main layout with a header area and content area
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header with tabs
                Constraint::Min(0),    // Content area
            ])
            .split(size);
        
        // Render the tabs header
        self.render_tabs(f, app_state, main_layout[0]);
        
        // Split content into left panel and right panel
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left sidebar
                Constraint::Percentage(70), // Right content
            ])
            .split(main_layout[1]);

        // Render left panel content based on active tab
        if app_state.show_favorites {
            self.render_favorites(f, app_state, content_layout[0]);
        } else if app_state.show_history {
            self.render_history(f, app_state, content_layout[0]);
        } else {
            self.render_folders(f, app_state, content_layout[0]);
        }

        // Render right panel
        self.render_right_panel(f, app_state, content_layout[1]);
        
        // Render a subtle footer with keyboard shortcuts
        self.render_footer(f, size);
    }
    
    fn render_tabs(&self, f: &mut Frame<CrosstermBackend<std::io::Stdout>>, app_state: &AppState, area: Rect) {
        let tab_titles = vec!["Folders", "History", "Favorites"];
        let selected_tab = if app_state.show_favorites {
            2
        } else if app_state.show_history {
            1
        } else {
            0
        };
        
        let tab_items: Vec<Spans> = tab_titles
            .iter()
            .map(|t| {
                let (first, rest) = t.split_at(1);
                Spans::from(vec![
                    Span::styled(first, Style::default().fg(HIGHLIGHT_COLOR).add_modifier(Modifier::BOLD)),
                    Span::styled(rest, Style::default().fg(TEXT_COLOR))
                ])
            })
            .collect();
        
        let tabs = Tabs::new(tab_items)
            .select(selected_tab)
            .style(Style::default().fg(INACTIVE_TAB_COLOR))
            .highlight_style(
                Style::default()
                    .fg(ACTIVE_TAB_COLOR)
                    .add_modifier(Modifier::BOLD)
            )
            .divider(Span::raw(" | "));
            
        f.render_widget(tabs, area);
    }

    fn render_favorites(
        &self,
        f: &mut Frame<CrosstermBackend<std::io::Stdout>>,
        app_state: &AppState,
        area: Rect,
    ) {
        let favorite_items: Vec<ListItem> = app_state
            .favorites
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == app_state.selected_favorite {
                    Style::default().fg(SELECTED_ITEM_COLOR).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT_COLOR)
                };
                
                // Create two-line list items with title and URL
                let title = Spans::from(Span::styled(&item.title, style));
                let url = Spans::from(Span::styled(
                    format!("  {}", item.url),
                    Style::default().fg(Color::DarkGray)
                ));
                
                ListItem::new(vec![title, url])
                    .style(Style::default().bg(if i == app_state.selected_favorite {
                        Color::Rgb(40, 40, 40) // Subtle highlight background
                    } else {
                        Color::Reset
                    }))
            })
            .collect();

        let favorites_list = List::new(favorite_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(BORDER_COLOR))
                    .title(Span::styled(" Favorites ", Style::default().fg(TITLE_COLOR)))
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
            
        f.render_widget(favorites_list, area);
    }

    fn render_history(
        &self,
        f: &mut Frame<CrosstermBackend<std::io::Stdout>>,
        app_state: &AppState,
        area: Rect,
    ) {
        let history_items: Vec<ListItem> = app_state
            .history
            .iter()
            .enumerate()
            .map(|(i, item)| {
                // Create two-line list items
                let title = Spans::from(Span::styled(&item.title, Style::default().fg(TEXT_COLOR)));
                let url = Spans::from(Span::styled(
                    format!("  {}", item.url),
                    Style::default().fg(Color::DarkGray)
                ));
                
                ListItem::new(vec![title, url])
            })
            .collect();

        let history_list = List::new(history_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(BORDER_COLOR))
                    .title(Span::styled(" History ", Style::default().fg(TITLE_COLOR)))
            );
            
        f.render_widget(history_list, area);
    }

    fn render_folders(
        &self,
        f: &mut Frame<CrosstermBackend<std::io::Stdout>>,
        app_state: &AppState,
        area: Rect,
    ) {
        let folder_items: Vec<ListItem> = app_state
            .folders
            .iter()
            .enumerate()
            .map(|(i, folder)| {
                let style = if i == app_state.selected_folder {
                    Style::default().fg(SELECTED_ITEM_COLOR).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT_COLOR)
                };
                
                ListItem::new(Span::styled(&folder.name, style))
                    .style(Style::default().bg(if i == app_state.selected_folder {
                        Color::Rgb(40, 40, 40) // Subtle highlight background
                    } else {
                        Color::Reset
                    }))
            })
            .collect();

        let folder_list = List::new(folder_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(BORDER_COLOR))
                    .title(Span::styled(" Folders ", Style::default().fg(TITLE_COLOR)))
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
            
        f.render_widget(folder_list, area);
    }
    
    fn render_right_panel(
        &self,
        f: &mut Frame<CrosstermBackend<std::io::Stdout>>,
        app_state: &AppState,
        area: Rect,
    ) {
        // This is a placeholder for the right panel content
        // In a real app, this would show the details of the selected item
        
        let content = if app_state.show_favorites && !app_state.favorites.is_empty() {
            let selected = &app_state.favorites[app_state.selected_favorite];
            format!("Title: {}\nURL: {}", selected.title, selected.url)
        } else if app_state.show_history && !app_state.history.is_empty() {
            "History details will appear here"
        } else if !app_state.folders.is_empty() {
            let selected = &app_state.folders[app_state.selected_folder];
            format!("Folder: {}", selected.name)
        } else {
            "No item selected"
        };
        
        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(BORDER_COLOR))
                    .title(Span::styled(" Details ", Style::default().fg(TITLE_COLOR)))
            )
            .style(Style::default().fg(TEXT_COLOR))
            .wrap(Wrap { trim: true });
            
        f.render_widget(paragraph, area);
    }
    
    fn render_footer(&self, f: &mut Frame<CrosstermBackend<std::io::Stdout>>, area: Rect) {
        let footer_area = Rect::new(
            area.x,
            area.height - 1,
            area.width,
            1
        );
        
        let keys = vec![
            Span::styled("q", Style::default().fg(HIGHLIGHT_COLOR)),
            Span::raw(" quit • "),
            Span::styled("↑/k", Style::default().fg(HIGHLIGHT_COLOR)),
            Span::raw(" "),
            Span::styled("↓/j", Style::default().fg(HIGHLIGHT_COLOR)),
            Span::raw(" navigate • "),
            Span::styled("f", Style::default().fg(HIGHLIGHT_COLOR)),
            Span::raw(" add favorite • "),
            Span::styled("tab", Style::default().fg(HIGHLIGHT_COLOR)),
            Span::raw(" switch view"),
        ];
        
        let footer = Paragraph::new(Spans::from(keys))
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
            
        f.render_widget(footer, footer_area);
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
                    KeyCode::Tab => {
                        // Cycle through views: Folders -> History -> Favorites -> Folders
                        if !app_state.show_history && !app_state.show_favorites {
                            // Currently in Folders, go to History
                            app_state.show_history = true;
                        } else if app_state.show_history {
                            // Currently in History, go to Favorites
                            app_state.show_history = false;
                            app_state.show_favorites = true;
                        } else {
                            // Currently in Favorites, go to Folders
                            app_state.show_favorites = false;
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
