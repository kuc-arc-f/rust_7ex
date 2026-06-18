
use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use serde::Deserialize;
use std::env;

use anyhow::{Context};
use bytemuck::cast_slice;
use dotenvy::dotenv;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
//use rusqlite::{ffi::sqlite3_auto_extension, Connection, Result, params};
use rusqlite::{ffi::sqlite3_auto_extension, Connection, params};
use zerocopy::AsBytes;
use std::fs;
use std::fmt;
use std::path::Path;
use std::io::{self, Read, Write};
use uuid::Uuid;

mod mod_search;

const DB_FILE: &'static str = "example.db";
const TOP_K: usize = 3;

async fn GetEmbedding(api_key: &str, text: &str) -> anyhow::Result<Vec<f32>> {
    let embedding_values = mod_search::get_embedding(&api_key, &text).await.unwrap();
    Ok(embedding_values)
}

/**
*
* @param
*
* @return
*/
fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| App::new().run(terminal))
}
#[derive(Debug, Deserialize)]
struct Item {
    id: i32,
    title: String,
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
    /// Whether the app is currently loading
    is_loading: bool,
    /// The time when loading started
    loading_start: Option<std::time::Instant>,

    query: String,
}

enum InputMode {
    Normal,
    Editing,
}

impl App {
    const fn new() -> Self {
        Self {
            input: String::new(),
            //input_mode: InputMode::Normal,
            input_mode: InputMode::Editing,
            messages: Vec::new(),
            character_index: 0,
            is_loading: false,
            loading_start: None, 
            query: String::new(),            
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    const fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn submit_message(&mut self) {
        self.messages = vec![]; 
        self.query = self.input.clone();
        self.input.clear();
        self.reset_cursor();        
        self.is_loading = true;
        self.loading_start = Some(std::time::Instant::now()); 
    }
    #[tokio::main]
    async fn search_proc(&mut self) {
        let query_str = self.query.clone();
        // 環境変数取得
        let db_url = DB_FILE;
        let api_key = std::env::var("GEMINI_API_KEY")
            .expect("環境変数 GEMINI_API_KEY が設定されていません");
        let input_f32 = GetEmbedding(&api_key, &query_str).await.unwrap();
        //println!("取得成功! ベクトル次元数: {}", input_f32.len()); 
        let result = mod_search::db_search(&input_f32, TOP_K, query_str.clone()).await;
        //println!("results.len={}" , result.len());
        //s.push_str(", World!"); 
        let mut msgRow1 = "query: ".to_string(); 
        msgRow1.push_str(&query_str.clone()); 
        self.messages.push(msgRow1.clone());
        let msgRow2 = " ".to_string(); 
        self.messages.push(msgRow2.clone());
        self.messages.push("AI :".to_string());
        let items: Vec<String> = result
            .lines()
            .map(|s| s.to_string())
            .collect();
        for item in items {
            let s3 = format!("{}", item);
            self.messages.push(s3);
        }        
        self.input.clear();
        self.reset_cursor();

    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if self.is_loading {
                self.search_proc();
                self.is_loading = false;
                self.loading_start = None;
            }

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Some(key) = event::read()?.as_key_press_event() {
                    match self.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('e') => {
                                self.input_mode = InputMode::Editing;                                
                            }
                            KeyCode::Char('q') => {
                                return Ok(());
                            }
                            _ => {}
                        },
                        InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Enter => self.submit_message(),
                            KeyCode::Char(to_insert) => self.enter_char(to_insert),
                            KeyCode::Backspace => self.delete_char(),
                            KeyCode::Left => self.move_cursor_left(),
                            KeyCode::Right => self.move_cursor_right(),
                            KeyCode::Esc => self.input_mode = InputMode::Normal,
                            _ => {}
                        },
                        InputMode::Editing => {}
                    }
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ]);
        let [help_area, input_area, messages_area] = frame.area().layout(&layout);

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    "Press ".into(),
                    "q".bold(),
                    " to exit, ".into(),
                    "e".bold(),
                    " to start editing.".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to stop editing, ".into(),
                    "Enter".bold(),
                    " to record the message".into(),
                ],
                Style::default(),
            ),
        };
        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        let input_text = if self.is_loading {
            "Now Search , please wait ..."
        } else {
            self.input.as_str()
        };        
        let input = Paragraph::new(input_text)
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, input_area);
        match self.input_mode {
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            InputMode::Normal => {}

            // Make the cursor visible and ask ratatui to put it at the specified coordinates after
            // rendering
            #[expect(clippy::cast_possible_truncation)]
            InputMode::Editing => {
                if !self.is_loading {
                    frame.set_cursor_position(Position::new(
                        // Draw the cursor at the current position in the input field.
                        // This position can be controlled via the left and right arrow key
                        input_area.x + self.character_index as u16 + 1,
                        // Move one line down, from the border to the input line
                        input_area.y + 1,
                    ));
                }
            }
        }

        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let content = Line::from(Span::raw(format!("{m}")));
                ListItem::new(content)
            })
            .collect();
        let messages = List::new(messages).block(Block::bordered().title("Messages"));
        frame.render_widget(messages, messages_area);
    }
}
