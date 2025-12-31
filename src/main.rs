use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;

use chrono::Utc;
use clipboard_rs::ClipboardContext;
use color_eyre::Result;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::{self};
use ratatui::DefaultTerminal;
use ratatui::Frame;

use crate::data::database::CipherData;
use crate::data::database::Data;
use crate::data::database::Database;
use crate::data::database::KdfData;
use crate::data::entry::EntryTag;
use crate::data::field::Field;
use crate::data::field::FieldValue;
use crate::ui::explorer::Explorer;
use crate::ui::password;
use crate::ui::password::PasswordPrompt;
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
	password: Option<PasswordPrompt>,
}

impl App {
	pub fn new(db_name: String) -> Self {
		let ents = vec![
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
			password: Some(PasswordPrompt::new(db_name, true)),
		}
	}

	fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
		loop {
			terminal.draw(|frame| self.draw(frame))?;

			if let Event::Key(key) = event::read()? {
				if let Some(password) = &mut self.password {
					if password.input(&key) {
						continue;
					}
					let pwd = password.submit();
					if pwd.is_none() {
						return Ok(());
					}
					panic!("Got password: {:#?}", password.submit());
				}
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
		if let Some(password) = &self.password {
			ctx.selected = true;
			password.render(frame, &mut ctx);
		} else {
			self.explorer.render(frame, &mut ctx);
		}

		if let Some((_, cursor)) = ctx.cursor {
			frame.set_cursor_position(cursor);
		}
	}
}

fn run(data: &mut Data, ask_password: bool) -> Option<String> {
	None
}

fn main() -> Result<()> {
	/*
	let args: Vec<String> = env::args().collect();
	let path = PathBuf::from(&args[0]);

	if !path.exists() {
		let mut data = Data::default();
		let password = run(&mut data, true).unwrap();

		let mut salt = [0u8; 16];
		rand::fill(&mut salt);
		let db = Database {
			version: data::database::Version::V1,
			cipher: CipherData::XChaCha20Poly1305V1 {},
			kdf: KdfData::Argon2Id {
				salt,
				memory: 65536,
				iterations: 2,
				key_len: 64,
				parallelism: 4,
			},
			blob: Vec::default(),
		};
		// Create DB
	}
	*/
	//let db = Database {
	//	version: Default::default(),
	//	cipher: CipherData::XChaCha20Poly1305 { nonce: [0; 24] },
	//	kdf: KdfData::Argon2Id {
	//		salt: [0; 16],
	//		memory: 65536,
	//		iterations: 3,
	//		paralellism: true,
	//	},
	//	blob: vec![5, 7, 6],
	//};
	//println!("{}", serde_json::to_string_pretty(&db).unwrap());
	//Ok(())
	let terminal = ratatui::init();
	let app_result = App::new("Database".into()).run(terminal);
	ratatui::restore();
	app_result
}
