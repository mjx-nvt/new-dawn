//! Run-time DDL (D-060, 0.10.5): Twenty's custom objects and fields as real SQLite tables and
//! columns, made from the data model settings. The metadata is the source of truth: `repair`
//! (at every open, and after any failure) creates whatever the metadata says and the schema
//! lacks, so a half-made object is finished at the next open. Deactivating keeps everything;
//! deleting drops a custom table but never a column (SQLite can't drop a referenced one).

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;
use sqlx::SqlitePool;

use super::timeline::snake;
use super::types::{columns, valid_ident, ColumnKind, FieldMeta, FieldOptions, FieldType, Json, ObjectDef, ObjectMeta, RelationKind, RelationOptions, SelectOption, StoredColumn};
use super::{object, objects};
use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
/// The system objects every record-bearing object links to, with their tables.
const TARGETS: [(&str, &str, &str); 5] = [
    ("noteTarget", "note_target", "noteTargets"),
    ("taskTarget", "task_target", "taskTargets"),
    ("attachment", "attachment", "attachments"),
    ("favorite", "favorite", "favorites"),
    ("timelineActivity", "timeline_activity", "timelineActivities"),
];
/// Column names every object has, so no field can take them.
const RESERVED: [&str; 7] = ["id", "position", "createdBy", "deletedAt", "createdAt", "updatedAt", "searchVector"];
/// The types the picker offers (Twenty's): the rest are the engine's own.
pub const CREATABLE: [FieldType; 18] = [
    FieldType::Text,
    FieldType::LongText,
    FieldType::RichText,
    FieldType::Number,
    FieldType::Boolean,
    FieldType::Date,
    FieldType::DateTime,
    FieldType::Select,
    FieldType::MultiSelect,
    FieldType::Currency,
    FieldType::Links,
    FieldType::Domain,
    FieldType::Emails,
    FieldType::Phones,
    FieldType::Address,
    FieldType::Rating,
    FieldType::Relation,
    FieldType::Array,
];

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

/// A label as an API name: `Project Assignment` → `projectAssignment`, `Đối tác` → `doiTac`
/// (accents and đ fold away, as everywhere in the kernel).
pub fn camel(label: &str) -> String {
    let mut out = String::new();
    let mut upper_next = false;
    for c in crate::kernel::text::fold(label).chars() {
        if !c.is_alphanumeric() {
            upper_next = !out.is_empty();
            continue;
        }
        if !c.is_ascii() {
            continue; // a letter a name can't hold (a diacritic): skipped, the word goes on
        }
        if out.is_empty() {
            out.push(c.to_ascii_lowercase());
        } else if upper_next {
            out.push(c.to_ascii_uppercase());
        } else {
            out.push(c);
        }
        upper_next = false;
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, 'f');
    }
    out
}

/// A select option's value from its label: `On site` → `ON_SITE`.
pub fn screaming(label: &str) -> String {
    snake(&camel(label)).to_ascii_uppercase()
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct ObjectInput {
    pub label_singular: String,
    pub label_plural: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub description: String,
    /// Twenty's "Skip creating a Name field" (a junction object).
    #[serde(default)]
    pub skip_name: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Type)]
