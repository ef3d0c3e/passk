use core::panic;

use chrono::{DateTime, Utc};
use clipboard_rs::{Clipboard, ClipboardContext};
use color_eyre::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode,
};
use rand::distr::{Alphanumeric, SampleString};
use ratatui::{
    layout::{Constraint, Flex, Layout, Offset, Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, List, ListItem, Paragraph},
    DefaultTerminal, Frame,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Field {
    name: String,
    value: String,
    hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    EntryEditor,
}

impl ActiveWidget {
    pub fn editing(&self) -> bool {
        match self {
            ActiveWidget::Search | ActiveWidget::AddEntry => true,
            _ => false,
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

    pub fn from_text(text: String) -> Self {
        let len = text.len();
        Self {
            input: text,
            character_index: len,
            active: false,
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active
    }

    pub fn set_input(&mut self, input: String) {
        self.character_index = input.len();
        self.input = input;
    }

    pub fn submit_input(&mut self) -> String {
        let mut empty = String::default();
        std::mem::swap(&mut self.input, &mut empty);
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

struct EntryEditor {
    position: i32,
    entry: Entry,
    copied: i32,
    editor: Option<FieldEditor>,
}

impl EntryEditor {
    pub fn new(entry: Entry) -> Self {
        Self {
            position: -1,
            entry,
            copied: -1,
            editor: None,
        }
    }

    pub fn input(&mut self, key: &KeyEvent) -> bool {
        if let Some(editor) = &mut self.editor {
            if let Some(field) = editor.input(key) {
                if let Some(field) = field {
                    if self.position != -1 {
                        self.entry.fields[self.position as usize] = field;
                    } else {
                        self.entry.fields.push(field);
                    }
                }
                self.editor = None;
                return false;
            }
            return false;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                self.position = std::cmp::max(self.position - 1, 0)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.position = std::cmp::min(
                    self.position + 1,
                    self.entry.fields.len().saturating_sub(1) as i32,
                )
            }

            KeyCode::Char('y') | KeyCode::Enter => {
                if self.position != -1 {
                    self.copied = self.position;
                    let ctx = ClipboardContext::new().unwrap();
                    ctx.set_text(self.entry.fields[self.position as usize].value.clone())
                        .unwrap();
                }
            }
            KeyCode::Char('e') => {
                if self.position != -1 {
                    self.editor = Some(FieldEditor::from_field(
                        &self.entry.fields[self.position as usize],
                    ))
                }
            }
            KeyCode::Char('a') => {
                self.position = -1;
                self.editor = Some(FieldEditor::new());
            }
            KeyCode::Esc if self.editor.is_none() => return true,
            _ => {}
        }
        return false;
    }

    fn format_field(field: &Field, selected: bool, copied: bool) -> Line {
        let fg = if selected { Color::Black } else { Color::White };
        let bg = if selected {
            Color::White
        } else {
            Color::default()
        };
        Line::from(vec![
            field.name.as_str().fg(fg).bg(bg),
            " ".into(),
            if field.hidden {
                "*****"
            } else {
                field.value.as_str()
            }
            .fg(Color::Magenta),
            if copied { " (y)" } else { "" }.fg(Color::Red),
        ])
    }

    pub fn draw(&self, frame: &mut Frame, rect: Rect) {
        if let Some(editor) = &self.editor {
            let title = format!(
                "{} > {}",
                self.entry.name,
                if self.position != -1 {
                    &self.entry.fields[self.position as usize].name
                } else {
                    "New Field"
                }
            );
            let area = frame.area();
            let vertical = Layout::vertical([Constraint::Length(10)]).flex(Flex::Center);
            let horizontal = Layout::horizontal([Constraint::Percentage(40)]).flex(Flex::Center);
            let [area] = area.layout(&vertical);
            let [area] = area.layout(&horizontal);
            editor.draw(frame, area, &title);
            return;
        }
        let items = self
            .entry
            .fields
            .iter()
            .enumerate()
            .map(|(id, ent)| {
                ListItem::new(Self::format_field(
                    ent,
                    id as i32 == self.position,
                    id as i32 == self.copied,
                ))
            })
            .collect::<Vec<_>>();
        let messages = List::new(items).block(Block::bordered().title(self.entry.name.as_str()));
        frame.render_widget(messages, rect);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(u8)]
enum ActiveField {
    #[default]
    None,
    Name,
    Value,
    Hidden,
}

struct FieldEditor {
    name: TextInput,
    value: TextInput,
    generator: Option<TextInput>,
    hidden: bool,
    active: ActiveField,
}

/* Name   : [.....]
 * Value  : [.....]
 * Hidden : [x]
 */
impl FieldEditor {
    pub fn new() -> Self {
        Self {
            name: TextInput::new(),
            value: TextInput::new(),
            generator: None,
            hidden: false,
            active: ActiveField::default(),
        }
    }

    pub fn set_active(&mut self, active: ActiveField) {
        match self.active {
            ActiveField::Name => self.name.active = false,
            ActiveField::Value => self.value.active = false,
            _ => {}
        }
        self.active = active;
        match self.active {
            ActiveField::Name => self.name.active = true,
            ActiveField::Value => self.value.active = true,
            _ => {}
        }
    }

    pub fn from_field(field: &Field) -> Self {
        Self {
            name: TextInput::from_text(field.name.clone()),
            value: TextInput::from_text(field.value.clone()),
            hidden: field.hidden,
            active: ActiveField::default(),
            generator: None,
        }
    }

    fn submit(&self) -> Field {
        Field {
            name: self.name.input.clone(),
            value: self.value.input.clone(),
            hidden: self.hidden,
        }
    }

    pub fn input(&mut self, key: &KeyEvent) -> Option<Option<Field>> {
        // Password generator
        if let Some(generator) = &mut self.generator {
            if key.code == KeyCode::Enter {
                if let Ok(length) = generator.submit_input().parse::<i32>() {
                    if length > 0 {
                        let generated =
                            Alphanumeric.sample_string(&mut rand::rng(), length as usize);
                        match self.active {
                            ActiveField::Name => self.name.set_input(generated),
                            ActiveField::Value => self.value.set_input(generated),
                            _ => {}
                        }
                    }
                }
                self.generator = None;
                return None;
            }
            generator.input(key);
            return None;
        }

        let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Down | KeyCode::Tab => {
                let next = match self.active {
                    ActiveField::None => ActiveField::Name,
                    ActiveField::Name => ActiveField::Value,
                    ActiveField::Value => ActiveField::Hidden,
                    ActiveField::Hidden => ActiveField::Hidden,
                };
                self.set_active(next);
            }
            KeyCode::Up | KeyCode::BackTab => {
                let prev = match self.active {
                    ActiveField::None => ActiveField::None,
                    ActiveField::Name => ActiveField::Name,
                    ActiveField::Value => ActiveField::Name,
                    ActiveField::Hidden => ActiveField::Value,
                };
                self.set_active(prev);
            }
            KeyCode::Char(' ') if self.active == ActiveField::Hidden => self.hidden = !self.hidden,
            KeyCode::Char('g') if ctrl_pressed => match self.active {
                ActiveField::Name | ActiveField::Value => {
                    let mut generator = TextInput::from_text("64".into());
                    generator.set_active(true);
                    self.generator = Some(generator)
                }
                _ => {}
            },
            KeyCode::Enter => return Some(Some(self.submit())),
            KeyCode::Esc => return Some(None),
            _ => match self.active {
                ActiveField::Name => self.name.input(key),
                ActiveField::Value => self.value.input(key),
                _ => {}
            },
        }
        return None;
    }

    pub fn draw(&self, frame: &mut Frame, rect: Rect, title: &str) {
        if let Some(generator) = &self.generator {
            let mut area = rect;
            area.height = 3;
            generator.draw(frame, area, "Length");
            return;
        }
        let boxed = Block::bordered().title(title);
        frame.render_widget(boxed, rect);

        let area = rect;
        let vertical = Layout::vertical([Constraint::Length(3)]);
        let horizontal =
            Layout::horizontal([Constraint::Length(rect.width - 2)]).flex(Flex::Center);
        let [area] = area.layout(&vertical);
        let [area] = area.layout(&horizontal);
        let text = Text::from(Line::from(vec![
            "⮁".bold().fg(Color::Green),
            " (navigate) ".into(),
            "esc".bold().fg(Color::Green),
            " (cancel) ".into(),
            "enter".bold().fg(Color::Green),
            " (submit) ".into(),
            "C-g".bold().fg(Color::Green),
            " (generate) ".into(),
            "space".bold().fg(Color::Green),
            " (toggle) ".into(),
        ]));
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, area.offset(Offset::new(0, 1)));
        self.name
            .draw(frame, area.offset(Offset::new(0, 2)), "Name");
        self.value
            .draw(frame, area.offset(Offset::new(0, 5)), "Value");

        // Checkbox
        let fg = if self.active == ActiveField::Hidden {
            Color::Yellow
        } else {
            Color::Green
        };
        let checkbox = Text::from(Line::from(vec![
            format!("[{}]", if self.hidden { "x" } else { " " }).fg(fg),
            " Hidden".fg(fg),
        ]));
        let area = rect;
        let vertical = Layout::vertical([Constraint::Length(1)]);
        let horizontal =
            Layout::horizontal([Constraint::Length(rect.width - 2)]).flex(Flex::Center);
        let [area] = area.layout(&vertical);
        let [area] = area.layout(&horizontal);
        frame.render_widget(checkbox, area.offset(Offset::new(0, 8)));
    }
}

struct App {
    search: TextInput,
    add_entry: TextInput,
    active_widget: ActiveWidget,
    entries: Vec<Entry>,
    filtered_entries: Vec<usize>,
    entries_position: i32,

    editor: Option<EntryEditor>,
}

impl App {
    pub fn new() -> Self {
        let entries = vec![
            Entry {
                created_at: Utc::now(),
                modified_at: Utc::now(),
                name: "Foobar".into(),
                fields: vec![
                    Field {
                        name: "Mail".into(),
                        value: "test@example.com".into(),
                        hidden: false,
                    },
                    Field {
                        name: "Username".into(),
                        value: "user".into(),
                        hidden: false,
                    },
                    Field {
                        name: "Password".into(),
                        value: "password123".into(),
                        hidden: true,
                    },
                ],
            },
            Entry {
                created_at: Utc::now(),
                modified_at: Utc::now(),
                name: "Gome".into(),
                fields: vec![],
            },
        ];
        let filtered = (0..entries.len()).collect::<Vec<_>>();
        Self {
            search: TextInput::new(),
            add_entry: TextInput::new(),
            active_widget: ActiveWidget::default(),
            entries,
            filtered_entries: filtered,
            entries_position: -1,
            editor: None,
        }
    }

    fn set_active(&mut self, active: ActiveWidget) {
        match self.active_widget {
            ActiveWidget::Search => self.search.set_active(false),
            ActiveWidget::AddEntry => self.add_entry.set_active(false),
            ActiveWidget::EntryEditor => self.editor = None,
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
                        KeyCode::Down => {
                            self.entries_position = std::cmp::min(
                                self.entries_position + 1,
                                self.filtered_entries.len().saturating_sub(1) as i32,
                            );
                        }
                        KeyCode::Up => {
                            self.entries_position = std::cmp::max(self.entries_position - 1, 0);
                        }
                        KeyCode::Enter => {
                            if self.entries_position != -1 {
                                self.editor = Some(EntryEditor::new(
                                    self.entries
                                        [self.filtered_entries[self.entries_position as usize]]
                                        .clone(),
                                ));
                                self.set_active(ActiveWidget::EntryEditor);
                            }
                        }
                        _ => {}
                    }
                } else if self.active_widget.editing() {
                    self.entries_position = -1;
                    if key.kind == KeyEventKind::Press {
                        self.search.input(&key);
                        self.add_entry.input(&key);
                        match key.code {
                            KeyCode::Enter => {}
                            KeyCode::Esc => {
                                if self.active_widget == ActiveWidget::AddEntry {
                                    self.add_entry.submit_input();
                                }
                                self.set_active(ActiveWidget::None)
                            }
                            _ => {}
                        }
                    }
                } else if self.active_widget == ActiveWidget::EntryEditor {
                    let Some(editor) = self.editor.as_mut() else {
                        panic!()
                    };
                    if editor.input(&key) {
                        self.set_active(ActiveWidget::None)
                    }
                }
            }
        }
    }

    fn format_entry(ent: &Entry, selected: bool) -> Line {
        let fg = if selected { Color::Black } else { Color::White };
        let bg = if selected {
            Color::White
        } else {
            Color::default()
        };
        Line::from(vec![
            ent.name.as_str().fg(fg).bg(bg),
            " ".bg(bg),
            format!("+{}", ent.fields.len()).fg(Color::Green).bg(bg),
        ])
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
            "?".bold().fg(Color::Green),
            " (help) ".into(),
            "/".bold().fg(Color::Green),
            " (search) ".into(),
            "a".bold().fg(Color::Green),
            " (add) ".into(),
            "⮁".bold().fg(Color::Green),
            " (navigate) ".into(),
            "y".bold().fg(Color::Green),
            " (copy) ".into(),
        ]));
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        // Search
        self.search.draw(frame, search_area, "Search");

        // Content
        let items = self
            .filtered_entries
            .iter()
            .map(|i| &self.entries[*i])
            .enumerate()
            .map(|(id, ent)| {
                ListItem::new(Self::format_entry(ent, id as i32 == self.entries_position))
            })
            .collect::<Vec<_>>();
        let messages = List::new(items).block(Block::bordered().title("Entries"));
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

        // Editor
        if let Some(editor) = self.editor.as_ref() {
            let field_num = editor.entry.fields.len() + 2;
            let vertical = Layout::vertical([Constraint::Max(field_num as u16)]).flex(Flex::Center);
            let horizontal = Layout::horizontal([Constraint::Percentage(40)]).flex(Flex::Center);
            let [area] = frame.area().layout(&vertical);
            let [area] = area.layout(&horizontal);
            editor.draw(frame, area);
        }
    }
}

fn main() -> Result<()> {
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result
}
