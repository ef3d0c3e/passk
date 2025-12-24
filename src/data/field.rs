use std::borrow::Cow;

use chrono::DateTime;
use chrono::Utc;
use clipboard_rs::Clipboard;
use clipboard_rs::ClipboardContext;
use clipboard_rs::ClipboardContextX11Options;
use serde::Deserialize;
use serde::Serialize;

use crate::CLIPBOARD_CTX;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TwoFACode {
	pub value: String,
	pub expired: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum FieldValue {
	Text(String),
	Url(String),
	Phone(String),
	Email(String),
	/// TOTP RFC6238
	TOTPRFC6238(String),
	/// TOTP (Steam)
	TOTPSteam(String),
	/// 2FA Recovery code
	TwoFactorRecovery(Vec<TwoFACode>),
	/// Binary data
	Binary {
		mimetype: String,
		base64: String,
	},
}

impl Default for FieldValue {
	fn default() -> Self {
		Self::Text(String::default())
	}
}

impl FieldValue {
	pub fn copy_to_clipboard(&self) {
		let content = match self {
			FieldValue::Text(text)
			| FieldValue::Url(text)
			| FieldValue::Phone(text)
			| FieldValue::Email(text) => text.clone(),
			FieldValue::TOTPRFC6238(_) => todo!(),
			FieldValue::TOTPSteam(_) => todo!(),
			FieldValue::TwoFactorRecovery(two_facodes) => todo!(),
			FieldValue::Binary { mimetype, base64 } => todo!(),
		};
		CLIPBOARD_CTX.set_text(content)
			.unwrap();
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
	/// Field name
	pub name: String,
	/// Field value
	pub value: FieldValue,
	/// Hide from preview
	pub hidden: bool,

	pub date_added: DateTime<Utc>,
	pub date_modified: DateTime<Utc>,
	pub date_accessed: DateTime<Utc>,
}

impl Default for Field {
	fn default() -> Self {
		let now = Utc::now();
		Self {
			name: Default::default(),
			value: Default::default(),
			hidden: Default::default(),
			date_added: now.clone(),
			date_modified: now.clone(),
			date_accessed: now,
		}
	}
}
