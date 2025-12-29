use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use crate::data::field::Field;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryTag {
	pub name: String,
	pub icon: Option<String>,
	pub color: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
	pub name: String,
	pub fields: Vec<Field>,
	pub tags: Vec<EntryTag>,

	pub created_at: DateTime<Utc>,
	pub modified_at: DateTime<Utc>,
	pub accessed_at: DateTime<Utc>,
}
