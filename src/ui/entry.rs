use core::panic;
use std::cell::RefCell;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Constraint;
use ratatui::layout::Flex;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::ScrollbarState;
use ratatui::Frame;

use crate::data::entry::Entry;
use crate::data::field::Field;
use crate::data::field::FieldValue;
use crate::style::ENTRY_BG;
use crate::style::HELP_LINE_BG;
use crate::ui::field_editor::FieldEditor;
use crate::widgets::confirm::Confirm;
use crate::widgets::form::Form;
use crate::widgets::form::FormSignal;
use crate::widgets::widget::Component;
use crate::widgets::widget::ComponentRenderCtx;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConfirmAction {
	Delete,
	Quit,
}

pub struct EntryEditor {
	entry: Entry,

	copied: Option<usize>,
	selected: Option<usize>,

	modified: bool,
	save: bool,
	confirm: Option<Confirm<'static>>,
	confirm_action: Option<ConfirmAction>,

	editor: Option<FieldEditor>,

	list_state: RefCell<ListState>,
	scrollbar: RefCell<ScrollbarState>,
}

impl EntryEditor {
	pub fn new(entry: Entry) -> Self {
		let len = entry.fields.len();
		Self {
			entry,
			copied: None,
			selected: None,
			modified: false,
			save: true,
			confirm: None,
			confirm_action: None,
			editor: None,
			list_state: RefCell::default(),
			scrollbar: RefCell::new(ScrollbarState::new(len).position(0)),
		}
	}

	pub fn move_selected(&mut self, offset: i32) {
		if self.entry.fields.is_empty() {
			self.selected = None;
			return;
		}

		if offset > 0 {
			if let Some(selected) = self.selected {
				self.selected = Some(std::cmp::min(
					self.entry.fields.len() - 1,
					selected + offset as usize,
				));
			} else {
				self.selected = Some(std::cmp::min(
					self.entry.fields.len() - 1,
					(offset as usize).saturating_sub(1),
				));
			}
		} else if offset < 0 {
			if let Some(selected) = self.selected {
				self.selected = Some(selected.saturating_sub((-offset) as usize));
			}
		}
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
					FieldValue::Binary {
						mimetype: _,
						base64: _,
					} => todo!(),
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
		} else {
			item.bg(ENTRY_BG[id % 2])
		}
	}

	pub fn submit(&self) -> Option<Entry> {
		if !self.save {
			return None;
		}

		Some(self.entry.clone())
	}
}

impl Component for EntryEditor {
	fn input(&mut self, key: &KeyEvent) -> bool {
		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);

		// Confirm
		if let Some(confirm) = &mut self.confirm {
			confirm.input(key);
			match confirm.submit() {
				Some(true) => {
					let action = self.confirm_action.unwrap();
					match action {
						ConfirmAction::Delete => {
							let selected = self.selected.unwrap();
							self.entry.fields.remove(selected);
							self.move_selected(-1);
						}
						ConfirmAction::Quit => {
							self.save = true;
							return false;
						}
					}
				}
				Some(false) => {
					let action = self.confirm_action.unwrap();
					if action == ConfirmAction::Quit {
     							self.save = false;
     							return false;
     						}
				}
				None => return true,
			}
			self.confirm = None;
			self.confirm_action = None;
			return true;
		}

		// Field editor
		if let Some(editor) = &mut self.editor {
			match editor.input_form(key) {
				Some(FormSignal::Exit) => self.editor = None,
				Some(FormSignal::Return) => {
					// TODO: Popup with error
					if let Some(field) = editor.submit() {
						if let Some(selected) = self.selected {
							self.entry.fields[selected] = field;
						} else {
							self.entry.fields.push(field);
						}
					}
					self.editor = None;
				}
				_ => {}
			}
			return true;
		}

