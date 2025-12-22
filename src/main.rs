use std::default;

use chrono::{DateTime, Utc};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, List, ListItem, Paragraph},
    DefaultTerminal, Frame,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Field {
    name: String,
    content: String,
    hidden: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Entry {
    created_at: DateTime<Utc>,
    modified_at: DateTime<Utc>,
    name: String,
    fields: Vec<Field>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum ActiveWidget {
    #[default]
    None,
    Search,
    AddEntry,
}

impl ActiveWidget {
    pub fn editing(&self) -> bool {
        match self {
            ActiveWidget::None => false,
            _ => true,
        }
    }
}

struct TextInput {
    input: String,
    character_index: usize,
    active: bool,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            input: String::default(),
            character_index: 0,
            active: false,
        }
    }

	pub fn set_active(&mut self, active: bool) {
		self.active = active
	}

	pub fn submit_input(&mut self) -> String {
		let mut empty = String::default();
		std::mem::swap(&mut self.input,&mut empty);
		self.character_index = 0;
		empty
	}

    pub fn input(&mut self, key: &KeyEvent) {
        if !self.active {
            return;
        }
        match key.code {
            KeyCode::Char(to_insert) => self.enter_char(to_insert),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            _ => {}
        }
    }

    pub fn draw(&self, frame: &mut Frame, rect: Rect, title: &str) {
        let popup = Paragraph::new(self.input.as_str())
            .style(if self.active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            })
            .block(Block::bordered().title(title));
        frame.render_widget(popup, rect);
        if self.active {
            frame.set_cursor_position(Position::new(
                // Draw the cursor at the current position in the input field.
                // This position is can be controlled via the left and right arrow key
                rect.x + self.character_index as u16 + 1,
                // Move one line down, from the border to the input line
                rect.y + 1,
            ))
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
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
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
}

struct App {
	search: TextInput,
	add_entry: TextInput,
    active_widget: ActiveWidget,
}

impl App {
    pub fn new() -> Self {
        Self {
			search: TextInput::new(),
			add_entry: TextInput::new(),
            active_widget: ActiveWidget::default(),
        }
    }

	fn set_active(&mut self, active: ActiveWidget) {
		match self.active_widget {
			ActiveWidget::Search => self.search.set_active(false),
			ActiveWidget::AddEntry => self.add_entry.set_active(false),
			_ => {}
		}
		self.active_widget = active;
		match self.active_widget {
			ActiveWidget::Search => self.search.set_active(true),
			ActiveWidget::AddEntry => self.add_entry.set_active(true),
			_ => {}
		}
	}

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                if self.active_widget == ActiveWidget::None {
                    match key.code {
						KeyCode::Char('/') => self.set_active(ActiveWidget::Search),
                        KeyCode::Char('a') => self.set_active(ActiveWidget::AddEntry),
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        _ => {}
                    }
                } else if self.active_widget.editing() {
                    if key.kind == KeyEventKind::Press {
						self.search.input(&key);
						self.add_entry.input(&key);
                        match key.code {
                            KeyCode::Enter => {

							},
                            KeyCode::Esc => {
								if self.active_widget == ActiveWidget::AddEntry {
									self.add_entry.submit_input();
								}
								self.set_active(ActiveWidget::None) 
							},
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ]);
        let [help_area, search_area, content_area] = vertical.areas(frame.area());

        // Help
        let text = Text::from(Line::from(vec![
            "PassK 0.1 ".into(),
            "?".bold().fg(Color::Yellow),
            " (help) ".into(),
            "/".bold().fg(Color::Yellow),
            " (search) ".into(),
            "a".bold().fg(Color::Yellow),
            " (add) ".into(),
            "⮁".bold().fg(Color::Yellow),
            " (navigate) ".into(),
            "y".bold().fg(Color::Yellow),
            " (copy) ".into(),
        ]));
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        // Search
		self.search.draw(frame, search_area, "Search");

        let msgs = vec!["foo", "bar", "baz"];
        let content: Vec<ListItem> = msgs
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let content = Line::from(Span::raw(format!("{i}: {m}")));
                ListItem::new(content)
            })
            .collect();
        let messages = List::new(content).block(Block::bordered().title("Entries"));
        frame.render_widget(messages, content_area);

        if self.active_widget == ActiveWidget::AddEntry {
            fn centered_area(area: Rect, percent_x: u16, size_y: u16) -> Rect {
                let vertical = Layout::vertical([Constraint::Length(size_y)]).flex(Flex::Center);
                let horizontal =
                    Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
                let [area] = area.layout(&vertical);
                let [area] = area.layout(&horizontal);
                area
            }

            let popup_area = centered_area(frame.area(), 60, 3);
			self.add_entry.draw(frame, popup_area, "New Entry");
        }
    }
}

fn main() -> Result<()> {
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result
}
