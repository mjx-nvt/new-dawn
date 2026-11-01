//! The record engine's metadata (D-060): objects, fields and field types — Twenty's data model
//! in SQLite. Every object is a real table; a composite field (a full name, an address…) is a
//! few columns side by side, named `<column>_<suffix>`; a many-to-one relation is `<column>_id`.

use serde::{Deserialize, Serialize};
use specta::Type;

/// A record's JSON as the engine reads and writes it (its shape comes from the metadata, so the
/// UI sees `unknown` and narrows by field type).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(transparent)]
#[specta(transparent)]
pub struct Json(#[specta(type = specta_typescript::Unknown)] pub serde_json::Value);

impl Default for Json {
    fn default() -> Self {
        Self(serde_json::Value::Null)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FieldType {
    Text,
    LongText,
    RichText,
    Number,
    Boolean,
    Date,
    DateTime,
    Select,
    MultiSelect,
    Currency,
    Links,
    Domain,
    Emails,
    Phones,
    Address,
    Rating,
    Relation,
    Array,
    Json,
    FullName,
    Actor,
    Uuid,
    Position,
}

impl FieldType {
    pub const ALL: [FieldType; 23] = [
        Self::Text,
        Self::LongText,
        Self::RichText,
        Self::Number,
        Self::Boolean,
        Self::Date,
        Self::DateTime,
        Self::Select,
        Self::MultiSelect,
        Self::Currency,
        Self::Links,
        Self::Domain,
        Self::Emails,
        Self::Phones,
        Self::Address,
        Self::Rating,
        Self::Relation,
        Self::Array,
        Self::Json,
        Self::FullName,
        Self::Actor,
        Self::Uuid,
        Self::Position,
    ];

    /// The name stored in `field_meta.type` (Twenty's spelling).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::LongText => "LONG_TEXT",
            Self::RichText => "RICH_TEXT",
            Self::Number => "NUMBER",
            Self::Boolean => "BOOLEAN",
            Self::Date => "DATE",
            Self::DateTime => "DATE_TIME",
            Self::Select => "SELECT",
            Self::MultiSelect => "MULTI_SELECT",
            Self::Currency => "CURRENCY",
            Self::Links => "LINKS",
            Self::Domain => "DOMAIN",
            Self::Emails => "EMAILS",
            Self::Phones => "PHONES",
            Self::Address => "ADDRESS",
            Self::Rating => "RATING",
            Self::Relation => "RELATION",
            Self::Array => "ARRAY",
            Self::Json => "JSON",
            Self::FullName => "FULL_NAME",
            Self::Actor => "ACTOR",
            Self::Uuid => "UUID",
            Self::Position => "POSITION",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.as_str() == name)
    }

    /// Holds free text the search index should see.
    pub fn is_searchable(self) -> bool {
        matches!(
            self,
            Self::Text
                | Self::LongText
                | Self::RichText
                | Self::Emails
                | Self::Phones
                | Self::Links
                | Self::Domain
                | Self::Address
                | Self::FullName
                | Self::Array
                | Self::Select
                | Self::MultiSelect
        )
    }
}

/// How a column is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKind {
    Text,
    Real,
    Int,
}

/// One column of a composite field: its key in the record's JSON (Twenty's sub-field name), its
/// column suffix, how it is stored, and whether the text is JSON.
#[derive(Debug, Clone, Copy)]
pub struct Sub {
    pub key: &'static str,
    pub suffix: &'static str,
    pub kind: ColumnKind,
    pub json: bool,
}

const fn sub(key: &'static str, suffix: &'static str, kind: ColumnKind, json: bool) -> Sub {
    Sub { key, suffix, kind, json }
}

