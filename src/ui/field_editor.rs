use std::cell::RefCell;
use std::default;
use std::sync::LazyLock;

use crate::data::field::Field;
use crate::data::field::FieldValue;
use crate::widgets::checkbox::Checkbox;
use crate::widgets::checkbox::CheckboxStyle;
use crate::widgets::combo_box::ComboBox;
use crate::widgets::combo_box::ComboBoxStyle;
use crate::widgets::combo_box::ComboItem;
use crate::widgets::form::Form;
use crate::widgets::form::FormEvent;
use crate::widgets::form::FormExt;
use crate::widgets::form::FormSignal;
use crate::widgets::form::FormStyle;
use crate::widgets::label::LabelDisplay;
use crate::widgets::label::LabelStyle;
use crate::widgets::label::Labeled;
use crate::widgets::text_input::TextInput;
use crate::widgets::text_input::TextInputStyle;
use crate::widgets::widget::Component;
use crate::widgets::widget::ComponentRenderCtx;
use crate::widgets::widget::ComponentVisitor;
use color_eyre::eyre::Error;
use color_eyre::owo_colors::OwoColorize;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::symbols::border::QUADRANT_OUTSIDE;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use serde_json::Value;

static FIELD_TYPE: LazyLock<[ComboItem; 7]> = LazyLock::new(|| {
	[
		ComboItem {
			kind: "Text".into(),
			icon: "󰅍 ".into(),
			value: "Text".into(),
		},
		ComboItem {
			kind: "Text".into(),
			icon: " ".into(),
			value: "URL".into(),
		},
		ComboItem {
			kind: "Text".into(),
			icon: "󰥒 ".into(),
			value: "Phone Number".into(),
		},
		ComboItem {
			kind: "Text".into(),
			icon: "󰇰 ".into(),
			value: "E-Mail".into(),
		},
		ComboItem {
			kind: "2FA".into(),
			icon: "󰐲 ".into(),
			value: "TOTP/Steam".into(),
		},
		ComboItem {
			kind: "2FA".into(),
			icon: "󰐲 ".into(),
			value: "TOTP/RFC 6238".into(),
		},
		ComboItem {
			kind: "2FA".into(),
			icon: "󰦯 ".into(),
			value: "2FA Recovery".into(),
		},
	]
});

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FieldValueKind {
	#[default]
	Text,
	Url,
	Phone,
	Mail,
	TOTPRFC6238,
	TOTPSteam,
	TwoFactorRecovery,
	Binary,
}

impl TryFrom<usize> for FieldValueKind {
	type Error = &'static str;

	fn try_from(value: usize) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(FieldValueKind::Text),
			1 => Ok(FieldValueKind::Url),
			2 => Ok(FieldValueKind::Phone),
			3 => Ok(FieldValueKind::Mail),
			4 => Ok(FieldValueKind::TOTPRFC6238),
			5 => Ok(FieldValueKind::TOTPSteam),
			6 => Ok(FieldValueKind::TwoFactorRecovery),
			7 => Ok(FieldValueKind::Binary),
			_ => Err("Invalid value"),
		}
	}
}

impl FieldValueKind {
	fn id(&self) -> usize {
		match self {
			FieldValueKind::Text => 0,
			FieldValueKind::Url => 1,
			FieldValueKind::Phone => 2,
			FieldValueKind::Mail => 3,
			FieldValueKind::TOTPRFC6238 => 4,
			FieldValueKind::TOTPSteam => 5,
			FieldValueKind::TwoFactorRecovery => 6,
			FieldValueKind::Binary => 7,
		}
	}

	fn name(&self) -> &'static str {
		match self {
			FieldValueKind::Text => "Text",
			FieldValueKind::Url => "URL",
			FieldValueKind::Phone => "Phone",
			FieldValueKind::Mail => "E-Mail",
			FieldValueKind::TOTPRFC6238 => "TOTP (RFC-6238)",
			FieldValueKind::TOTPSteam => "TOTP (Steam)",
			FieldValueKind::TwoFactorRecovery => "2FA Recovery",
			FieldValueKind::Binary => "Binary",
		}
	}
}

pub struct FieldEditor {
	title: String,
	style: FormStyle,

	// Form data
	field_name: Labeled<'static, TextInput<'static>>,
	field_hidden: Checkbox<'static>,
	field_type: Labeled<'static, ComboBox<'static, 'static>>,

