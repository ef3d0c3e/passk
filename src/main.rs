use core::panic;
use std::sync::LazyLock;

use chrono::DateTime;
use chrono::Utc;
use clipboard_rs::ClipboardContext;
use color_eyre::Result;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;
use crossterm::event::{self};
use ratatui::layout::Constraint;
use ratatui::layout::Flex;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::Paragraph;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use serde::Deserialize;
use serde::Serialize;

use crate::data::field::Field;
use crate::data::field::FieldValue;
use crate::ui::entry_editor::EntryEditor;
use crate::widgets::text_input::TextInput;
use crate::widgets::widget::Component;

pub mod data;
pub mod widgets;
pub mod ui;
pub mod style;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
	pub name: String,
	pub fields: Vec<Field>,

	pub created_at: DateTime<Utc>,
	pub modified_at: DateTime<Utc>,
	pub accessed_at: DateTime<Utc>,
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

pub static CLIPBOARD_CTX : LazyLock<ClipboardContext> = LazyLock::new(|| {
	ClipboardContext::new().unwrap()
});

struct App {
	search: TextInput<'static>,
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
				accessed_at: Utc::now(),
				name: "Foobar".into(),
				fields: vec![
					Field {
					name: "Username".into(),
					value: FieldValue::Text("Foobar".into()),
					hidden: false,
					date_added: Utc::now(),
					date_modified: Utc::now(),
					date_accessed: Utc::now(),
				},
					Field {
					name: "Mail".into(),
					value: FieldValue::Email("test@example.com".into()),
					hidden: false,
					date_added: Utc::now(),
					date_modified: Utc::now(),
					date_accessed: Utc::now(),
				},
					Field {
					name: "Password".into(),
					value: FieldValue::Text("password123".into()),
					hidden: true,
					date_added: Utc::now(),
					date_modified: Utc::now(),
					date_accessed: Utc::now(),
				},
				],
			},
			Entry {
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
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
			ActiveWidget::Search => {/*self.search.set_active(false)*/},
			ActiveWidget::AddEntry => {/*self.add_entry.set_active(false)*/},
			ActiveWidget::EntryEditor => self.editor = None,
			_ => {}
		}
		self.active_widget = active;
		match self.active_widget {
			ActiveWidget::Search => {/*self.search.set_active(true)*/},
			ActiveWidget::AddEntry => {/*self.add_entry.set_active(true)*/},
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
			//self.add_entry.draw(frame, popup_area, None);
		}

		// Search
		//self.search.draw(frame, search_area, None);

		// Editor
		if let Some(editor) = self.editor.as_ref() {
			editor.draw(frame, content_area);
		}
	}
}

fn main() -> Result<()> {
	let terminal = ratatui::init();
	let app_result = App::new().run(terminal);
	ratatui::restore();
	app_result
}