pub struct ObjectPatch {
    #[serde(default)]
    pub label_singular: Option<String>,
    #[serde(default)]
    pub label_plural: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// A field id; an empty string clears it.
    #[serde(default)]
    pub label_identifier_field_id: Option<String>,
    #[serde(default)]
    pub image_identifier_field_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationInputKind {
    ManyToOne,
    OneToMany,
    /// Through a junction object, made for you.
    ManyToMany,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct RelationInput {
    /// The other object, by singular name, plural name or id.
    pub target: String,
    pub kind: RelationInputKind,
    /// The field's label on the other object (Twenty's "Field name on destination object").
    #[serde(default)]
    pub inverse_label: Option<String>,
    /// A one-to-many through a junction object shows the far records (Twenty's toggle).
    #[serde(default)]
    pub junction: bool,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct FieldInput {
    pub label: String,
    pub r#type: FieldType,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub options: FieldOptions,
    #[serde(default)]
    pub default_value: Option<Json>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub relation: Option<RelationInput>,
}

#[derive(Debug, Clone, Default, Deserialize, Type)]
pub struct FieldPatch {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub options: Option<FieldOptions>,
    /// An empty string clears the default.
    #[serde(default)]
    pub default_value: Option<Json>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub unique: Option<bool>,
}

/// A field to insert: what `field_meta` holds.
#[derive(Debug, Clone, Serialize)]
struct Spec {
    name: String,
    label: String,
    r#type: FieldType,
    column: String,
    options: FieldOptions,
    default_value: Option<Value>,
    nullable: bool,
    unique: bool,
    system: bool,
    custom: bool,
    icon: String,
    description: String,
}

impl Spec {
    fn new(name: &str, label: &str, r#type: FieldType, column: &str, icon: &str) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            r#type,
            column: column.into(),
            options: FieldOptions::default(),
            default_value: None,
            nullable: true,
            unique: false,
            system: false,
            custom: false,
            icon: icon.into(),
            description: String::new(),
        }
    }
    fn system(mut self) -> Self {
        self.system = true;
        self
    }
    fn relation(mut self, target: &str, kind: RelationKind, inverse: &str, junction: bool) -> Self {
        self.options.relation = Some(RelationOptions { target: target.into(), kind, inverse: Some(inverse.into()), junction });
        self
    }
}

fn to_one(name: &str, label: &str, target: &str, inverse: &str) -> Spec {
    Spec::new(name, label, FieldType::Relation, &snake(name), "IconArrowUpRight").relation(target, RelationKind::ManyToOne, inverse, false)
}

fn to_many(name: &str, label: &str, target: &str, inverse: &str) -> Spec {
    Spec::new(name, label, FieldType::Relation, "", "IconArrowUpRight").relation(target, RelationKind::OneToMany, inverse, false)
}

async fn next_position(pool: &SqlitePool, object_id: &str) -> Result<f64> {
    let (p,): (f64,) = sqlx::query_as("SELECT coalesce(max(position), 0.0) + 1.0 FROM field_meta WHERE object_id = ?").bind(object_id).fetch_one(pool).await?;
    Ok(p)
}

async fn insert_field(pool: &SqlitePool, object_id: &str, spec: &Spec, position: f64) -> Result<String> {
    let id = uuid::Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO field_meta (id, object_id, name, label, type, column_name, options_json, default_json, is_custom, is_active, \
         is_nullable, is_unique, is_system, position, description, icon) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(object_id)
    .bind(&spec.name)
    .bind(&spec.label)
    .bind(spec.r#type.as_str())
    .bind(&spec.column)
    .bind(serde_json::to_string(&spec.options)?)
    .bind(spec.default_value.as_ref().map(Value::to_string))
    .bind(spec.custom)
    .bind(spec.nullable)
    .bind(spec.unique)
    .bind(spec.system)
    .bind(position)
    .bind(&spec.description)
    .bind(&spec.icon)
    .execute(pool)
    .await?;
    Ok(id)
}

async fn has_field(pool: &SqlitePool, object_id: &str, name: &str) -> Result<bool> {
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM field_meta WHERE object_id = ? AND name = ?").bind(object_id).bind(name).fetch_optional(pool).await?;
    Ok(row.is_some())
}

/// The columns every record table has outside its fields.
const BASE_COLUMNS: [&str; 9] = ["id", "position", "created_by_source", "created_by_name", "created_by_member_id", "created_by_context", "deleted_at", "created_at", "updated_at"];

/// A spec as the metadata will read it, for the column mapping.
fn as_meta(spec: &Spec) -> FieldMeta {
    FieldMeta {
        id: String::new(),
        object_id: String::new(),
        name: spec.name.clone(),
        label: spec.label.clone(),
        r#type: spec.r#type,
        column: spec.column.clone(),
        options: spec.options.clone(),
        default_value: spec.default_value.clone().map(Json),
        is_custom: spec.custom,
        is_active: true,
        is_nullable: spec.nullable,
        is_unique: spec.unique,
        is_system: spec.system,
        position: 0.0,
        description: spec.description.clone(),
        icon: spec.icon.clone(),
    }
}

/// A new field's columns must be names SQL takes, and must not be the table's base columns, a
/// live field's columns, or (on a standard table) any column the migrations made — only a
/// custom table reuses a column, a deleted field's.
async fn check_columns(pool: &SqlitePool, object: &ObjectDef, spec: &Spec) -> Result<()> {
    let owned: Vec<String> = object.fields.iter().flat_map(columns).map(|c| c.name).collect();
    let present = table_columns(pool, &object.meta.table_name).await?;
    for col in columns(&as_meta(spec)) {
        if !valid_ident(&col.name) {
            return Err(invalid(format!("“{}” can't be a column name", spec.label)));
        }
        if BASE_COLUMNS.contains(&col.name.as_str()) || owned.contains(&col.name) {
            return Err(invalid(format!("“{}” would take a column another field uses", spec.label)));
        }
        if present.contains(&col.name) && !object.meta.is_custom {
            return Err(invalid(format!("“{}” would take a column this table already has", spec.label)));
        }
    }
    Ok(())
}

/// A relation to the object itself needs two different field names.
fn distinct_sides(from: &ObjectDef, to: &ObjectDef, name: &str, inverse: &str) -> Result<()> {
    if from.meta.id == to.meta.id && name == inverse {
        return Err(invalid("A relation to the same object needs a different name on each side"));
    }
    Ok(())
}

/// A new field name for a label on an object: camelCase, not reserved, not taken.
async fn field_name(pool: &SqlitePool, object: &ObjectDef, label: &str) -> Result<String> {
    let name = camel(label);
    if name.is_empty() {
        return Err(invalid("A field needs a label with letters"));
    }
    if RESERVED.contains(&name.as_str()) || !valid_ident(&snake(&name)) {
        return Err(invalid(format!("“{label}” is a reserved name")));
    }
    if has_field(pool, &object.meta.id, &name).await? {
        return Err(invalid(format!("{} already has a field named “{label}”", object.meta.label_plural)));
    }
    Ok(name)
}

// ---- schema -------------------------------------------------------------------------------

async fn table_columns(pool: &SqlitePool, table: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT name FROM pragma_table_info('{table}')"))).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|(n,)| n).collect())
}

/// The SQL type and default of one of a field's columns, as the standard tables have them. A
/// relation to a custom object carries no foreign key: SQLite can't drop a constraint, and a
/// dangling one would break every write to the table once the custom table is gone (the engine
/// checks that a relation points at a live record itself; `delete_object` clears the links).
fn column_sql(field: &FieldMeta, col: &StoredColumn, target: Option<(&str, bool)>) -> String {
    if field.r#type == FieldType::Relation {
        return match target {
            Some((table, false)) => format!("TEXT REFERENCES {table} (id) ON DELETE SET NULL"),
            _ => "TEXT".into(),
        };
    }
    match field.r#type {
        FieldType::Date | FieldType::DateTime => "TEXT".into(),
        FieldType::Json => "TEXT NOT NULL DEFAULT 'null'".into(),
        FieldType::Select if !field.is_nullable => {
            let value = field
                .default_value
                .as_ref()
                .and_then(|d| d.0.as_str().map(str::to_owned))
                .or_else(|| field.options.select.first().map(|o| o.value.clone()))
                .unwrap_or_default();
            format!("TEXT NOT NULL DEFAULT '{}'", value.replace('\'', "''"))
        }
        FieldType::Boolean => "INTEGER NOT NULL DEFAULT 0".into(),
        _ => match (col.kind, col.json) {
            (ColumnKind::Text, true) => "TEXT NOT NULL DEFAULT '[]'".into(),
            (ColumnKind::Text, false) => "TEXT NOT NULL DEFAULT ''".into(),
            (ColumnKind::Real, _) => "REAL".into(),
            (ColumnKind::Int, _) => "INTEGER".into(),
        },
    }
}

/// Adds the columns a field needs and its table lacks (a deleted field's column is reused).
async fn ensure_columns(pool: &SqlitePool, table: &str, field: &FieldMeta, target: Option<(&str, bool)>) -> Result<()> {
    let present = table_columns(pool, table).await?;
    for col in columns(field) {
        if !valid_ident(&col.name) {
            return Err(invalid(format!("“{}” is not a column name", col.name)));
        }
        if present.contains(&col.name) {
            continue;
        }
        sqlx::query(sqlx::AssertSqlSafe(format!("ALTER TABLE {table} ADD COLUMN {} {}", col.name, column_sql(field, &col, target)))).execute(pool).await?;
        if field.r#type == FieldType::Relation {
            sqlx::query(sqlx::AssertSqlSafe(format!("CREATE INDEX IF NOT EXISTS idx_{table}_{} ON {table} ({})", col.name, col.name))).execute(pool).await?;
        }
    }
    Ok(())
}