	value_kind: Option<FieldValueKind>,
	prev_value_kind: Option<FieldValueKind>,
	field_value: Option<Labeled<'static, TextInput<'static>>>,

	selected: Option<usize>,
	scroll: RefCell<u16>,
}

static LABEL_STYLE: LazyLock<LabelStyle> = LazyLock::new(|| LabelStyle {
	padding: [0, 0],
	display: LabelDisplay::Block {
		block: Block::bordered(),
	},
	style: Some(Style::default().fg(Color::White)),
	style_selected: None,
});
static TEXTINPUT_STYLE: LazyLock<TextInputStyle> = LazyLock::new(|| TextInputStyle {
	padding: [0, 0],
	markers: ["".into(), "".into()],
	style: Some(Style::default().fg(Color::White)),
	selected_style: None,
});
static CHECKBOX_STYLE: LazyLock<CheckboxStyle> = LazyLock::new(|| CheckboxStyle {
	padding: [1, 0],
	spacing: 1,
	markers: ["󰄱 ".into(), "󰄵 ".into()],
	style: Some(Style::default().fg(Color::White)),
	selected_style: None,
});
static COMBOBOX_STYLE: LazyLock<ComboBoxStyle> = LazyLock::new(|| ComboBoxStyle {
	padding: Default::default(),
	markers: ["".into(), "".into()],
	indicator: [" ".into(), " ".into()],
	completion: [
		Style::default().bg(Color::Cyan).fg(Color::Black),
		Style::default().bg(Color::Black).fg(Color::White).bold(),
		Style::default().bg(Color::Black).fg(Color::White).italic(),
	],
	completion_selected: [
		Style::default().bg(Color::Cyan).fg(Color::Black),
		Style::default().bg(Color::Black).fg(Color::Yellow).bold(),
		Style::default().bg(Color::Black).fg(Color::Yellow).italic(),
	],
	style: Default::default(),
	selected_style: Default::default(),
});

impl FieldEditor {
	pub fn new(title: String) -> Self {
		Self {
			title,
			style: FormStyle {
				bg: Color::from_u32(0x2f2f2f),
			},
			field_name: Labeled::new(Span::from("Name"), TextInput::new().style(&TEXTINPUT_STYLE))
				.style(&LABEL_STYLE),
			field_hidden: Checkbox::new(false, Span::from("Hidden")).style(&CHECKBOX_STYLE),
			field_type: Labeled::new(
				Span::from("Type"),
				ComboBox::new(FIELD_TYPE.as_slice()).style(&COMBOBOX_STYLE),
			)
			.style(&LABEL_STYLE),

			value_kind: None,
			prev_value_kind: None,
			field_value: None,
			selected: None,
			scroll: RefCell::default(),
		}
	}

	pub fn with_value(mut self, field: &Field) -> Self {
		self.field_name.inner.set_input(field.name.clone());
		self.field_hidden.set_value(field.hidden);
		let kind = match &field.value {
			FieldValue::Text(text) => {
				let kind = FieldValueKind::Text;
				self.field_value = Some(
					Labeled::new(
						kind.name().into(),
						TextInput::new()
							.style(&TEXTINPUT_STYLE)
							.with_input(text.clone()),
					)
					.style(&LABEL_STYLE),
				);
				kind
			}
			FieldValue::Url(text) => {
				let kind = FieldValueKind::Url;
				self.field_value = Some(
					Labeled::new(
						kind.name().into(),
						TextInput::new()
							.style(&TEXTINPUT_STYLE)
							.with_input(text.clone()),
					)
					.style(&LABEL_STYLE),
				);
				kind
			},
			FieldValue::Phone(text) => {
				let kind = FieldValueKind::Phone;
				self.field_value = Some(
					Labeled::new(
						kind.name().into(),
						TextInput::new()
							.style(&TEXTINPUT_STYLE)
							.with_input(text.clone()),
					)
					.style(&LABEL_STYLE),
				);
				kind
			},
			FieldValue::Email(text) => {
				let kind = FieldValueKind::Mail;
				self.field_value = Some(
					Labeled::new(
						kind.name().into(),
						TextInput::new()
							.style(&TEXTINPUT_STYLE)
							.with_input(text.clone()),
					)
					.style(&LABEL_STYLE),
				);
				kind
			},
			_ => todo!(),
		};
		self.field_type.inner.set_input(kind.name().to_owned());
		self.value_kind = Some(kind);
		self.prev_value_kind = Some(kind);
		self
	}
}

