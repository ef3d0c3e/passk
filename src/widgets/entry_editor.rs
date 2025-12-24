use std::cell::RefCell;

use crate::data::field::Field;
use crate::data::field::FieldValue;
use crate::widgets::confirm::ConfirmDialog;
use crate::widgets::field_editor::FieldEditor;
use crate::Entry;

use color_eyre::owo_colors::OwoColorize;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Constraint;
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
	editor: Option<FieldEditor<'static>>,
	/// Confirm dialog
	confirm: Option<ConfirmDialog<'static>>,

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
		/*
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
		*/

		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		match key.code {
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

			/*
			KeyCode::Char('e') | KeyCode::Enter => {
				if self.selected != -1 {
					self.editor = Some(
						FieldEditor::new(Line::from(self.entry.name.clone()))
							.with_field(&self.entry.fields[self.selected as usize]),
					)
				}
			}
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
			KeyCode::Char('a') => {
				self.selected = -1;
				self.editor = Some(FieldEditor::new(Line::from("Add Field")));
			}
			*/
			KeyCode::Esc if self.editor.is_none() => return true,
			_ => {}
		}
		return false;
	}

	fn field_preview(
		width: u16,
		field: &Field,
		selected: bool,
		yanked: bool,
		id: usize,
	) -> ListItem {
		let bg_cols = [
			Color::from_u32(0x322b44),
			Color::from_u32(0x241f31),
			Color::from_u32(0x5d507f),
		];
		let sep = std::cmp::max((width as f32 * 0.3) as u16, 20);

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
				FieldValue::TwoFactorRecovery(two_facodes) => todo!(),
				FieldValue::Binary { mimetype, base64 } => todo!(),
			}
		};
		let modifiers = if yanked {
			" 󱓥".fg(Color::Red)
		} else {
			Span::from("")
		};

		let item = ListItem::new(Line::from(vec![
			" ".into(),
			field.name.as_str().bold(),
			": ".into(),
			value,
			modifiers,
		]));
		if selected {
			item.bg(bg_cols[2])
			//list.underlined()
		} else {
			item.bg(bg_cols[id % 2])
		}
	}

	pub fn draw(&self, frame: &mut Frame, rect: Rect) {
		//if let Some(editor) = &self.editor {
		//	let title = format!(
		//		"{} > {}",
		//		self.entry.name,
		//		if self.selected != -1 {
		//			&self.entry.fields[self.selected as usize].name
		//		} else {
		//			"New Field"
		//		}
		//	);
		//	let area = frame.area();
		//	let vertical = Layout::vertical([Constraint::Length(10)]).flex(Flex::Center);
		//	let horizontal = Layout::horizontal([Constraint::Percentage(40)]).flex(Flex::Center);
		//	let [area] = area.layout(&vertical);
		//	let [area] = area.layout(&horizontal);
		//	editor.draw(frame, area);
		//	return;
		//}

		let title = Line::from(vec![self.entry.name.as_str().fg(Color::Cyan).bold()]);
		let help = Line::from(vec![
			" ⮁".bold().fg(Color::Green),
			" (navigate) ".into(),
			"S-⮁".bold().fg(Color::Green),
			" (reorder) ".into(),
			"a".bold().fg(Color::Green),
			" (add) ".into(),
			"d".bold().fg(Color::Green),
			" (delete) ".into(),
			"y".bold().fg(Color::Green),
			" (yank)".into(),
		])
		.bg(Color::from_u32(0x1a60b5));

		let vertical = Layout::vertical([Constraint::Length(1), Constraint::Percentage(100)]);
		let [help_area, content_area] = vertical.areas(rect);

		let items = self
			.entry
			.fields
			.iter()
			.enumerate()
			.map(|(id, ent)| {
				Self::field_preview(
					content_area.width,
					ent,
					id as i32 == self.selected,
					id as i32 == self.copied,
					id,
				)
			})
			.collect::<Vec<_>>();
		let messages = List::new(items).block(
			Block::default()
				.title(title)
				.title_alignment(ratatui::layout::HorizontalAlignment::Center),
		);
		frame.render_widget(Clear, rect);
		frame.render_widget(help, help_area);
		frame.render_widget(messages, content_area);
	}
}