const FULL_NAME: [Sub; 2] = [sub("firstName", "first", ColumnKind::Text, false), sub("lastName", "last", ColumnKind::Text, false)];
const EMAILS: [Sub; 2] = [sub("primaryEmail", "primary", ColumnKind::Text, false), sub("additionalEmails", "additional", ColumnKind::Text, true)];
const PHONES: [Sub; 4] = [
    sub("primaryPhoneNumber", "number", ColumnKind::Text, false),
    sub("primaryPhoneCountryCode", "country", ColumnKind::Text, false),
    sub("primaryPhoneCallingCode", "calling", ColumnKind::Text, false),
    sub("additionalPhones", "additional", ColumnKind::Text, true),
];
const LINKS: [Sub; 3] = [
    sub("primaryLinkLabel", "label", ColumnKind::Text, false),
    sub("primaryLinkUrl", "url", ColumnKind::Text, false),
    sub("secondaryLinks", "secondary", ColumnKind::Text, true),
];
const ADDRESS: [Sub; 8] = [
    sub("addressStreet1", "street1", ColumnKind::Text, false),
    sub("addressStreet2", "street2", ColumnKind::Text, false),
    sub("addressCity", "city", ColumnKind::Text, false),
    sub("addressState", "state", ColumnKind::Text, false),
    sub("addressCountry", "country", ColumnKind::Text, false),
    sub("addressPostcode", "postcode", ColumnKind::Text, false),
    sub("addressLat", "lat", ColumnKind::Real, false),
    sub("addressLng", "lng", ColumnKind::Real, false),
];
const CURRENCY: [Sub; 2] = [sub("amountMicros", "micros", ColumnKind::Int, false), sub("currencyCode", "code", ColumnKind::Text, false)];
const ACTOR: [Sub; 4] = [
    sub("source", "source", ColumnKind::Text, false),
    sub("name", "name", ColumnKind::Text, false),
    sub("workspaceMemberId", "member_id", ColumnKind::Text, false),
    sub("context", "context", ColumnKind::Text, true),
];

/// The sub-fields of a composite type (empty for a scalar).
pub fn subs(t: FieldType) -> &'static [Sub] {
    match t {
        FieldType::FullName => &FULL_NAME,
        FieldType::Emails => &EMAILS,
        FieldType::Phones => &PHONES,
        FieldType::Links | FieldType::Domain => &LINKS,
        FieldType::Address => &ADDRESS,
        FieldType::Currency => &CURRENCY,
        FieldType::Actor => &ACTOR,
        _ => &[],
    }
}

/// A scalar type's column: how it is stored, and whether the text is JSON.
pub fn scalar(t: FieldType) -> (ColumnKind, bool) {
    match t {
        FieldType::Number | FieldType::Position => (ColumnKind::Real, false),
        FieldType::Rating | FieldType::Boolean => (ColumnKind::Int, false),
        FieldType::MultiSelect | FieldType::Array | FieldType::Json => (ColumnKind::Text, true),
        _ => (ColumnKind::Text, false),
    }
}

/// One stored column of a field.
#[derive(Debug, Clone)]
pub struct StoredColumn {
    /// The record's JSON key: the field's name, or a composite's sub-field key.
    pub key: Option<&'static str>,
    pub name: String,
    pub kind: ColumnKind,
    pub json: bool,
}

/// The columns a field occupies in its table (none for a one-to-many relation).
pub fn columns(field: &FieldMeta) -> Vec<StoredColumn> {
    let composite = subs(field.r#type);
    if !composite.is_empty() {
        return composite
            .iter()
            .map(|s| StoredColumn { key: Some(s.key), name: format!("{}_{}", field.column, s.suffix), kind: s.kind, json: s.json })
            .collect();
    }
    if field.r#type == FieldType::Relation {
        return match field.options.relation.as_ref().map(|r| r.kind) {
            Some(RelationKind::ManyToOne) => vec![StoredColumn { key: None, name: format!("{}_id", field.column), kind: ColumnKind::Text, json: false }],
            _ => Vec::new(),
        };
    }
    let (kind, json) = scalar(field.r#type);
    vec![StoredColumn { key: None, name: field.column.clone(), kind, json }]
}

/// An object: a table and its fields (`object_meta`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct ObjectMeta {
    pub id: String,
    pub name_singular: String,
    pub name_plural: String,
    pub label_singular: String,
    pub label_plural: String,
    pub icon: String,
    pub description: String,
    pub table_name: String,
    pub is_custom: bool,
    pub is_active: bool,
    /// Hidden from the sidebar and the data model's list (targets, favorites, the timeline…).
    pub is_system: bool,
    pub label_identifier_field_id: Option<String>,
    pub image_identifier_field_id: Option<String>,
    pub position: f64,
}