impl Form for FieldEditor {
	type Return = bool;

	fn component_count(&self) -> usize {
		match self.value_kind {
			Some(_) => 4,
			None => 3,
		}
	}

	fn component(&self, id: usize) -> Option<&dyn Component> {
		match id {
			0 => Some(&self.field_name),
			1 => Some(&self.field_hidden),
			2 => Some(&self.field_type),
			3 => {
				if let Some(field) = &self.field_value {
					Some(field)
				} else {
					None
				}
			}
			_ => None,
		}
	}

	fn component_mut(&mut self, id: usize) -> Option<&mut dyn Component> {
		match id {
			0 => Some(&mut self.field_name),
			1 => Some(&mut self.field_hidden),
			2 => Some(&mut self.field_type),
			3 => {
				if let Some(field) = &mut self.field_value {
					Some(field)
				} else {
					None
				}
			}
			_ => None,
		}
	}

	fn get_style(&self) -> &FormStyle {
		&self.style
	}

	fn selected(&self) -> Option<usize> {
		self.selected
	}

	fn set_selected(&mut self, selected: Option<usize>) {
		self.selected = selected;
	}

	fn scroll(&self) -> u16 {
		*self.scroll.borrow()
	}

	fn set_scroll(&self, scroll: u16) {
		*self.scroll.borrow_mut() = scroll;
	}

	fn event(&mut self, ev: FormEvent) -> Option<FormSignal<Self::Return>> {
		match ev {
			FormEvent::Key { key } => {
				if key.code == KeyCode::Esc {
					return Some(FormSignal::Exit);
				} else if key.code == KeyCode::Enter {
					return Some(FormSignal::Return(true));
				}
			}
			FormEvent::Edit { id: 2, key: _ } => {
				if let Some(Ok(kind)) = self
					.field_type
					.inner
					.submit()
					.map(|id| FieldValueKind::try_from(id))
				{
					if Some(kind) != self.prev_value_kind 
					{
						self.prev_value_kind = self.value_kind;
						self.value_kind = Some(kind);
						match kind {
							FieldValueKind::Text
								| FieldValueKind::Url
								| FieldValueKind::Phone
								| FieldValueKind::Mail => {
									self.field_value = Some(
										Labeled::new(
											kind.name().into(),
											TextInput::new().style(&TEXTINPUT_STYLE),
										)
										.style(&LABEL_STYLE),
									)
								}
							_ => todo!(),
						}
					}
				} else {
					self.prev_value_kind = self.value_kind;
					self.value_kind = None;
					self.field_value = None;
				}
			}
			_ => {}
		}
		None
	}

	fn render_form(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let area = ctx.area;
		let border = Block::bordered()
			.border_set(QUADRANT_OUTSIDE)
			.title(self.title.as_str())
			.title_style(Style::default().fg(Color::White))
			.title_alignment(ratatui::layout::HorizontalAlignment::Center)
			.bg(self.style.bg)
			.fg(Color::from_u32(0x1a1a1f));
		frame.render_widget(border, area);
		let text = Text::from(Line::from(vec![
			"⮁".bold().fg(Color::Green),
			" (navigate) ".fg(Color::White),
			"esc".bold().fg(Color::Green),
			" (cancel) ".fg(Color::White),
			"enter".bold().fg(Color::Green),
			" (submit) ".fg(Color::White),
			"space".bold().fg(Color::Green),
			" (toggle) ".fg(Color::White),
			"C-g".bold().fg(Color::Green),
			" (generate) ".fg(Color::White),
		]));
		let help_message = Paragraph::new(text);
		frame.render_widget(
			help_message,
			Rect {
				x: area.x + 1,
				y: area.y + 1,
				width: area.width.saturating_sub(2),
				height: 1,
			},
		);

		ctx.area.x += 1;
		ctx.area.width = ctx.area.width.saturating_sub(2);
		ctx.area.y += 2;
		ctx.area.height = ctx.area.height.saturating_sub(3);
		self.render_body(frame, ctx);
	}
}