/// A custom object's table with the columns every record has.
async fn ensure_table(pool: &SqlitePool, table: &str) -> Result<()> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE TABLE IF NOT EXISTS {table} (\
         id TEXT PRIMARY KEY, \
         position REAL NOT NULL DEFAULT 0, \
         created_by_source TEXT NOT NULL DEFAULT 'MANUAL', \
         created_by_name TEXT NOT NULL DEFAULT '', \
         created_by_member_id TEXT, \
         created_by_context TEXT NOT NULL DEFAULT '{{}}', \
         deleted_at TEXT, \
         created_at TEXT NOT NULL DEFAULT ({NOW}), \
         updated_at TEXT NOT NULL DEFAULT ({NOW}))"
    )))
    .execute(pool)
    .await?;
    Ok(())
}

/// `<object>_id` on the five target tables, so notes, tasks, files, favorites and the timeline
/// can point at a custom object's records (plain columns: see `column_sql`).
async fn ensure_target_columns(pool: &SqlitePool, object: &ObjectDef) -> Result<()> {
    let column = format!("{}_id", snake(&object.meta.name_singular));
    if !valid_ident(&column) {
        return Err(invalid(format!("“{}” can't be a column name", object.meta.name_singular)));
    }
    for (_, table, _) in TARGETS {
        if table_columns(pool, table).await?.contains(&column) {
            continue;
        }
        sqlx::query(sqlx::AssertSqlSafe(format!("ALTER TABLE {table} ADD COLUMN {column} TEXT"))).execute(pool).await?;
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE INDEX IF NOT EXISTS idx_{table}_{column} ON {table} ({column})"))).execute(pool).await?;
    }
    Ok(())
}

/// The table a many-to-one relation points at, and whether it is a custom one.
async fn target_table_of(pool: &SqlitePool, field: &FieldMeta) -> Result<Option<(String, bool)>> {
    let Some(rel) = &field.options.relation else { return Ok(None) };
    if rel.kind != RelationKind::ManyToOne {
        return Ok(None);
    }
    let target = object(pool, &rel.target).await?;
    Ok(Some((target.meta.table_name, target.meta.is_custom)))
}

/// Everything the metadata says and the schema lacks — a custom object's table, its link
/// columns on the target tables, every custom field's columns. Idempotent; run at every open.
/// One object's trouble is logged and the others still get repaired (the open goes on).
pub async fn repair(pool: &SqlitePool) -> Result<()> {
    for meta in objects(pool).await? {
        if let Err(err) = repair_object(pool, &meta).await {
            tracing::error!("repair of {}: {err:#}", meta.name_singular);
        }
    }
    Ok(())
}

async fn repair_object(pool: &SqlitePool, meta: &ObjectMeta) -> Result<()> {
    let obj = object(pool, &meta.id).await?;
    if meta.is_custom {
        ensure_table(pool, &meta.table_name).await?;
        ensure_target_columns(pool, &obj).await?;
    }
    for field in obj.fields.iter().filter(|f| f.is_custom || meta.is_custom) {
        let target = target_table_of(pool, field).await?;
        ensure_columns(pool, &meta.table_name, field, target.as_ref().map(|(t, c)| (t.as_str(), *c))).await?;
    }
    Ok(())
}

// ---- objects ------------------------------------------------------------------------------

