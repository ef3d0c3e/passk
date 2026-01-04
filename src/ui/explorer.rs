use core::panic;
use std::cell::RefCell;
use std::sync::LazyLock;

use chrono::Utc;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Constraint;
use ratatui::layout::Flex;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::Frame;

use crate::data::entry::Entry;
use crate::data::entry::EntryTag;
use crate::style::ENTRY_BG;
use crate::style::HELP_LINE_BG;
use crate::ui::entry::EntryEditor;
use crate::ui::entry_tag_editor::EntryTagEditor;
use crate::widgets::form::Form;
use crate::widgets::form::FormExt;
use crate::widgets::form::FormSignal;
use crate::widgets::label::LabelDisplay;
use crate::widgets::label::LabelStyle;
use crate::widgets::label::Labeled;
use crate::widgets::text_input::TextInput;
use crate::widgets::text_input::TextInputStyle;
use crate::widgets::widget::Component;
use crate::widgets::widget::ComponentRenderCtx;

#[derive(Default)]
pub struct ExplorerFilter {
	pub name: String,
	pub tags: Vec<String>,
}

impl From<&str> for ExplorerFilter {
	fn from(value: &str) -> Self {
		let mut filter = Self::default();
		// TODO..
		let rest = &value[..];
		while !rest.is_empty() {}
		filter
	}
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ActiveWidget {
	#[default]
	Search,
	Content,
}

static SEARCH_LABEL_STYLE: LazyLock<LabelStyle> = LazyLock::new(|| LabelStyle {
	padding: [0, 0],
	display: LabelDisplay::Block {
		block: Box::new(Block::bordered().border_type(ratatui::widgets::BorderType::Thick)),
	},
	style: Some(
		Style::default()
			.fg(Color::Black)
			.bg(Color::from_u32(0x241f31)),
	),
	style_selected: Some(
		Style::default()
			.fg(Color::Cyan)
			.bg(Color::from_u32(0x241f31)),
	),
});
static SEARCH_INPUT_STYLE: LazyLock<TextInputStyle> = LazyLock::new(|| TextInputStyle {
	padding: [0, 0],
	markers: ["".into(), "".into()],
	style: Some(
		Style::default()
			.fg(Color::White)
			.bg(Color::from_u32(0x241f31)),
	),
	style_selected: Some(
		Style::default()
			.fg(Color::Cyan)
			.bg(Color::from_u32(0x241f31)),
	),
});
static NEWENTRY_LABEL_STYLE: LazyLock<LabelStyle> = LazyLock::new(|| LabelStyle {
	padding: [0, 0],
	display: LabelDisplay::Block {
		block: Box::new(Block::bordered().border_type(ratatui::widgets::BorderType::Thick)),
	},
	style: Some(
		Style::default()
			.fg(Color::Black)
			.bg(Color::from_u32(0x241f31)),
	),
	style_selected: Some(
		Style::default()
			.fg(Color::Cyan)
			.bg(Color::from_u32(0x241f31)),
	),
});
static NEWENTRY_INPUT_STYLE: LazyLock<TextInputStyle> = LazyLock::new(|| TextInputStyle {
	padding: [0, 0],
	markers: ["".into(), "".into()],
	style: Some(
		Style::default()
			.fg(Color::White)
			.bg(Color::from_u32(0x241f31)),
	),
	style_selected: Some(
		Style::default()
			.fg(Color::Cyan)
			.bg(Color::from_u32(0x241f31)),
	),
});

pub struct Explorer {
	entries: Vec<Entry>,
	filtered_entries: Vec<usize>,
	active: ActiveWidget,
	selected: usize,

	filter_field: Labeled<'static, TextInput<'static>>,
	filter: ExplorerFilter,

	list_state: RefCell<ListState>,
	scrollbar: RefCell<ScrollbarState>,

	new_entry: Option<Labeled<'static, TextInput<'static>>>,
	editor: Option<EntryEditor>,
	tag_editor: Option<EntryTagEditor>,
}

impl Explorer {
	pub fn new(entries: Vec<Entry>) -> Self {
		let len = entries.len();
		Self {
			entries,
			filtered_entries: (0..len).collect(),
			active: Default::default(),
			selected: 0,
			filter_field: Labeled::new(
				"Filter".into(),
				TextInput::new().style(&SEARCH_INPUT_STYLE),
			)
			.style(&SEARCH_LABEL_STYLE),
			filter: Default::default(),
			list_state: RefCell::default(),
			scrollbar: RefCell::new(ScrollbarState::new(len).position(0)),
			new_entry: None,
			editor: None,
			tag_editor: None,
		}
	}