/*
pub struct FieldEditor<'s> {
	title: Line<'s>,

	active: ActiveField,
	name: TextInput<'s>,
	hidden: bool,
	value_type: ComboBox<'s, 'static>,
	prev_value_type: i32,
	value: Vec<TextInput<'s>>,
	//generator: Option<TextInput<'s>>,
	scrollbar: RefCell<ScrollbarState>,
}

impl<'s> FieldEditor<'s> {
	pub fn new(title: Line<'s>) -> Self {
		let mut s = Self {
			title,
			active: ActiveField::None,
			name: TextInput::new(Line::from(vec!["Name".into()]), Constraint::Percentage(100)),
			hidden: false,
			value_type: ComboBox::new(
				Line::from(vec!["Type".into()]),
				Constraint::Percentage(100),
				&*FIELD_TYPE,
			),
			prev_value_type: -1,
			value: vec![],
			scrollbar: RefCell::new(ScrollbarState::new(0).position(0)),
		};
		s.update_scrollbar();
		s
	}

	pub fn with_field(mut self, field: &Field) -> Self {
		self.set_value_type(Some(&field.value));
		self.hidden = field.hidden;
		self.name.set_input(field.name.clone());
		self.prev_value_type = field.value.get_id() as i32;
		self.value_type
			.set_input(FIELD_TYPE[field.value.get_id()].value.clone());
		self
	}

	pub fn update_scrollbar(&mut self) {
		let height = 1 + 3 * (2 + self.value.len());
		*self.scrollbar.borrow_mut() = ScrollbarState::new(height);
	}

	fn set_value_type(&mut self, value_type: Option<&FieldValue>) {
		let Some(value_type) = value_type else {
			self.value.clear();
			self.update_scrollbar();
			return;
		};

		match value_type {
			FieldValue::Text(text) => {
				self.value = vec![TextInput::new(
					Line::from(vec!["Text".into()]),
					Constraint::Percentage(100),
				)
				.with_input(text.to_owned())];
			}
			FieldValue::Url(url) => {
				self.value = vec![TextInput::new(
					Line::from(vec!["URL".into()]),
					Constraint::Percentage(100),
				)
				.with_input(url.to_owned())];
			}
			FieldValue::Phone(phone) => {
				self.value = vec![TextInput::new(
					Line::from(vec!["Phone Number".into()]),
					Constraint::Percentage(100),
				)
				.with_input(phone.to_owned())];
			}
			FieldValue::Email(email) => {
				self.value = vec![TextInput::new(
					Line::from(vec!["E-Mail".into()]),
					Constraint::Percentage(100),
				)
				.with_input(email.to_owned()),
				TextInput::new(
					Line::from(vec!["Foo".into()]),
					Constraint::Percentage(100),
					),
				TextInput::new(
					Line::from(vec!["Bar".into()]),
					Constraint::Percentage(100),
					),
				TextInput::new(
					Line::from(vec!["Baz".into()]),
					Constraint::Percentage(100),
					),
				TextInput::new(
					Line::from(vec!["Quz".into()]),
					Constraint::Percentage(100),
					),
				TextInput::new(
					Line::from(vec!["Qux".into()]),
					Constraint::Percentage(100),
					)
					];
			}
			FieldValue::TOTPRFC6238(_) => todo!(),
			FieldValue::TOTPSteam(_) => todo!(),
			FieldValue::TwoFactorRecovery(two_facodes) => todo!(),
			FieldValue::Binary { mimetype, base64 } => todo!(),
		}
		self.update_scrollbar();
	}

	pub fn move_cursor(&mut self, offset: i32) {
		match self.active {
			ActiveField::None => {}
			ActiveField::Name => self.name.set_active(false),
			ActiveField::Type => self.value_type.set_active(false),
			ActiveField::Hidden => {}
			ActiveField::Values(i) => self.value[i].set_active(false),
		}
		if offset == 1 {
			self.active = match self.active {
				ActiveField::None => ActiveField::Name,
				ActiveField::Name => ActiveField::Type,
				ActiveField::Type => ActiveField::Hidden,
				ActiveField::Hidden => {
					if !self.value.is_empty() {
						ActiveField::Values(0)
					} else {
						ActiveField::Hidden
					}
				}
				ActiveField::Values(i) => {
					if self.value.len() > i + 1 {
						ActiveField::Values(i + 1)
					} else {
						ActiveField::Values(i)
					}
				}
			};
		} else if offset == -1 {
			self.active = match self.active {
				ActiveField::None => ActiveField::None,
				ActiveField::Name => ActiveField::Name,
				ActiveField::Type => ActiveField::Name,
				ActiveField::Hidden => ActiveField::Type,
				ActiveField::Values(i) => {
					if i == 0 {
						ActiveField::Hidden
					} else {
						ActiveField::Values(i - 1)
					}
				}
			};
		}
		match self.active {
			ActiveField::None => {}
			ActiveField::Name => self.name.set_active(true),
			ActiveField::Type => self.value_type.set_active(true),
			ActiveField::Hidden => {}
			ActiveField::Values(i) => {
				self.value[i].set_active(true);
			}
		}
	}

	pub fn input(&mut self, key: &KeyEvent) -> Option<Option<(String, bool, FieldValue)>> {
		match self.active {
			ActiveField::None => {}
			ActiveField::Name => self.name.input(key),
			ActiveField::Type => {
				self.value_type.input(key);
				if self.value_type.is_completing() {
					return None;
				}
				if let Some(idx) = self.value_type.submit() {
					if idx as i32 != self.prev_value_type {
						self.prev_value_type = idx as i32;
						match idx {
							0 => self.set_value_type(Some(&FieldValue::Text(String::default()))),
							1 => self.set_value_type(Some(&FieldValue::Url(String::default()))),
							2 => self.set_value_type(Some(&FieldValue::Phone(String::default()))),
							3 => self.set_value_type(Some(&FieldValue::Email(String::default()))),
							_ => {
								self.set_value_type(None);
							}
						}
					}
				}
			}
			ActiveField::Hidden => {
				if key.code == KeyCode::Char(' ') {
					self.hidden = !self.hidden;
					return None;
				}
			}
			ActiveField::Values(i) => self.value[i].input(key),
		}

		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		match key.code {
			KeyCode::Up => self.move_cursor(-1),
			KeyCode::Char('p') if ctrl_pressed => self.move_cursor(-1),
			KeyCode::Down => self.move_cursor(1),
			KeyCode::Char('n') if ctrl_pressed => self.move_cursor(1),

			KeyCode::Enter => {
				// TODO: Validate
				match self.value_type.submit() {
					Some(0) => {
						return Some(Some((
							self.name.submit(),
							self.hidden,
							FieldValue::Text(self.value[0].submit()),
						)))
					}
					Some(1) => {
						return Some(Some((
							self.name.submit(),
							self.hidden,
							FieldValue::Url(self.value[0].submit()),
						)))
					}
					Some(2) => {
						return Some(Some((
							self.name.submit(),
							self.hidden,
							FieldValue::Phone(self.value[0].submit()),
						)))
					}
					Some(3) => {
						return Some(Some((
							self.name.submit(),
							self.hidden,
							FieldValue::Email(self.value[0].submit()),
						)))
					}
					_ => return Some(None),
				}
			}
			KeyCode::Esc => return Some(None),
			_ => {}
		}
		None
	}

	pub fn draw(&self, frame: &mut Frame, area: Rect) {
		frame.render_widget(Clear, area);

		let vertical = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]);
		let inner = Rect {
			x: area.x + 1,
			y: area.y + 1,
			width: area.width - 2,
			height: area.height - 2,
		};
		let [help_area, content_area] = vertical.areas(inner);

		let help = Line::from(vec![
			" ⮁".bold().fg(Color::Green),
			" (navigate) ".into(),
			"esc".bold().fg(Color::Green),
			" (cancel) ".into(),
			"enter".bold().fg(Color::Green),
			" (submit) ".into(),
			"space".bold().fg(Color::Green),
			" (toggle) ".into(),
			"C-g".bold().fg(Color::Green),
			" (generate) ".into(),
		]);
		frame.render_widget(help, help_area);

		let border = Block::bordered()
			.border_set(QUADRANT_OUTSIDE)
			.fg(Color::Black);
		frame.render_widget(border, area);

		// Name
		self.name.draw(frame, content_area, Some(Color::Black));
		// Value type
		self.value_type
			.draw(frame, content_area.offset(Offset::new(0, 3)), Some(Color::Black));
		// Checkbox
		let checkbox = Line::from(vec![
			" ".into(),
			["[ ]", "[x]"][self.hidden as usize].bold(),
			" Hidden".into(),
		]);
		if self.active == ActiveField::Hidden {
			frame.render_widget(
				checkbox.fg(Color::Yellow),
				content_area.offset(Offset::new(0, 6)),
			);
		} else {
			frame.render_widget(checkbox, content_area.offset(Offset::new(0, 6)));
		}

		let mut yoff = 7;
		for widget in &self.value {
			widget.draw(frame, content_area.offset(Offset::new(0, yoff)), Some(Color::Black));
			yoff += 3;
		}
	}
}
*/

