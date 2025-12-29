use std::cell::RefCell;

use crate::data::entry::Entry;
use crate::data::field::Field;
use crate::data::field::FieldValue;
use crate::style::ENTRY_BG;
use crate::style::HELP_LINE_BG;
use crate::ui::field_editor::FieldEditor;
use crate::widgets::confirm::Confirm;
use crate::widgets::form::Form;
use crate::widgets::form::FormSignal;
use crate::widgets::widget::ComponentRenderCtx;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Constraint;
use ratatui::layout::Flex;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::ScrollbarState;
use ratatui::Frame;

pub struct EntryEditor {
	/// Edited entry
	entry: Entry,
	/// ID of current field (-1 for none)
	selected: i32,
	/// ID of copied field (-1 for none)
	copied: i32,
	/// Editor for fields
	editor: Option<FieldEditor>,
	/// Confirm dialog
	confirm: Option<Confirm<'static>>,
	/// Set to true if modified
	modified: bool,

	list_state: RefCell<ListState>,
	scrollbar: RefCell<ScrollbarState>,
}

impl EntryEditor {
	pub fn new(entry: Entry) -> Self {
		let num_fields = entry.fields.len();
		Self {
			entry,
			selected: -1,
			copied: -1,
			editor: None,
			confirm: None,
			modified: false,
			list_state: RefCell::default(),
			scrollbar: RefCell::new(ScrollbarState::new(num_fields).position(0)),
		}
	}

	fn move_cursor(&mut self, offset: i32) {
		if offset >= 0 {
			self.selected = std::cmp::min(
				self.selected + offset,
				self.entry.fields.len().saturating_sub(1) as i32,
			);
		} else {
			self.selected = std::cmp::max(self.selected + offset, 0);
		}

		if self.entry.fields.is_empty() {
			self.selected = -1;
			self.list_state.borrow_mut().select(None);
			let scrollbar = self.scrollbar.borrow().position(0);
			*self.scrollbar.borrow_mut() = scrollbar;
		} else {
			self.list_state
				.borrow_mut()
				.select(Some(self.selected as usize));
			let scrollbar = self.scrollbar.borrow().position(self.selected as usize);
			*self.scrollbar.borrow_mut() = scrollbar
		}
	}