/// A field of an object (`field_meta`), with its options parsed.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FieldMeta {
    pub id: String,
    pub object_id: String,
    pub name: String,
    pub label: String,
    pub r#type: FieldType,
    /// The base column name; composites and relations derive theirs from it ('' = no column).
    pub column: String,
    pub options: FieldOptions,
    pub default_value: Option<Json>,
    pub is_custom: bool,
    pub is_active: bool,
    pub is_nullable: bool,
    pub is_unique: bool,
    pub is_system: bool,
    pub position: f64,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq)]
pub struct FieldOptions {
    /// SELECT / MULTI_SELECT options, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub select: Vec<SelectOption>,
    /// NUMBER: decimals shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    /// CURRENCY: the default code; PHONES: the default country (ISO 3166-1 alpha-2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// TEXT shown on several rows (Twenty's "Long text").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation: Option<RelationOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    /// Twenty's colour names: green · turquoise · sky · blue · purple · pink · red · orange ·
    /// yellow · gray.
    pub color: String,
    pub position: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationKind {
    ManyToOne,
    OneToMany,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct RelationOptions {
    /// The other object's `name_singular`.
    pub target: String,
    pub kind: RelationKind,
    /// The field on the other object that shows this relation from its side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inverse: Option<String>,
    /// A one-to-many that passes through a junction object (Twenty's many-to-many).
    #[serde(default)]
    pub junction: bool,
}

/// A table or column name the engine will put in SQL: a lower-case letter, then lower-case
/// letters, digits and underscores, at most 63 long.
pub fn valid_ident(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && name.len() <= 63
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The row of `field_meta` as stored.
#[derive(Debug, sqlx::FromRow)]
pub struct FieldRow {
    pub id: String,
    pub object_id: String,
    pub name: String,
    pub label: String,
    #[sqlx(rename = "type")]
    pub type_name: String,
    #[sqlx(rename = "column_name")]
    pub column: String,
    pub options_json: String,
    pub default_json: Option<String>,
    pub is_custom: bool,
    pub is_active: bool,
    pub is_nullable: bool,
    pub is_unique: bool,
    pub is_system: bool,
    pub position: f64,
    pub description: String,
    pub icon: String,
}

impl FieldRow {
    pub fn into_meta(self) -> Option<FieldMeta> {
        let r#type = FieldType::parse(&self.type_name)?;
        if !self.column.is_empty() && !valid_ident(&self.column) {
            return None;
        }
        Some(FieldMeta {
            id: self.id,
            object_id: self.object_id,
            name: self.name,
            label: self.label,
            r#type,
            column: self.column,
            options: serde_json::from_str(&self.options_json).unwrap_or_default(),
            default_value: self.default_json.and_then(|d| serde_json::from_str(&d).ok()).map(Json),
            is_custom: self.is_custom,
            is_active: self.is_active,
            is_nullable: self.is_nullable,
            is_unique: self.is_unique,
            is_system: self.is_system,
            position: self.position,
            description: self.description,
            icon: self.icon,
        })
    }
}

/// An object with its fields, as the query layer needs it.
#[derive(Debug, Clone, Serialize, Type)]
pub struct ObjectDef {
    pub meta: ObjectMeta,
    pub fields: Vec<FieldMeta>,
}

impl ObjectDef {
    pub fn field(&self, name: &str) -> Option<&FieldMeta> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// The field whose value names a record (Twenty's label identifier), else the first text one.
    pub fn label_field(&self) -> Option<&FieldMeta> {
        self.meta
            .label_identifier_field_id
            .as_deref()
            .and_then(|id| self.fields.iter().find(|f| f.id == id))
            .or_else(|| self.fields.iter().find(|f| matches!(f.r#type, FieldType::Text | FieldType::FullName)))
    }

    /// SQL for a record's label (used for relation pickers and the table's first column).
    pub fn label_sql(&self, alias: &str) -> String {
        match self.label_field() {
            Some(f) if f.r#type == FieldType::FullName => format!(
                "trim(coalesce({a}.{c}_first, '') || ' ' || coalesce({a}.{c}_last, ''))",
                a = alias,
                c = f.column
            ),
            Some(f) => format!("coalesce({}.{}, '')", alias, f.column),
            None => "''".to_owned(),
        }
    }
}