/*
/* Name   : [.....]
 * Value  : [.....]
 * Hidden : [x]
 */
impl<'s> FieldEditor<'s> {
	pub fn new(title: Line<'s>) -> Self {
		let vertical = Layout::vertical([Constraint::Length(3 * 2 + 2)]);
		let horizontal =
			Layout::horizontal([Constraint::Percentage(100)]).flex(Flex::Center);
		Self {
			name: TextInput::new(Line::from("Name"), Constraint::Percentage(100)),
			value: TextInput::new(Line::from("Value"), Constraint::Percentage(100)),
			hidden: false,
			title,
			layout: [horizontal, vertical],
			active: ActiveField::default(),
			generator: None,
		}
	}

	pub fn with_field(mut self, field: &Field) -> Self {
		self.name = self.name.with_input(field.name.clone());
		self.value = self.value.with_input(field.value.clone());
		self
	}

	pub fn set_active(&mut self, active: ActiveField) {
		match self.active {
			ActiveField::Name => self.name.set_active(false),
			ActiveField::Value => self.value.set_active(false),
			_ => {}
		}
		self.active = active;
		match self.active {
			ActiveField::Name => self.name.set_active(true),
			ActiveField::Value => self.value.set_active(true),
			_ => {}
		}
	}

	fn submit(&mut self) -> Field {
		todo!();
		//Field {
		//	name: self.name.submit(),
		//	value: self.value.submit(),
		//	hidden: self.hidden,
		//}
	}