	fn move_cursor(&mut self, offset: i32) {
		if self.filtered_entries.is_empty() {
			self.list_state.borrow_mut().select(None);
			self.selected = 0;
			return;
		}
		if offset > 0 {
			self.selected = std::cmp::min(
				self.selected + offset as usize,
				self.filtered_entries.len() - 1,
			);
		} else if offset < 0 {
			self.selected = self.selected.saturating_sub((-offset) as usize);
		}

		let scrollbar = self.scrollbar.borrow().position(self.selected);
		*self.scrollbar.borrow_mut() = scrollbar;
	}

	fn update_filter(&mut self) {
		// TODO
		self.filtered_entries = (0..self.entries.len()).collect();
	}

	fn format_entry(ent: Option<&Entry>, selected: bool, id: usize) -> ListItem {
		fn format_tag(tag: &EntryTag) -> Span {
			let style = Style::default()
				.fg(Color::from_u32(tag.color.unwrap_or(0xDEA13B)))
				.italic();
			if let Some(icon) = &tag.icon {
				Span::styled(format!("+{} {icon}", tag.name), style)
			} else {
				Span::styled(format!("+{}", tag.name), style)
			}
		}

		let bg = ENTRY_BG[if selected { 2 } else { id % 2 }];
		let Some(ent) = ent else {
			return ListItem::from(Line::from("")).bg(bg);
		};
		let mut comp = vec![" ".into()];

		// Name
		let mut rest = &ent.name[..];
		loop {
			let Some(next) = rest.find(|c| c == '/') else {
				comp.push(Span::styled(rest, Style::default().fg(Color::Green).bold()));
				break;
			};
			comp.push(Span::styled(
				&rest[..next],
				Style::default().fg(Color::from_u32(0xafafaf)),
			));
			comp.push(Span::styled(
				"/",
				Style::default().fg(Color::from_u32(0xAf5f5f)).bold(),
			));
			rest = &rest[next + 1..]
		}

		comp.push(" ".into());
		// Fields
		comp.push(Span::styled(
			format!("({})", ent.fields.len()),
			Style::default().fg(Color::from_u32(0x4f4f4f)).italic(),
		));

		// Tags
		for tag in &ent.tags {
			comp.push(" ".into());
			comp.push(format_tag(tag));
		}

		ListItem::from(Line::from(comp)).bg(bg)
	}

	pub fn submit(&self) -> Vec<Entry> {
		self.entries.clone()
	}
}

impl Component for Explorer {
	fn input(&mut self, key: &KeyEvent) -> bool {
		// Entry editor
		if let Some(editor) = &mut self.editor {
			if !editor.input(key) {
				if let Some(ent) = editor.submit() {
					self.entries[self.selected] = ent;
				}
				self.editor = None;
			}
			return true;
		}
		// Tag editor
		if let Some(editor) = &mut self.tag_editor {
			match editor.input_form(key) {
				Some(FormSignal::Return) => {
					if let Some(tags) = editor.submit() {
						self.entries[self.selected].tags = tags;
					} else { /* TODO */
					};
					self.tag_editor = None
				}
				Some(FormSignal::Exit) => self.tag_editor = None,
				_ => {}
			}
			return true;
		}
		// New entry
		if let Some(new_entry) = &mut self.new_entry {
			if !new_entry.input(key) {
				let name = new_entry.inner.submit();
				let now = Utc::now();
				self.entries.push(Entry {
					name,
					fields: vec![],
					tags: vec![],
					created_at: now,
					modified_at: now,
					accessed_at: now,
				});
				self.new_entry = None;
				self.update_filter();
			}
			return true;
		}

		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		if self.active == ActiveWidget::Search {
			if self.filter_field.inner.input(key) {
				return true;
			}
			match key.code {
				KeyCode::Down | KeyCode::Tab | KeyCode::Esc => self.active = ActiveWidget::Content,
				KeyCode::Char('n') if ctrl_pressed => self.active = ActiveWidget::Content,
				_ => return false,
			}
			return true;
		}

		match key.code {
			KeyCode::Char('/') => self.active = ActiveWidget::Search,
			KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => self.move_cursor(1),
			KeyCode::Char('n') if ctrl_pressed => self.move_cursor(1),
			KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => self.move_cursor(-1),
			KeyCode::Char('p') if ctrl_pressed => self.move_cursor(-1),
			KeyCode::Char('e') | KeyCode::Enter => {
				if !self.entries.is_empty() {
					let ent = &self.entries[self.filtered_entries[self.selected]];
					self.editor = Some(EntryEditor::new(ent.clone()))
				}
			}
			KeyCode::Char('t') => {
				if !self.entries.is_empty() {
					let ent = &self.entries[self.filtered_entries[self.selected]];
					self.tag_editor = Some(EntryTagEditor::new(
						format!("Tags for {}", ent.name),
						&ent.tags,
					))
				}
			}
			KeyCode::Char('a') => {
				self.new_entry = Some(
					Labeled::new(
						"New Entry".into(),
						TextInput::new().style(&NEWENTRY_INPUT_STYLE),
					)
					.style(&NEWENTRY_LABEL_STYLE),
				);
			}
			_ => return false,
		}
		true
	}

	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let area = ctx.area;
		frame.render_widget(Clear, area);

