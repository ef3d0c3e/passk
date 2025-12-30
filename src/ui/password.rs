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

use crate::widgets::confirm::Confirm;
use crate::widgets::label::LabelDisplay;
use crate::widgets::label::LabelStyle;
use crate::widgets::label::Labeled;
use crate::widgets::popup::Popup;
use crate::widgets::text_input::TextInput;
use crate::widgets::text_input::TextInputStyle;
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
static PASSWORD_INPUT_STYLE: LazyLock<TextInputStyle> = LazyLock::new(|| TextInputStyle {
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

pub struct PasswordPrompt {
	db_name: String,
	new_password: bool,
	input: Labeled<'static, TextInput<'static>>,
	popup: Option<Popup<'static>>,
	block: Block<'static>,
	password: Option<String>,
}

impl PasswordPrompt {
	pub fn new(db_name: String, new_password: bool) -> Self {
		let title = format!("Password for '{}'", db_name);
		Self {
			db_name,
			new_password,
			input: Labeled::new(
				"Password".into(),
				TextInput::new().style(&PASSWORD_INPUT_STYLE),
			)
			.style(&PASSWORD_LABEL_STYLE),
			popup: None,
			block: Block::bordered()
				.title(title)
				.title_alignment(HorizontalAlignment::Center)
				.border_type(BorderType::QuadrantOutside),
			password: None,
		}
	}

	pub fn submit(&self) -> Option<String> {
		self.password.clone()
	}
}

impl Component for PasswordPrompt {
	fn input(&mut self, key: &KeyEvent) -> bool {
		if let Some(popup) = &mut self.popup {
			if !popup.input(key) {
				self.popup = None;
			}
			return true;
		}
		if self.input.input(key) {
			return true;
		}
		match key.code {
			KeyCode::Enter => {
				if self.new_password && self.password.is_none() {
					self.password = Some(self.input.inner.submit());
					self.block = self
						.block
						.clone()
						.title(format!("Confirm password for '{}'", self.db_name));
					self.input.inner.set_input(String::default());
				} else if self.new_password {
					let confirm = self.input.inner.submit();
					if Some(confirm) != self.password {
						self.popup = Some(Popup::new(
							"Invalid Passwords".into(),
							Paragraph::new(Text::from("Passwords do not match!")),
						));
						self.password = None;
						self.block = self
							.block
							.clone()
							.title(format!("Password for '{}'", self.db_name));
					} else {
						return false;
					}
				} else {
					self.password = Some(self.input.inner.submit());
					return false;
				}
			}
			KeyCode::Esc => {
				return false;
			}
			_ => {}
		}
		true
	}

	fn render(&self, frame: &mut Frame, ctx: &mut ComponentRenderCtx) {
		let vertical = Layout::vertical([Constraint::Length(2 + self.input.height())]).flex(Flex::Center);
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
		self.input.render(frame, ctx);
	}

	fn height(&self) -> u16 {
		self.input.height() + 2
	}
}