/// A custom object: its metadata (the Name field as label identifier, the common fields, the
/// links to notes, tasks, files, favorites and the timeline), its table, its link columns.
pub async fn create_object(pool: &SqlitePool, input: ObjectInput) -> Result<ObjectDef> {
    let label_singular = input.label_singular.trim();
    let label_plural = input.label_plural.trim();
    if label_singular.is_empty() || label_plural.is_empty() {
        return Err(invalid("An object needs a singular and a plural label"));
    }
    let name = camel(label_singular);
    let plural = camel(label_plural);
    if name.is_empty() || plural.is_empty() || name == plural {
        return Err(invalid("The singular and plural labels must differ and have letters"));
    }
    let table = format!("custom_{}", snake(&name));
    if !valid_ident(&table) || !valid_ident(&snake(&plural)) {
        return Err(invalid(format!("“{label_singular}” can't be a table name")));
    }
    for taken in [&name, &plural] {
        if object(pool, taken).await.is_ok() {
            return Err(invalid(format!("An object named “{taken}” exists")));
        }
    }
    // Projects' own favorite columns (D-077, D-079, D-080) are no custom object's link column.
    if ["pm_view_id", "pm_cycle_id", "pm_module_id", "pm_page_id"].contains(&format!("{}_id", snake(&name)).as_str()) {
        return Err(invalid(format!("“{label_singular}” is a name New Dawn uses")));
    }
    let id = uuid::Uuid::now_v7().to_string();
    let (position,): (f64,) = sqlx::query_as("SELECT coalesce(max(position), 0.0) + 1.0 FROM object_meta").fetch_one(pool).await?;
    sqlx::query(
        "INSERT INTO object_meta (id, name_singular, name_plural, label_singular, label_plural, icon, description, table_name, \
         is_custom, is_active, is_system, position) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, 1, 0, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&plural)
    .bind(label_singular)
    .bind(label_plural)
    .bind(input.icon.trim())
    .bind(input.description.trim())
    .bind(&table)
    .bind(position)
    .execute(pool)
    .await?;

    let mut specs = Vec::new();
    if !input.skip_name {
        let mut spec = Spec::new("name", "Name", FieldType::Text, "name", "IconAbc");
        spec.nullable = false;
        spec.custom = true;
        specs.push(spec);
    }
    specs.push(Spec::new("position", "Position", FieldType::Position, "position", "IconHierarchy2").system());
    specs.push(Spec::new("createdBy", "Created by", FieldType::Actor, "created_by", "IconCreativeCommonsSa").system());
    for (target, _, inverse) in TARGETS {
        specs.push(to_many(inverse, inverse, target, &name).system());
    }
    for (i, spec) in specs.iter().enumerate() {
        let field_id = insert_field(pool, &id, spec, i as f64).await?;
        if spec.name == "name" {
            sqlx::query("UPDATE object_meta SET label_identifier_field_id = ? WHERE id = ?").bind(&field_id).bind(&id).execute(pool).await?;
        }
    }
    // The far side: the system objects point back at the new object's records.
    for (target, _, inverse) in TARGETS {
        let system_object = object(pool, target).await?;
        if !has_field(pool, &system_object.meta.id, &name).await? {
            let position = next_position(pool, &system_object.meta.id).await?;
            insert_field(pool, &system_object.meta.id, &to_one(&name, label_singular, &name, inverse).system(), position).await?;
        }
    }
    let made = object(pool, &id).await?;
    ensure_table(pool, &table).await?;
    ensure_target_columns(pool, &made).await?;
    for field in &made.fields {
        ensure_columns(pool, &table, field, None).await?;
    }
    super::views::ensure_index(pool, &made).await?;
    Ok(made)
}

pub async fn update_object(pool: &SqlitePool, id: &str, patch: ObjectPatch) -> Result<ObjectDef> {
    let current = object(pool, id).await?;
    let text = |value: Option<String>, was: &str| value.map(|v| v.trim().to_owned()).unwrap_or_else(|| was.to_owned());
    let label_singular = text(patch.label_singular, &current.meta.label_singular);
    let label_plural = text(patch.label_plural, &current.meta.label_plural);
    if label_singular.is_empty() || label_plural.is_empty() {
        return Err(invalid("An object needs a singular and a plural label"));
    }
    let identifier = |value: Option<String>, was: &Option<String>, kinds: &[FieldType]| -> Result<Option<String>> {
        let Some(v) = value else { return Ok(was.clone()) };
        if v.is_empty() {
            return Ok(None);
        }
        match current.fields.iter().find(|f| f.id == v) {
            Some(f) if kinds.contains(&f.r#type) => Ok(Some(v)),
            _ => Err(invalid("An identifier is one of the object's text fields")),
        }
    };
    let label_id = identifier(patch.label_identifier_field_id, &current.meta.label_identifier_field_id, &[FieldType::Text, FieldType::FullName])?;
    let image_id = identifier(patch.image_identifier_field_id, &current.meta.image_identifier_field_id, &[FieldType::Text, FieldType::Links])?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE object_meta SET label_singular = ?, label_plural = ?, icon = ?, description = ?, label_identifier_field_id = ?, \
         image_identifier_field_id = ?, updated_at = {NOW} WHERE id = ?"
    )))
    .bind(&label_singular)
    .bind(&label_plural)
    .bind(text(patch.icon, &current.meta.icon))
    .bind(text(patch.description, &current.meta.description))
    .bind(label_id)
    .bind(image_id)
    .bind(&current.meta.id)
    .execute(pool)
    .await?;
    object(pool, id).await
}