		// Help bar
		let help = Line::from(vec![
			" PassK 0.1 ".bold().fg(Color::Red),
			"⮁".bold().fg(Color::Green),
			" (navigate) ".fg(Color::White),
			"/".bold().fg(Color::Green),
			" (filter) ".fg(Color::White),
			"a".bold().fg(Color::Green),
			" (add) ".fg(Color::White),
			"esc".bold().fg(Color::Green),
			" (cancel) ".fg(Color::White),
			"enter".bold().fg(Color::Green),
			" (open) ".fg(Color::White),
		])
		.bg(HELP_LINE_BG);
		let mut help_area = area;
		help_area.height = 1;
		ctx.selected = self.active == ActiveWidget::Search;
		frame.render_widget(help, help_area);

		// Filter
		let mut filter_area = area;
		filter_area.y += 1;
		filter_area.height = self.filter_field.height();
		ctx.area = filter_area;
		self.filter_field.render(frame, ctx);

		// Entries
		let mut ent_area = area;
		ent_area.y += filter_area.y + filter_area.height;
		ent_area.height = area.height.saturating_sub(ent_area.y);
		ent_area.width = ent_area.width.saturating_sub(1);

		let mut items = self
			.filtered_entries
			.iter()
			.map(|i| {
				(
					self.active == ActiveWidget::Content && *i == self.selected,
					&self.entries[*i],
				)
			})
			.enumerate()
			.map(|(id, (selected, ent))| Self::format_entry(Some(ent), selected, id))
			.collect::<Vec<_>>();
		while items.len() < ent_area.height as usize {
			items.push(Self::format_entry(None, false, items.len()));
		}

		let scroll_offset = (self.selected + 1)
			.saturating_sub(ent_area.height as usize)
			.min(
				self.filtered_entries
					.len()
					.saturating_sub(ent_area.height as usize),
			);
		let mut list_state = self.list_state.borrow_mut();
		list_state.select(Some(self.selected));

		*self.scrollbar.borrow_mut() = ScrollbarState::new(
			self.filtered_entries
				.len()
				.saturating_sub(ent_area.height as usize)
				.max(1),
		)
		.position(scroll_offset);
		frame.render_stateful_widget(List::new(items), ent_area, &mut *list_state);

		// Scrollbar
		let mut scrollbar_area = ent_area;
		scrollbar_area.x = area.width.saturating_sub(1);
		scrollbar_area.width = 1;

		frame.render_stateful_widget(
			Scrollbar::default()
				.orientation(ScrollbarOrientation::VerticalRight)
				.style(Style::default().fg(Color::from_u32(0x7f7faf))),
			scrollbar_area,
			&mut *self.scrollbar.borrow_mut(),
		);

		// Editor
		ctx.area = area;
		if let Some(editor) = &self.editor {
			editor.render(frame, ctx);
		}
		// Tag Editor
		if let Some(editor) = &self.tag_editor {
			let horizontal = Layout::horizontal([Constraint::Percentage(40)]).flex(Flex::Center);
			let vertical =
				Layout::vertical([Constraint::Length(editor.height())]).flex(Flex::Center);
			let [area] = ctx.area.layout(&horizontal);
			let [area] = area.layout(&vertical);
			ctx.area = area;
			editor.render_form(frame, ctx);
		}
		// New entry
		if let Some(new_editor) = &self.new_entry {
			let horizontal = Layout::horizontal([Constraint::Percentage(40)]).flex(Flex::Center);
			let vertical = Layout::vertical([Constraint::Length(3)]).flex(Flex::Center);
			let [area] = ctx.area.layout(&horizontal);
			let [area] = area.layout(&vertical);
			ctx.area = area;
			ctx.selected = true;
			new_editor.render(frame, ctx);
		}
	}

	fn height(&self) -> u16 {
		panic!()
	}
}