		match key.code {
			// Reorder
			KeyCode::Up | KeyCode::Char('k') if shift_pressed => {
				if let Some(selected) = self.selected {
					if selected != 0 {
						self.entry.fields.swap(selected, selected - 1);
						self.move_selected(-1);
						self.modified = true;
					}
				}
			}
			KeyCode::Down | KeyCode::Char('j') if shift_pressed => {
				if let Some(selected) = self.selected {
					if selected + 1 != self.entry.fields.len() {
						self.entry.fields.swap(selected, selected + 1);
						self.move_selected(1);
						self.modified = true;
					}
				}
			}

			// Movement
			KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => self.move_selected(-1),
			KeyCode::Char('p') if ctrl_pressed => self.move_selected(-1),
			KeyCode::PageUp => self.move_selected(-16),
			KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => self.move_selected(1),
			KeyCode::Char('n') if ctrl_pressed => self.move_selected(1),
			KeyCode::PageDown => self.move_selected(16),

			// Copy
			KeyCode::Char('y') => {
				if let Some(selected) = self.selected {
					self.copied = self.selected;
					self.entry.fields[selected].value.copy_to_clipboard();
				}
			}
			KeyCode::Char('c') if ctrl_pressed => {
				if let Some(selected) = self.selected {
					self.copied = self.selected;
					self.entry.fields[selected].value.copy_to_clipboard();
				}
			}
			// Edit
			KeyCode::Char('e') | KeyCode::Enter => {
				if let Some(selected) = self.selected {
					let field = &self.entry.fields[selected];
					self.editor = Some(
						FieldEditor::new(format!("Edit Field: {}", field.name)).with_value(field),
					);
					self.modified = true;
				}
			}
			// Add
			KeyCode::Char('a') => {
				self.selected = None;
				self.editor = Some(FieldEditor::new("New Field".into()));
				self.modified = true;
			}
			// Delete
			KeyCode::Delete | KeyCode::Char('d') => {
				if let Some(selected) = self.selected {
					let field = &self.entry.fields[selected];
					self.confirm = Some(Confirm::new(
						format!("Delete field {}", field.name),
						Paragraph::new(Text::from(format!(
							"Really delete field '{}'?",
							field.name
						))),
					));
					self.confirm_action = Some(ConfirmAction::Delete);
				}
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
			KeyCode::Esc | KeyCode::Char('q') => {
				if self.modified {
					self.confirm = Some(Confirm::new(
						"Save Changes".into(),
						Paragraph::new(Text::from("Exit and save changes?")),
					));
					self.confirm_action = Some(ConfirmAction::Quit);
				} else {
					return false;
				}
			}
			_ => {}
		}
		true
	}

	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let title = Line::from(vec![
			self.entry.name.as_str().fg(Color::Cyan).bold(),
			if self.modified {
				"󰽂 ".fg(Color::Magenta).bold()
			} else {
				"  ".into()
			},
		]);
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
		let [help_area, content_area] = vertical.areas(ctx.area);

		let mut items = self
			.entry
			.fields
			.iter()
			.enumerate()
			.map(|(id, ent)| {
				Self::field_preview(
					content_area.width,
					Some(ent),
					Some(id) == self.selected,
					Some(id) == self.copied,
					id,
				)
			})
			.collect::<Vec<_>>();
		while items.len() < content_area.height as usize {
			items.push(Self::field_preview(
				content_area.width,
				None,
				false,
				false,
				items.len(),
			));
		}
		let messages = List::new(items).block(
			Block::default()
				.title(title)
				.title_alignment(ratatui::layout::HorizontalAlignment::Center),
		);
		frame.render_widget(Clear, ctx.area);
		frame.render_widget(help, help_area);
		frame.render_widget(messages, content_area);

		// Field editor
		if let Some(editor) = &self.editor {
			let _title = format!(
				"{} > {}",
				self.entry.name,
				if let Some(selected) = self.selected {
					&self.entry.fields[selected].name
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

		// Confirm
		if let Some(confirm) = &self.confirm {
			confirm.render(frame, ctx);
		}
	}

	fn height(&self) -> u16 {
		panic!()
	}
}