/// Off hides the object everywhere and takes it out of search; its rows stay. On brings it back.
pub async fn set_object_active(pool: &SqlitePool, id: &str, active: bool) -> Result<ObjectDef> {
    let current = object(pool, id).await?;
    if current.meta.is_system {
        return Err(invalid("A system object stays as it is"));
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE object_meta SET is_active = ?, updated_at = {NOW} WHERE id = ?")))
        .bind(active)
        .bind(&current.meta.id)
        .execute(pool)
        .await?;
    let updated = object(pool, id).await?;
    if active {
        super::search::sync(pool, &updated).await?;
    } else {
        sqlx::query("DELETE FROM search_index WHERE object = ?").bind(&updated.meta.name_singular).execute(pool).await?;
    }
    Ok(updated)
}

/// Every relation field, on any object, that points at an object.
async fn fields_targeting(pool: &SqlitePool, name: &str) -> Result<Vec<(String, FieldMeta)>> {
    let mut out = Vec::new();
    for meta in objects(pool).await? {
        if meta.name_singular == name {
            continue;
        }
        for field in object(pool, &meta.id).await?.fields {
            if field.options.relation.as_ref().is_some_and(|r| r.target == name) {
                out.push((meta.id.clone(), field));
            }
        }
    }
    Ok(out)
}

/// A custom object that is off: its table goes, with its fields, the fields that pointed at it
/// (their columns emptied), the link rows of notes, tasks, files, favorites and the timeline,
/// its views, its page layout and its search rows. The `<object>_id` columns elsewhere stay.
pub async fn delete_object(pool: &SqlitePool, id: &str) -> Result<()> {
    let current = object(pool, id).await?;
    if !current.meta.is_custom || !current.meta.table_name.starts_with("custom_") {
        return Err(invalid("A standard object can be deactivated, not deleted"));
    }
    if current.meta.is_active {
        return Err(invalid("Deactivate the object first"));
    }
    let column = format!("{}_id", snake(&current.meta.name_singular));
    if !valid_ident(&column) {
        return Err(invalid("The object's link column isn't a column name"));
    }
    for (_, table, _) in TARGETS {
        if table_columns(pool, table).await?.contains(&column) {
            sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {table} WHERE {column} IS NOT NULL"))).execute(pool).await?;
        }
    }
    for (object_id, field) in fields_targeting(pool, &current.meta.name_singular).await? {
        if field.options.relation.as_ref().is_some_and(|r| r.kind == RelationKind::ManyToOne) {
            let table = object(pool, &object_id).await?.meta.table_name;
            if let Some(col) = columns(&field).first().filter(|c| valid_ident(&c.name)) {
                sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {table} SET {c} = NULL WHERE {c} IS NOT NULL", c = col.name))).execute(pool).await?;
            }
        }
        sqlx::query("DELETE FROM field_meta WHERE id = ?").bind(&field.id).execute(pool).await?;
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP TABLE IF EXISTS {}", current.meta.table_name))).execute(pool).await?;
    sqlx::query("DELETE FROM search_index WHERE object = ?").bind(&current.meta.name_singular).execute(pool).await?;
    super::navigation::forget_visits(pool, &current.meta.name_singular, None).await?;
    sqlx::query("DELETE FROM object_meta WHERE id = ?").bind(&current.meta.id).execute(pool).await?;
    Ok(())
}

// ---- fields -------------------------------------------------------------------------------

/// A select's default must be one of its options — given by value or by label; stored by value.
fn resolve_default(r#type: FieldType, options: &FieldOptions, default: Option<Value>) -> Result<Option<Value>> {
    let Some(default) = default.filter(|d| !d.is_null() && d.as_str() != Some("")) else { return Ok(None) };
    if r#type != FieldType::Select {
        return Ok(Some(default));
    }
    let wanted = default.as_str().unwrap_or_default();
    options
        .select
        .iter()
        .find(|o| o.value == wanted || o.label == wanted)
        .map(|o| Some(Value::String(o.value.clone())))
        .ok_or_else(|| invalid("The default is one of the options"))
}

/// Select options as Twenty keeps them: a value from the label when missing, positions in order.
fn clean_options(options: &FieldOptions) -> FieldOptions {
    let mut out = options.clone();
    out.select = options
        .select
        .iter()
        .filter(|o| !o.label.trim().is_empty())
        .enumerate()
        .map(|(i, o)| SelectOption {
            value: if o.value.trim().is_empty() { screaming(&o.label) } else { o.value.trim().to_owned() },
            label: o.label.trim().to_owned(),
            color: if o.color.is_empty() { "gray".into() } else { o.color.clone() },
            position: i as f64,
        })
        .collect();
    out
}

/// A field from the picker. A relation also makes its far side: the inverse field on the
/// target (a many-to-one's one-to-many, a one-to-many's many-to-one with the column on the
/// target's table), or, for a many-to-many, a junction object with a many-to-one to each side.
pub async fn create_field(pool: &SqlitePool, object_def: &ObjectDef, input: FieldInput) -> Result<FieldMeta> {
    if object_def.meta.is_system {
        return Err(invalid("A system object keeps its fields"));
    }
    if !CREATABLE.contains(&input.r#type) {
        return Err(invalid(format!("{} fields aren't made by hand", input.r#type.as_str())));
    }
    let label = input.label.trim().to_owned();
    let name = field_name(pool, object_def, &label).await?;
    if input.r#type == FieldType::Relation {
        return create_relation(pool, object_def, &name, &label, input).await;
    }
    let mut spec = Spec::new(&name, &label, input.r#type, &snake(&name), input.icon.trim());
    spec.options = clean_options(&input.options);
    spec.default_value = resolve_default(spec.r#type, &spec.options, input.default_value.map(|d| d.0))?;
    spec.nullable = !input.required;
    spec.unique = input.unique;
    spec.custom = true;
    spec.description = input.description.trim().to_owned();
    if matches!(spec.r#type, FieldType::Select | FieldType::MultiSelect) && spec.options.select.is_empty() {
        return Err(invalid("A select needs at least one option"));
    }
    check_columns(pool, object_def, &spec).await?;
    let position = next_position(pool, &object_def.meta.id).await?;
    insert_field(pool, &object_def.meta.id, &spec, position).await?;
    let made = object(pool, &object_def.meta.id).await?;
    let field = made.field(&name).cloned().ok_or_else(|| anyhow!("the field just made is missing"))?;
    ensure_columns(pool, &made.meta.table_name, &field, None).await?;
    super::views::adopt_field(pool, &made, &field).await?;
    Ok(field)
}

async fn create_relation(pool: &SqlitePool, from: &ObjectDef, name: &str, label: &str, input: FieldInput) -> Result<FieldMeta> {
    let relation = input.relation.ok_or_else(|| invalid("A relation names its target object"))?;
    let to = object(pool, &relation.target).await?;
    if to.meta.is_system {
        return Err(invalid("A relation points at a record object"));
    }
    let inverse_label = relation.inverse_label.as_deref().map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned);
    match relation.kind {
        RelationInputKind::ManyToOne => {
            let inverse_label = inverse_label.unwrap_or_else(|| from.meta.label_plural.clone());
            let inverse = field_name(pool, &to, &inverse_label).await?;
            distinct_sides(from, &to, name, &inverse)?;
            let mut here = Spec::new(name, label, FieldType::Relation, &snake(name), input.icon.trim()).relation(&to.meta.name_singular, RelationKind::ManyToOne, &inverse, false);
            here.custom = true;
            here.description = input.description.trim().to_owned();
            check_columns(pool, from, &here).await?;
            let mut there = to_many(&inverse, &inverse_label, &from.meta.name_singular, name);
            there.custom = true;
            if let Some(r) = there.options.relation.as_mut() {
                r.junction = relation.junction;
            }
            insert_field(pool, &from.meta.id, &here, next_position(pool, &from.meta.id).await?).await?;
            insert_field(pool, &to.meta.id, &there, next_position(pool, &to.meta.id).await?).await?;
            let made = object(pool, &from.meta.id).await?;
            let field = made.field(name).cloned().ok_or_else(|| anyhow!("the field just made is missing"))?;
            ensure_columns(pool, &made.meta.table_name, &field, Some((&to.meta.table_name, to.meta.is_custom))).await?;
            super::views::adopt_field(pool, &made, &field).await?;
            Ok(field)
        }
        RelationInputKind::OneToMany => {
            let inverse_label = inverse_label.unwrap_or_else(|| from.meta.label_singular.clone());
            let inverse = field_name(pool, &to, &inverse_label).await?;
            distinct_sides(from, &to, name, &inverse)?;
            let mut here = to_many(name, label, &to.meta.name_singular, &inverse);
            here.custom = true;
            here.icon = input.icon.trim().to_owned();
            here.description = input.description.trim().to_owned();
            if let Some(r) = here.options.relation.as_mut() {
                r.junction = relation.junction;
            }
            let mut there = to_one(&inverse, &inverse_label, &from.meta.name_singular, name);
            there.custom = true;
            check_columns(pool, &to, &there).await?;
            insert_field(pool, &from.meta.id, &here, next_position(pool, &from.meta.id).await?).await?;
            insert_field(pool, &to.meta.id, &there, next_position(pool, &to.meta.id).await?).await?;
            let far = object(pool, &to.meta.id).await?;
            let far_field = far.field(&inverse).cloned().ok_or_else(|| anyhow!("the far field just made is missing"))?;
            ensure_columns(pool, &far.meta.table_name, &far_field, Some((&from.meta.table_name, from.meta.is_custom))).await?;
            super::views::adopt_field(pool, &far, &far_field).await?;
            Ok(object(pool, &from.meta.id).await?.field(name).cloned().ok_or_else(|| anyhow!("the field just made is missing"))?)
        }
        RelationInputKind::ManyToMany => {
            // Twenty's many-to-many: a junction object, a many-to-one to each side, and a
            // one-to-many on each side through it. Everything that can be refused is checked
            // before the junction exists.
            let inverse_label = inverse_label.unwrap_or_else(|| from.meta.label_plural.clone());
            let inverse = field_name(pool, &to, &inverse_label).await?;
            distinct_sides(from, &to, name, &inverse)?;
            let junction_label = format!("{} {}", from.meta.label_singular, to.meta.label_singular);
            let junction = create_object(
                pool,
                ObjectInput { label_singular: junction_label.clone(), label_plural: format!("{junction_label}s"), icon: "IconArrowsExchange".into(), description: format!("Links {} and {}", from.meta.label_plural, to.meta.label_plural), skip_name: true },
            )
            .await?;
            let from_side = &from.meta.name_singular;
            let to_side = if to.meta.name_singular == *from_side { format!("{}2", to.meta.name_singular) } else { to.meta.name_singular.clone() };
            let mut here = to_many(name, label, &junction.meta.name_singular, from_side);
            here.custom = true;
            here.icon = input.icon.trim().to_owned();
            here.description = input.description.trim().to_owned();
            if let Some(r) = here.options.relation.as_mut() {
                r.junction = true;
            }
            let mut there = to_many(&inverse, &inverse_label, &junction.meta.name_singular, &to_side);
            there.custom = true;
            if let Some(r) = there.options.relation.as_mut() {
                r.junction = true;
            }
            let mut j_from = to_one(from_side, &from.meta.label_singular, from_side, name);
            j_from.custom = true;
            let mut j_to = to_one(&to_side, &to.meta.label_singular, &to.meta.name_singular, &inverse);
            j_to.custom = true;
            insert_field(pool, &from.meta.id, &here, next_position(pool, &from.meta.id).await?).await?;
            insert_field(pool, &to.meta.id, &there, next_position(pool, &to.meta.id).await?).await?;
            insert_field(pool, &junction.meta.id, &j_from, next_position(pool, &junction.meta.id).await?).await?;
            insert_field(pool, &junction.meta.id, &j_to, next_position(pool, &junction.meta.id).await?).await?;
            let junction = object(pool, &junction.meta.id).await?;
            for (field_name, table, custom) in [(from_side.as_str(), from.meta.table_name.as_str(), from.meta.is_custom), (to_side.as_str(), to.meta.table_name.as_str(), to.meta.is_custom)] {
                let f = junction.field(field_name).cloned().ok_or_else(|| anyhow!("the junction's field is missing"))?;
                ensure_columns(pool, &junction.meta.table_name, &f, Some((table, custom))).await?;
                super::views::adopt_field(pool, &junction, &f).await?;
            }
            Ok(object(pool, &from.meta.id).await?.field(name).cloned().ok_or_else(|| anyhow!("the field just made is missing"))?)
        }
    }
}

fn field_of<'a>(object_def: &'a ObjectDef, field_id: &str) -> Result<&'a FieldMeta> {
    object_def.fields.iter().find(|f| f.id == field_id).ok_or_else(|| anyhow!(AppError::not_found("No such field")))
}

/// The inverse of a relation field, when it has one.
async fn inverse_of(pool: &SqlitePool, field: &FieldMeta) -> Result<Option<(String, FieldMeta)>> {
    let Some(rel) = &field.options.relation else { return Ok(None) };
    let Some(inverse) = &rel.inverse else { return Ok(None) };
    let Ok(target) = object(pool, &rel.target).await else { return Ok(None) };
    Ok(target.field(inverse).cloned().map(|f| (target.meta.id.clone(), f)))
}

/// Label, description, icon, options, default, required, unique — never the type or the
/// column. Unique is refused while duplicate values exist, deleted records included (Twenty).
pub async fn update_field(pool: &SqlitePool, object_def: &ObjectDef, field_id: &str, patch: FieldPatch) -> Result<FieldMeta> {
    let current = field_of(object_def, field_id)?;
    let label = patch.label.map(|l| l.trim().to_owned()).unwrap_or_else(|| current.label.clone());
    if label.is_empty() {
        return Err(invalid("A field needs a label"));
    }
    let changed_options = patch.options.is_some();
    let mut options = patch.options.map(|o| clean_options(&o)).unwrap_or_else(|| current.options.clone());
    // The relation itself never changes here; only Twenty's junction toggle does.
    match (&mut options.relation, &current.options.relation) {
        (Some(next), Some(was)) => {
            let junction = next.junction;
            *next = was.clone();
            next.junction = junction;
        }
        (next, was) => *next = was.clone(),
    }
    if matches!(current.r#type, FieldType::Select | FieldType::MultiSelect) && options.select.is_empty() {
        return Err(invalid("A select needs at least one option"));
    }
    let unique = patch.unique.unwrap_or(current.is_unique);
    if unique && !current.is_unique {
        if let Some(col) = columns(current).first() {
            let (dupes,): (i64,) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM (SELECT {c} FROM {t} WHERE {c} IS NOT NULL AND {c} <> '' GROUP BY {c} HAVING count(*) > 1)",
                c = col.name,
                t = object_def.meta.table_name
            )))
            .fetch_one(pool)
            .await?;
            if dupes > 0 {
                return Err(invalid(format!("{label} has duplicate values (deleted records included), so it can't be unique yet")));
            }
        }
    }
    // A JSON null can't reach an Option (serde reads it as absent), so an empty string clears.
    let default_json = match patch.default_value {
        Some(Json(v)) if v.is_null() || v.as_str() == Some("") => None,
        Some(Json(v)) => resolve_default(current.r#type, &options, Some(v))?.map(|d| d.to_string()),
        None if changed_options => resolve_default(current.r#type, &options, current.default_value.clone().map(|d| d.0)).ok().flatten().map(|d| d.to_string()),
        None => current.default_value.as_ref().map(|d| d.0.to_string()),
    };
    sqlx::query(
        "UPDATE field_meta SET label = ?, description = ?, icon = ?, options_json = ?, default_json = ?, is_nullable = ?, is_unique = ?, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(&label)
    .bind(patch.description.map(|d| d.trim().to_owned()).unwrap_or_else(|| current.description.clone()))
    .bind(patch.icon.map(|i| i.trim().to_owned()).unwrap_or_else(|| current.icon.clone()))
    .bind(serde_json::to_string(&options)?)
    .bind(default_json)
    .bind(!patch.required.unwrap_or(!current.is_nullable))
    .bind(unique)
    .bind(field_id)
    .execute(pool)
    .await?;
    Ok(field_of(&object(pool, &object_def.meta.id).await?, field_id)?.clone())
}

/// Off hides the field (and a relation's far side); the column keeps its values. The label
/// identifier and the system fields stay on.
pub async fn set_field_active(pool: &SqlitePool, object_def: &ObjectDef, field_id: &str, active: bool) -> Result<FieldMeta> {
    let current = field_of(object_def, field_id)?;
    if current.is_system {
        return Err(invalid("A system field stays as it is"));
    }
    if !active && object_def.meta.label_identifier_field_id.as_deref() == Some(field_id) {
        return Err(invalid("The label identifier stays on — pick another first"));
    }
    let mut ids = vec![field_id.to_owned()];
    if let Some((_, inverse)) = inverse_of(pool, current).await? {
        ids.push(inverse.id);
    }
    for id in ids {
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE field_meta SET is_active = ?, updated_at = {NOW} WHERE id = ?")))
            .bind(active)
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(field_of(&object(pool, &object_def.meta.id).await?, field_id)?.clone())
}

/// A custom field that is off: its row goes (and a relation's far side); the column stays.
pub async fn delete_field(pool: &SqlitePool, object_def: &ObjectDef, field_id: &str) -> Result<()> {
    let current = field_of(object_def, field_id)?;
    if !current.is_custom {
        return Err(invalid("A standard field can be deactivated, not deleted"));
    }
    if current.is_active {
        return Err(invalid("Deactivate the field first"));
    }
    let mut ids = vec![field_id.to_owned()];
    if let Some((_, inverse)) = inverse_of(pool, current).await? {
        ids.push(inverse.id);
    }
    for id in ids {
        sqlx::query("DELETE FROM field_meta WHERE id = ?").bind(id).execute(pool).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::kernel::records::query::{self, Actor, Query};
    use serde_json::json;

    #[test]
    fn labels_become_names() {
        assert_eq!(camel("Project Assignment"), "projectAssignment");
        assert_eq!(camel("  partnership "), "partnership");
        assert_eq!(camel("2nd owner"), "f2ndOwner");
        assert_eq!(camel("Đối tác"), "doiTac");
        assert_eq!(screaming("On site"), "ON_SITE");
    }

    /// Partnership: a select, a relation to Companies, a many-to-many to People through a
    /// junction — all usable through the engine; renamed, switched off and on, deleted; a
    /// dropped table comes back from repair.
    #[test]
    fn a_custom_object_lives_through_the_engine() {
        let root = std::env::temp_dir().join(format!("newdawn-ddl-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let actor = Actor::system("test");
            let made = create_object(&pool, ObjectInput { label_singular: "Partnership".into(), label_plural: "Partnerships".into(), icon: "IconHeartHandshake".into(), description: String::new(), skip_name: false })
                .await
                .unwrap();
            assert_eq!(made.meta.table_name, "custom_partnership");
            assert!(made.meta.is_custom && made.meta.is_active);
            assert_eq!(made.label_field().unwrap().name, "name");
            assert!(table_columns(&pool, "custom_partnership").await.unwrap().contains(&"name".to_string()));
            assert!(table_columns(&pool, "task_target").await.unwrap().contains(&"partnership_id".to_string()));
            assert!(object(&pool, "taskTarget").await.unwrap().field("partnership").is_some());
            assert!(create_object(&pool, ObjectInput { label_singular: "Partnership".into(), label_plural: "Deals".into(), icon: String::new(), description: String::new(), skip_name: false }).await.is_err(), "taken");

            let stage = create_field(
                &pool,
                &made,
                FieldInput {
                    label: "Stage".into(),
                    r#type: FieldType::Select,
                    description: String::new(),
                    icon: String::new(),
                    options: FieldOptions {
                        select: vec![SelectOption { value: String::new(), label: "Lead".into(), color: "green".into(), position: 0.0 }, SelectOption { value: String::new(), label: "Signed".into(), color: "sky".into(), position: 1.0 }],
                        ..Default::default()
                    },
                    default_value: None,
                    required: true,
                    unique: false,
                    relation: None,
                },
            )
            .await
            .unwrap();
            assert_eq!(stage.options.select[1].value, "SIGNED");
            assert!(!stage.is_nullable && stage.is_custom);
            let company_field = create_field(
                &pool,
                &object(&pool, "partnership").await.unwrap(),
                FieldInput { label: "Company".into(), r#type: FieldType::Relation, description: String::new(), icon: String::new(), options: FieldOptions::default(), default_value: None, required: false, unique: false, relation: Some(RelationInput { target: "companies".into(), kind: RelationInputKind::ManyToOne, inverse_label: None, junction: false }) },
            )
            .await
            .unwrap();
            assert_eq!(company_field.options.relation.as_ref().unwrap().inverse.as_deref(), Some("partnerships"));
            let company = object(&pool, "company").await.unwrap();
            assert_eq!(company.field("partnerships").unwrap().options.relation.as_ref().unwrap().kind, RelationKind::OneToMany);
            assert!(table_columns(&pool, "custom_partnership").await.unwrap().contains(&"company_id".to_string()));

            let partners = create_field(
                &pool,
                &object(&pool, "partnership").await.unwrap(),
                FieldInput { label: "Partners".into(), r#type: FieldType::Relation, description: String::new(), icon: String::new(), options: FieldOptions::default(), default_value: None, required: false, unique: false, relation: Some(RelationInput { target: "person".into(), kind: RelationInputKind::ManyToMany, inverse_label: None, junction: false }) },
            )
            .await
            .unwrap();
            let rel = partners.options.relation.as_ref().unwrap();
            assert!(rel.junction && rel.kind == RelationKind::OneToMany && rel.target == "partnershipPerson");
            let junction = object(&pool, "partnershipPerson").await.unwrap();
            assert_eq!(junction.meta.table_name, "custom_partnership_person");
            assert!(junction.field("partnership").is_some() && junction.field("person").is_some());
            assert!(object(&pool, "person").await.unwrap().field("partnerships").is_some());

            // Records through the engine, with the new fields.
            let keppel = query::create(&pool, &company, &json!({ "name": "Keppel Land" }), &actor).await.unwrap();
            let partnership = object(&pool, "partnership").await.unwrap();
            let deal = query::create(&pool, &partnership, &json!({ "name": "VIFC × Keppel", "stage": "SIGNED", "company": keppel["id"] }), &actor).await.unwrap();
            assert_eq!(deal["stage"], "SIGNED");
            assert_eq!(deal["company"]["label"], "Keppel Land");
            let signed = Query { filter: Some(Json(json!({ "field": "stage", "op": "is", "value": ["SIGNED"] }))), ..Default::default() };
            assert_eq!(query::list(&pool, &partnership, &signed).await.unwrap().total, 1);
            let link = query::create(&pool, &junction, &json!({ "partnership": deal["id"], "person": query::create(&pool, &object(&pool, "person").await.unwrap(), &json!({ "name": { "firstName": "Harry", "lastName": "Do" } }), &actor).await.unwrap()["id"] }), &actor)
                .await
                .unwrap();
            assert_eq!(link["partnership"]["label"], "VIFC × Keppel");

            // Edits — and what the guards refuse.


            assert!(create_field(&pool, &partnership, FieldInput { label: "Company id".into(), r#type: FieldType::Text, description: String::new(), icon: String::new(), options: FieldOptions::default(), default_value: None, required: false, unique: false, relation: None }).await.is_err(), "a label that would alias the company_id column");


            assert!(create_field(&pool, &partnership, FieldInput { label: "Created by name".into(), r#type: FieldType::Text, description: String::new(), icon: String::new(), options: FieldOptions::default(), default_value: None, required: false, unique: false, relation: None }).await.is_err(), "a base column");


            assert!(create_field(&pool, &object(&pool, "taskTarget").await.unwrap(), FieldInput { label: "Note".into(), r#type: FieldType::Text, description: String::new(), icon: String::new(), options: FieldOptions::default(), default_value: None, required: false, unique: false, relation: None }).await.is_err(), "no fields on a system object");


            assert!(create_field(&pool, &partnership, FieldInput { label: "Tier".into(), r#type: FieldType::Select, description: String::new(), icon: String::new(), options: FieldOptions { select: vec![SelectOption { value: String::new(), label: "Gold".into(), color: "yellow".into(), position: 0.0 }], ..Default::default() }, default_value: Some(Json(json!("Platinum"))), required: false, unique: false, relation: None }).await.is_err(), "a default outside the options");
            let renamed = update_field(&pool, &partnership, &stage.id, FieldPatch { label: Some("Status".into()), ..Default::default() }).await.unwrap();
            assert_eq!((renamed.label.as_str(), renamed.name.as_str()), ("Status", "stage"));
            query::create(&pool, &partnership, &json!({ "name": "VIFC × Keppel" }), &actor).await.unwrap();
            let name_id = partnership.label_field().unwrap().id.clone();
            assert!(update_field(&pool, &partnership, &name_id, FieldPatch { unique: Some(true), ..Default::default() }).await.is_err(), "duplicates");
            assert!(set_field_active(&pool, &partnership, &name_id, false).await.is_err(), "the label identifier stays on");
            let off = set_field_active(&pool, &partnership, &company_field.id, false).await.unwrap();
            assert!(!off.is_active);
            assert!(!object(&pool, "company").await.unwrap().field("partnerships").unwrap().is_active, "the far side follows");
            assert!(delete_field(&pool, &partnership, &stage.id).await.is_err(), "on, so not deletable");
            delete_field(&pool, &object(&pool, "partnership").await.unwrap(), &company_field.id).await.unwrap();
            assert!(object(&pool, "company").await.unwrap().field("partnerships").is_none());

            // Off, on, off, gone; the junction object stays until deleted itself.
            assert!(delete_object(&pool, &partnership.meta.id).await.is_err(), "on");
            let off = set_object_active(&pool, &partnership.meta.id, false).await.unwrap();
            assert!(!off.meta.is_active);
            let (rows,): (i64,) = sqlx::query_as("SELECT count(*) FROM search_index WHERE object = 'partnership'").fetch_one(&pool).await.unwrap();
            assert_eq!(rows, 0);
            assert!(set_object_active(&pool, &partnership.meta.id, true).await.unwrap().meta.is_active);
            set_object_active(&pool, &partnership.meta.id, false).await.unwrap();
            delete_object(&pool, &partnership.meta.id).await.unwrap();
            assert!(object(&pool, "partnership").await.is_err());
            assert!(object(&pool, "partnershipPerson").await.unwrap().field("partnership").is_none(), "the relation to it went");
            let (tables,): (i64,) = sqlx::query_as("SELECT count(*) FROM sqlite_master WHERE name = 'custom_partnership'").fetch_one(&pool).await.unwrap();
            assert_eq!(tables, 0);

            // Repair brings back what the metadata says.
            sqlx::query("DROP TABLE custom_partnership_person").execute(&pool).await.unwrap();
            repair(&pool).await.unwrap();
            let cols = table_columns(&pool, "custom_partnership_person").await.unwrap();
            assert!(cols.contains(&"person_id".to_string()) && cols.contains(&"created_by_source".to_string()));
            hub.close_all().await;
        });
        std::fs::remove_dir_all(&root).unwrap();
    }
}
