use core::panic;
use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use clipboard_rs::{Clipboard, ClipboardContext};
use color_eyre::{owo_colors::OwoColorize, Result};
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode,
};
use rand::distr::{Alphanumeric, SampleString};
use ratatui::{
    DefaultTerminal, Frame, layout::{Constraint, Flex, Layout, Offset, Position, Rect}, style::{Color, Style, Stylize}, text::{Line, Text}, widgets::{Block, Clear, List, ListItem, Paragraph}
};
use serde::{de, Deserialize, Serialize};

use crate::widgets::{confirm::ConfirmDialog, combo_box::ComboBox, field_editor::FieldEditor, text_input::TextInput};

pub mod widgets;

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

struct EntryEditor {
    entry: Entry,
    /// ID of current field (-1 for none)
    selected: i32,
    /// ID of copied field (-1 for none)
    copied: i32,
    editor: Option<FieldEditor<'static>>,
    confirm: Option<ConfirmDialog<'static>>,
}

impl EntryEditor {
    pub fn new(entry: Entry) -> Self {
        Self {
            entry,
            selected: -1,
            copied: -1,
            editor: None,
            confirm: None,
        }
    }

    pub fn input(&mut self, key: &KeyEvent) -> bool {
        // Field editor
        if let Some(editor) = &mut self.editor {
            if let Some(field) = editor.input(key) {
                if let Some(field) = field {
                    if self.selected != -1 {
                        self.entry.fields[self.selected as usize] = field;
                    } else {
                        self.entry.fields.push(field);
                    }
                }
                self.editor = None;
                return false;
            }
            return false;
        }

        // Delete confirm box
        if let Some(confirm) = &mut self.confirm {
            if let Some(val) = confirm.input(key) {
                if val {
                    self.entry.fields.remove(self.selected as usize);
					self.selected = -1;
					self.copied = -1;
                }
                self.confirm = None;
            }
            return false;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                self.selected = std::cmp::max(self.selected - 1, 0);
				if self.entry.fields.is_empty() {
					self.selected = -1
				}
				
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.selected = std::cmp::min(
                    self.selected + 1,
                    self.entry.fields.len().saturating_sub(1) as i32,
                );
				if self.entry.fields.is_empty() {
					self.selected = -1
				}
            }

            KeyCode::Char('y') | KeyCode::Enter => {
                if self.selected != -1 {
                    self.copied = self.selected;
                    let ctx = ClipboardContext::new().unwrap();
                    ctx.set_text(self.entry.fields[self.selected as usize].value.clone())
                        .unwrap();
                }
            }
            KeyCode::Char('e') => {
                if self.selected != -1 {
                    self.editor = Some(FieldEditor::new(Line::from(self.entry.name.clone()))
						.with_field(&self.entry.fields[self.selected as usize])
                    )
                }
            }
            KeyCode::Backspace | KeyCode::Delete | KeyCode::Char('d') => {
                if self.selected != -1 {
                    let title = Line::from(vec!["Confirm".into()]);
                    let desc = Line::from(vec![
                        "Delete field '".into(),
                        self.entry.fields[self.selected as usize]
                            .name
                            .clone()
                            .fg(Color::Blue),
                        "'?".into(),
                    ]);
                    self.confirm = Some(ConfirmDialog::new(title, vec![ListItem::from(desc)]));
                }
            }
            KeyCode::Char('a') => {
                self.selected = -1;
                self.editor = Some(FieldEditor::new(Line::from("Add Field")));
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
                if self.selected != -1 {
                    &self.entry.fields[self.selected as usize].name
                } else {
                    "New Field"
                }
            );
            let area = frame.area();
            let vertical = Layout::vertical([Constraint::Length(10)]).flex(Flex::Center);
            let horizontal = Layout::horizontal([Constraint::Percentage(40)]).flex(Flex::Center);
            let [area] = area.layout(&vertical);
            let [area] = area.layout(&horizontal);
            editor.draw(frame, area);
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
                    id as i32 == self.selected,
                    id as i32 == self.copied,
                ))
            })
            .collect::<Vec<_>>();
        let messages = List::new(items).block(Block::bordered().title(self.entry.name.as_str()));
        frame.render_widget(messages, rect);

        if let Some(confirm) = &self.confirm {
            confirm.draw(frame);
        }
    }
}

struct App {
    search: ComboBox<'static>,
    add_entry: TextInput<'static>,
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
		let mut centries = BTreeSet::default();
		centries.insert("Text".to_string());
		centries.insert("TOTP/Steam".to_string());
		centries.insert("TOTP/RFC 6238".to_string());
        Self {
            search: ComboBox::new(Line::from("Search"), Constraint::Percentage(100), centries),
            add_entry: TextInput::new(Line::from("New Entry"), Constraint::Percentage(100)),
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
                        KeyCode::Enter | KeyCode::Char('e') => {
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
                                    self.add_entry.submit();
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
            self.add_entry.draw(frame, popup_area);
        }

        // Search
        self.search.draw(frame, search_area);

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
