use std::sync::LazyLock;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::layout::Constraint;
use ratatui::layout::Flex;
use ratatui::layout::HorizontalAlignment;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_segmentation::UnicodeSegmentation;

use crate::widgets::checkbox::Checkbox;
use crate::widgets::form::Form;
use crate::widgets::form::FormExt;
use crate::widgets::form::FormSignal;
use crate::widgets::form::FormStyle;
use crate::widgets::label::LabelDisplay;
use crate::widgets::label::LabelStyle;
use crate::widgets::label::Labeled;
use crate::widgets::popup::Popup;
use crate::widgets::text_input::TextInput;
use crate::widgets::text_input::TextInputStyle;
use crate::widgets::text_input_custom::CustomTextInput;
use crate::widgets::text_input_custom::CustomTextInputStyle;
use crate::widgets::text_input_custom::TextFormatter;
use crate::widgets::widget::Component;
use crate::widgets::widget::ComponentRenderCtx;

static PASSWORD_LABEL_STYLE: LazyLock<LabelStyle> = LazyLock::new(|| LabelStyle {
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
static PASSWORD_INPUT_STYLE: LazyLock<CustomTextInputStyle> =
	LazyLock::new(|| CustomTextInputStyle {
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

fn block(title: String) -> Block<'static> {
	Block::bordered()
		.title(title)
		.title_alignment(HorizontalAlignment::Center)
		.border_type(BorderType::QuadrantOutside)
		.border_style(Style::default().fg(Color::from_u32(0x7f7f7f)))
}

struct PasswordFormatter {
	hidden: bool,
}

impl<'s> TextFormatter<'s> for PasswordFormatter {
	fn format(&self, input: &str) -> Vec<ratatui::prelude::Span<'s>> {
		input
			.graphemes(true)
			.map(|gr| {
				if self.hidden {
					Span::raw("*".to_owned())
				} else {
					Span::raw(gr.to_owned())
				}
			})
			.collect()
	}
}

pub struct PasswordPrompt {
	style: FormStyle,
	db_name: String,
	new_password: bool,

	selected: usize,
	input: Labeled<'static, CustomTextInput<'static, PasswordFormatter>>,
	hidden: Checkbox<'static>,

	popup: Option<Popup<'static>>,
	block: Block<'static>,
	password: Option<String>,
	has_confirmation: bool,
}

impl PasswordPrompt {
	pub fn new(db_name: String, new_password: bool) -> Self {
		let title = format!("Password for '{}'", db_name);
		Self {
			style: FormStyle {
				border: true,
				bg: Color::from_u32(0x1f1f1f),
			},
			db_name,
			new_password,
			selected: 0,
			input: Labeled::new(
				"Password".into(),
				CustomTextInput::new(PasswordFormatter { hidden: true })
					.style(&PASSWORD_INPUT_STYLE),
			)
			.style(&PASSWORD_LABEL_STYLE),
			hidden: Checkbox::new(true, "Hidden".into()),
			popup: None,
			block: block(title),
			password: None,
			has_confirmation: !new_password,
		}
	}

	pub fn submit(&self) -> Option<String> {
		if !self.has_confirmation {
			return None;
		}
		self.password.clone()
	}

	pub fn is_new(&self) -> bool {
		self.new_password
	}

	pub fn set_error(&mut self, title: String, message: String) {
		self.popup = Some(Popup::new(title, Paragraph::new(Text::from(message))));
	}
}

impl Form for PasswordPrompt {
	fn component_count(&self) -> usize {
		2
	}

	fn component(&self, index: usize) -> Option<&dyn Component> {
		match index {
			0 => Some(&self.input),
			1 => Some(&self.hidden),
			_ => None,
		}
	}

	fn component_mut(&mut self, index: usize) -> Option<&mut dyn Component> {
		match index {
			0 => Some(&mut self.input),
			1 => Some(&mut self.hidden),
			_ => None,
		}
	}

	fn selected(&self) -> Option<usize> {
		Some(self.selected)
	}

	fn set_selected(&mut self, selected: Option<usize>) {
		self.selected = selected.unwrap();
	}

	fn get_style(&self) -> &crate::widgets::form::FormStyle {
		&self.style
	}

	fn scroll(&self) -> u16 {
		0
	}

	fn set_scroll(&self, _scroll: u16) {}

	fn input_form(&mut self, key: &KeyEvent) -> Option<crate::widgets::form::FormSignal> {
		if let Some(popup) = &mut self.popup {
			if popup.input(key) {
				self.popup = None;
			}
			return None;
		}
		if FormExt::input(self, key) {
			if self.selected == 1 {
				self.input.inner.formatter_mut(|fmt| {
					fmt.hidden = self.hidden.value();
				})
			}
			return None;
		}

		match key.code {
			KeyCode::Enter => {
				if self.input.inner.get_input().is_empty() {
					self.popup = Some(Popup::new(
						"Invalid Password".into(),
						Paragraph::new(Text::from("Password is empty!")),
					));
					return None;
				}

				if self.new_password && self.password.is_none() {
					self.password = Some(self.input.inner.submit());
					self.block = block(format!("Confirm password for '{}'", self.db_name));
					self.input.inner.set_input(String::default());
				} else if self.new_password {
					let confirm = self.input.inner.submit();
					if Some(confirm) != self.password {
						self.popup = Some(Popup::new(
							"Invalid Passwords".into(),
							Paragraph::new(Text::from("Passwords do not match!")),
						));
						self.password = None;
						self.block = block(format!("Password for '{}'", self.db_name));
						self.input.inner.set_input(String::default());
					} else {
						self.has_confirmation = true;
						return Some(FormSignal::Return);
					}
				} else {
					self.password = Some(self.input.inner.submit());
					return Some(FormSignal::Return);
				}
			}
			KeyCode::Esc => {
				return Some(FormSignal::Exit);
			}
			_ => {}
		}
		None
	}

	fn render_form(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let vertical = Layout::vertical([Constraint::Length(self.height())]).flex(Flex::Center);
		let horizontal = Layout::horizontal([Constraint::Percentage(50)]).flex(Flex::Center);

		let area = ctx.area;
		let [area] = area.layout(&vertical);
		let [area] = area.layout(&horizontal);

		let inner = Rect {
			x: area.x + 1,
			y: area.y + 1,
			width: area.width.saturating_sub(2),
			height: area.height.saturating_sub(2),
		};

		frame.render_widget(Clear, area);
		frame.render_widget(&self.block, area);

		ctx.area = inner;
		self.render_body(frame, ctx);

		if let Some(popup) = &self.popup {
			ctx.area = frame.area();
			popup.render(frame, ctx);
		}
	}
}
