use std::sync::LazyLock;

use chrono::DateTime;
use chrono::Utc;
use clipboard_rs::ClipboardContext;
use color_eyre::Result;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::{self};
use ratatui::DefaultTerminal;
use ratatui::Frame;

use crate::data::entry::EntryTag;
use crate::data::field::Field;
use crate::data::field::FieldValue;
use crate::ui::explorer::Explorer;
use crate::widgets::widget::Component;
use crate::widgets::widget::ComponentRenderCtx;

pub mod data;
pub mod style;
pub mod ui;
pub mod widgets;

pub static CLIPBOARD_CTX: LazyLock<ClipboardContext> =
	LazyLock::new(|| ClipboardContext::new().unwrap());

struct App {
	explorer: Explorer,
}

impl App {
	pub fn new() -> Self {
		let mut ents = vec![
			data::entry::Entry {
				name: "test".into(),
				fields: vec![
					Field {
						name: "Username".into(),
						value: FieldValue::Text("ef3d0c3e".into()),
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
				tags: vec![EntryTag {
					name: "tag1".into(),
					icon: None,
					color: None,
				}],
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
			},
			data::entry::Entry {
				name: "foo/bar/baz".into(),
				fields: vec![],
				tags: vec![EntryTag {
					name: "tag2".into(),
					icon: Some(" ".into()),
					color: None,
				}],
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
			},
			data::entry::Entry {
				name: "ent_0".into(),
				fields: vec![],
				tags: vec![],
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
			},
			data::entry::Entry {
				name: "ent_1".into(),
				fields: vec![],
				tags: vec![],
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
			},
			data::entry::Entry {
				name: "ent_2".into(),
				fields: vec![],
				tags: vec![],
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
			},
			data::entry::Entry {
				name: "ent_3".into(),
				fields: vec![],
				tags: vec![],
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
			},
			data::entry::Entry {
				name: "ent_4".into(),
				fields: vec![],
				tags: vec![],
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
			},
			data::entry::Entry {
				name: "ent_5".into(),
				fields: vec![],
				tags: vec![],
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
			},
			data::entry::Entry {
				name: "ent_6".into(),
				fields: vec![],
				tags: vec![],
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
			},
			data::entry::Entry {
				name: "bar".into(),
				fields: vec![],
				tags: vec![EntryTag {
					name: "tag2".into(),
					icon: Some(" ".into()),
					color: None,
				}],
				created_at: Utc::now(),
				modified_at: Utc::now(),
				accessed_at: Utc::now(),
			},
		];
		Self {
			explorer: Explorer::new(ents),
		}
	}

	fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
		loop {
			terminal.draw(|frame| self.draw(frame))?;

			if let Event::Key(key) = event::read()? {
				if self.explorer.input(&key) {
					continue;
				}

				match key.code {
					KeyCode::Char('q') => return Ok(()),
					_ => {}
				}
			}
		}
	}

	fn draw(&self, frame: &mut Frame) {
		let mut overlays = vec![];
		let mut ctx = ComponentRenderCtx {
			area: frame.area(),
			selected: false,
			queue: &mut overlays,
			depth: 0,
			cursor: None,
		};
		self.explorer.render(frame, &mut ctx);

		if let Some((_, cursor)) = ctx.cursor {
			frame.set_cursor_position(cursor);
		}
	}
}

fn main() -> Result<()> {
	let terminal = ratatui::init();
	let app_result = App::new().run(terminal);
	ratatui::restore();
	app_result
}