	pub fn input(&mut self, key: &KeyEvent) -> Option<Option<Field>> {
		// Password generator
		if let Some(generator) = &mut self.generator {
			if key.code == KeyCode::Enter {
				if let Ok(length) = generator.submit().parse::<i32>() {
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
					let mut generator =
						TextInput::new(Line::from("Length"), Constraint::Percentage(100))
							.with_input("64".into());
					generator.set_active(true);
					self.generator = Some(generator)
				}
				_ => {}
			},
			KeyCode::Enter => {
				let field = self.submit();
				if field.name.trim().is_empty() {
					return Some(None);
				}
				return Some(Some(field));
			}
			KeyCode::Esc => return Some(None),
			_ => match self.active {
				ActiveField::Name => self.name.input(key),
				ActiveField::Value => self.value.input(key),
				_ => {}
			},
		}
		return None;
	}

	pub fn draw(&self, frame: &mut Frame, rect: Rect) {
		// Field generator
		if let Some(generator) = &self.generator {
			let mut area = rect;
			area.height = 3;
			generator.draw(frame, area);
			return;
		}

		let boxed = Block::bordered().title(self.title.clone());
		frame.render_widget(boxed, rect);

		let mut area = rect;
		area.width -= 2;
		area.height -= 2;
		let [area] = area.layout(&self.layout[0]);
		let [area] = area.layout(&self.layout[1]);
		let text = Text::from(Line::from(vec![
			"⮁".bold().fg(Color::Green),
			" (navigate) ".into(),
			"S-⮁".bold().fg(Color::Green),
			" (order) ".into(),
			"esc".bold().fg(Color::Green),
			" (cancel) ".into(),
			"enter".bold().fg(Color::Green),
			" (submit) ".into(),
			"space".bold().fg(Color::Green),
			" (toggle) ".into(),
			"C-g".bold().fg(Color::Green),
			" (generate) ".into(),
		]));
		let help_message = Paragraph::new(text);
		frame.render_widget(help_message, area.offset(Offset::new(0, 1)));
		self.name.draw(frame, area.offset(Offset::new(0, 2)));
		self.value.draw(frame, area.offset(Offset::new(0, 5)));

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
*/
