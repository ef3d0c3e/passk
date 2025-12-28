use std::cell::RefCell;
use std::sync::LazyLock;

use crate::data::field::Field;
use crate::data::field::FieldValue;
use crate::ui::field_generator::FieldGenerator;
use crate::widgets::checkbox::Checkbox;
use crate::widgets::checkbox::CheckboxStyle;
use crate::widgets::combo_box::ComboBox;
use crate::widgets::combo_box::ComboBoxStyle;
use crate::widgets::combo_box::ComboItem;
use crate::widgets::form::Form;
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
use chrono::DateTime;
use chrono::Utc;
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
	EMail,
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
			3 => Ok(FieldValueKind::EMail),
			4 => Ok(FieldValueKind::TOTPRFC6238),
			5 => Ok(FieldValueKind::TOTPSteam),
			6 => Ok(FieldValueKind::TwoFactorRecovery),
			7 => Ok(FieldValueKind::Binary),
			_ => Err("Invalid value"),
		}
	}
}

impl FieldValueKind {
	fn name(&self) -> &'static str {
		match self {
			FieldValueKind::Text => "Text",
			FieldValueKind::Url => "URL",
			FieldValueKind::Phone => "Phone Number",
			FieldValueKind::EMail => "E-Mail",
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

	created_at: DateTime<Utc>,

	// Form data
	field_name: Labeled<'static, TextInput<'static>>,
	field_hidden: Checkbox<'static>,
	field_type: Labeled<'static, ComboBox<'static, 'static>>,

	value_kind: Option<FieldValueKind>,
	prev_value_kind: Option<FieldValueKind>,
	field_value: Option<Labeled<'static, TextInput<'static>>>,

	selected: Option<usize>,
	scroll: RefCell<u16>,

	generator: Option<FieldGenerator>,
}

static LABEL_STYLE: LazyLock<LabelStyle> = LazyLock::new(|| LabelStyle {
	padding: [0, 0],
	display: LabelDisplay::Block {
		block: Box::new(Block::bordered()),
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
			created_at: Utc::now(),
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
			generator: None,
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
			}
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
			}
			FieldValue::Email(text) => {
				let kind = FieldValueKind::EMail;
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
			_ => todo!(),
		};
		self.created_at = field.date_added;
		self.field_type.inner.set_input(kind.name().to_owned());
		self.value_kind = Some(kind);
		self.prev_value_kind = Some(kind);
		self.selected = Some(3);
		self
	}

	pub fn submit(&self) -> Option<Field> {
		let kind = self.value_kind?;
		let now = Utc::now();

		Some(Field {
			name: self.field_name.inner.submit(),
			value: match kind {
				FieldValueKind::Text => {
					FieldValue::Text(self.field_value.as_ref().unwrap().inner.submit())
				}
				FieldValueKind::Url => {
					FieldValue::Url(self.field_value.as_ref().unwrap().inner.submit())
				}
				FieldValueKind::Phone => {
					FieldValue::Phone(self.field_value.as_ref().unwrap().inner.submit())
				}
				FieldValueKind::EMail => {
					FieldValue::Email(self.field_value.as_ref().unwrap().inner.submit())
				}
				_ => todo!(),
			},
			hidden: self.field_hidden.value(),
			date_added: self.created_at,
			date_modified: now,
			date_accessed: now,
		})
	}
}

impl Form for FieldEditor {
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

	fn input_form(&mut self, key: &KeyEvent) -> Option<FormSignal> {
		// Subform
		if let Some(generator) = &mut self.generator {
			let signal = generator.input_form(key);
			match signal {
				Some(FormSignal::Exit) => self.generator = None,
				Some(FormSignal::Return) => {
					if let Some(generated) =  generator.submit()
					{
						if self.selected == Some(0) {
							self.field_name.inner.set_input(generated);
						}
						else if self.selected == Some(3) {
							if let Some(field) = &mut self.field_value {
									field.inner.set_input(generated);
							}
						}
					}
					self.generator = None 
				},
				_ => {}
			}
			return None;
		}

		// Dispatch input to components
		if FormExt::input(self, key) {
			// Update value kind
			if self.selected == Some(2) {
				if let Some(Ok(kind)) = self.field_type.inner.submit().map(FieldValueKind::try_from)
				{
					if Some(kind) != self.prev_value_kind {
						self.prev_value_kind = self.value_kind;
						self.value_kind = Some(kind);
						match kind {
							FieldValueKind::Text
							| FieldValueKind::Url
							| FieldValueKind::Phone
							| FieldValueKind::EMail => {
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
			return None;
		}

		// Quit
		if key.code == KeyCode::Esc {
			return Some(FormSignal::Exit);
		} else if key.code == KeyCode::Enter {
			return Some(FormSignal::Return);
		}

		// Generator
		let ctrl_pressed = key.modifiers.contains(KeyModifiers::CONTROL);
		if ctrl_pressed
			&& key.code == KeyCode::Char('g')
			&& (self.selected == Some(0) || self.selected == Some(3))
		{
			let name = if self.selected == Some(0) { "Name" } else {
				self.value_kind.unwrap().name()
			};
			self.generator = Some(FieldGenerator::new(format!("Generate for {name}")))
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

		if let Some(generator) = &self.generator {
			ctx.with_child(|ctx| {
				ctx.area.x += 1;
				ctx.area.width = ctx.area.width.saturating_sub(3);
				ctx.area.y += 2;
				ctx.area.height = ctx.area.height.saturating_sub(3);
				generator.render_form(frame, ctx);
			});
		}
	}
}