	pub fn input(&mut self, key: &KeyEvent) -> bool {
		// Field editor
		if let Some(editor) = &mut self.editor {
			let signal = editor.input_form(key);
			match signal {
				Some(FormSignal::Exit) => self.editor = None,
				Some(FormSignal::Return) => {
					// TODO: Popup with error
					if let Some(field) = editor.submit()
					{
						if self.selected == -1 {
							self.entry.fields.push(field);
						} else {
							self.entry.fields[self.selected as usize] = field;

						}
					}
					self.editor = None;
				},
				_ => {},
			}
			return false;
		}

		// Delete confirm box
		/*
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
		*/

		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);
		match key.code {
			// Reorder
			KeyCode::Up | KeyCode::Char('k') if shift_pressed => {
				if self.selected > 0 {
					self.entry.fields.swap(self.selected as usize, self.selected as usize - 1);
					self.move_cursor(-1);
					self.modified = true;
				}
			}
			KeyCode::Down | KeyCode::Char('j') if shift_pressed => {
				if self.selected != -1 && self.selected + 1 != self.entry.fields.len() as i32 {
					self.entry.fields.swap(self.selected as usize, self.selected as usize + 1);
					self.move_cursor(1);
					self.modified = true;
				}
			}

			// Movement
			KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => self.move_cursor(-1),
			KeyCode::Char('p') if ctrl_pressed => self.move_cursor(-1),
			KeyCode::PageUp => self.move_cursor(-16),
			KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => self.move_cursor(1),
			KeyCode::Char('n') if ctrl_pressed => self.move_cursor(1),
			KeyCode::PageDown => self.move_cursor(16),

			// Copy
			KeyCode::Char('y') => {
				if self.selected != -1 {
					self.copied = self.selected;
					self.entry.fields[self.selected as usize]
						.value
						.copy_to_clipboard();
				}
			}
			KeyCode::Char('c') if ctrl_pressed => {
				if self.selected != -1 {
					self.copied = self.selected;
					self.entry.fields[self.selected as usize]
						.value
						.copy_to_clipboard();
				}
			}

			KeyCode::Char('e') | KeyCode::Enter => {
				if self.selected != -1 {
					let field = &self.entry.fields[self.selected as usize];
					self.editor = Some(
						FieldEditor::new(format!("Edit Field: {}", field.name))
						.with_value(field)
							//.with_field(&self.entry.fields[self.selected as usize]),
					)
				}
			}
			KeyCode::Char('a') => {
				self.selected = -1;
				self.editor = Some(FieldEditor::new("New Field".into()));
			}
			/*
			KeyCode::Delete | KeyCode::Char('d') => {
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
			*/
			KeyCode::Esc | KeyCode::Char('q') => return true,
			_ => {}
		}
		false
	}

	fn field_preview(
		width: u16,
		field: Option<&Field>,
		selected: bool,
		yanked: bool,
		id: usize,
	) -> ListItem<'_> {
		let sep = std::cmp::max((width as f32 * 0.3) as u16, 20);

		let item = if let Some(field) = field {
			let name = field.name.as_str().bold();

			let value: Span = if field.hidden {
				"*****".fg(Color::Red)
			} else {
				match &field.value {
					FieldValue::Text(s) => s.as_str().italic(),
					FieldValue::Url(s) => s.as_str().underlined().fg(Color::Blue), // TODO HYPERLINK
					FieldValue::Phone(s) => s.as_str().bold().fg(Color::Yellow),
					FieldValue::Email(s) => s.as_str().underlined().fg(Color::Green), // TODO HYPERLINK
					FieldValue::TOTPRFC6238(_) => todo!(),
					FieldValue::TOTPSteam(_) => todo!(),
					FieldValue::TwoFactorRecovery(_two_facodes) => todo!(),
					FieldValue::Binary { mimetype: _, base64: _ } => todo!(),
				}
			};
			let modifiers = if yanked {
				" 󱓥".fg(Color::Red)
			} else {
				Span::from("")
			};

			let padding_width = (sep).saturating_sub(1 + name.width() as u16);
			let spacer = Span::styled(
				" ".repeat(padding_width as usize),
				ratatui::style::Style::default(),
			);

			ListItem::new(Line::from(vec![
				" ".into(),
				name,
				spacer,
				"| ".fg(Color::DarkGray),
				value,
				modifiers,
			]))
		} else {
			ListItem::new(Line::from(vec![]))
		};

		if selected {
			item.bg(ENTRY_BG[2])
			//list.underlined()
		} else {
			item.bg(ENTRY_BG[id % 2])
		}
	}

	pub fn draw(&self, frame: &mut Frame, rect: Rect) {
		let title = Line::from(
			vec![
			self.entry.name.as_str().fg(Color::Cyan).bold(),
			if self.modified {
				"󰽂 ".fg(Color::Magenta).bold()
			} else {
				"  ".into()
			}
			]
			);
		let help = Line::from(vec![
			" ⮁".bold().fg(Color::Green),
			" (navigate) ".into(),
			"S-⮁".bold().fg(Color::Green),
			" (reorder) ".into(),
			"a".bold().fg(Color::Green),
			" (add) ".into(),
			"e".bold().fg(Color::Green),
			" (edit) ".into(),
			"d".bold().fg(Color::Green),
			" (delete) ".into(),
			"y".bold().fg(Color::Green),
			" (yank)".into(),
		])
		.bg(HELP_LINE_BG);

		let vertical = Layout::vertical([Constraint::Length(1), Constraint::Percentage(100)]);
		let [help_area, content_area] = vertical.areas(rect);

		let mut items = self
			.entry
			.fields
			.iter()
			.enumerate()
			.map(|(id, ent)| {
				Self::field_preview(
					content_area.width,
					Some(ent),
					id as i32 == self.selected,
					id as i32 == self.copied,
					id,
				)
			})
			.collect::<Vec<_>>();
		while items.len() < content_area.height as usize {
			items.push(Self::field_preview(content_area.width, None, false, false, items.len()));
		}
		let messages = List::new(items).block(
			Block::default()
				.title(title)
				.title_alignment(ratatui::layout::HorizontalAlignment::Center),
		);
		frame.render_widget(Clear, rect);
		frame.render_widget(help, help_area);
		frame.render_widget(messages, content_area);

		// Field editor
		if let Some(editor) = &self.editor {
			let _title = format!(
				"{} > {}",
				self.entry.name,
				if self.selected != -1 {
					&self.entry.fields[self.selected as usize].name
				} else {
					"New Field"
				}
			);
			let area = frame.area();
			let vertical = Layout::vertical([Constraint::Length(20)]).flex(Flex::Center);
			let horizontal = Layout::horizontal([Constraint::Percentage(40)]).flex(Flex::Center);
			let [area] = area.layout(&vertical);
			let [area] = area.layout(&horizontal);
			let mut queue = vec![];
			let mut ctx = ComponentRenderCtx {
				area,
				selected: false,
				queue: &mut queue,
				depth: 0,
				cursor: None,
			};
			editor.render_form(frame, &mut ctx);
			if let Some((_, cursor)) = ctx.cursor {
				frame.set_cursor_position(cursor);
			}
		}
	}
}
