//! The CRM's assistant (D-070, 0.10.13): its agents (settings, like Twenty's), chat threads —
//! one may sit on a record — and a turn loop on the local model with tools over the record
//! engine. Lookups run at once; a change comes back as a proposal only the owner applies.

pub mod tools;

use std::future::Future;

use anyhow::{anyhow, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use sqlx::SqlitePool;
use tauri::AppHandle;

use crate::error::AppError;
use crate::kernel::ai::ollama::{self, ChatReply, TextSession};
use crate::kernel::ai::queue::{Lane, Task};
use crate::kernel::records::types::Json;
use crate::kernel::records::{self, query, Actor};

/// Lookups the model may make before it answers.
pub const MAX_ROUNDS: usize = 6;
const MAX_TOKENS: u32 = 1500;
/// What a tool's result may take of the conversation: this turn's in full (a 20-record lookup
/// fits), an earlier turn's in brief, so a long chat stays within the model's context.
const TOOL_CHARS: usize = 9000;
const EARLIER_TOOL_CHARS: usize = 600;
/// The messages of a thread the model reads again (the latest).
const HISTORY: usize = 40;
const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
const DEFAULT_INSTRUCTIONS: &str = "You help the owner find, count and understand the records of their CRM, and propose changes for them to apply.";
const GROUND_RULES: &str = "Look things up with the tools — never invent a record, a number or a date. When you name a record, link it as [its label](its link), with the link a tool gave you. Amounts are in their currency's units. To create or change anything — a record, a field, a note — use a propose_ tool: the owner applies it, so say that it is proposed, not done. Answer briefly, in the owner's language.";

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

// ---- agents -------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Type)]
pub struct AssistantAgent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    /// "" = the app's selected model.
    pub model: String,
    pub tools: Vec<String>,
    pub is_default: bool,
}

#[derive(sqlx::FromRow)]
struct AssistantAgentRow {
    id: String,
    name: String,
    description: String,
    instructions: String,
    model: String,
    tools_json: String,
    is_default: bool,
}

impl From<AssistantAgentRow> for AssistantAgent {
    fn from(r: AssistantAgentRow) -> Self {
        let tools: Vec<String> = serde_json::from_str(&r.tools_json).unwrap_or_default();
        Self { id: r.id, name: r.name, description: r.description, instructions: r.instructions, model: r.model, tools, is_default: r.is_default }
    }
}

const AGENT_COLUMNS: &str = "id, name, description, instructions, model, tools_json, is_default";

#[derive(Debug, Clone, Deserialize, Type)]
pub struct AssistantAgentInput {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub tools: Vec<String>,
}

/// The default agent, made the first time it's asked for (two asks at once make one).
async fn ensure_default(pool: &SqlitePool) -> Result<()> {
    sqlx::query("INSERT INTO crm_agent (id, name, description, instructions, tools_json, is_default) VALUES (?, 'Assistant', 'Answers over the CRM and proposes changes', ?, ?, 1) ON CONFLICT DO NOTHING")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(DEFAULT_INSTRUCTIONS)
        .bind(json!(tools::ALL).to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// The agents, the default first.
pub async fn agents(pool: &SqlitePool) -> Result<Vec<AssistantAgent>> {
    ensure_default(pool).await?;
    let rows: Vec<AssistantAgentRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {AGENT_COLUMNS} FROM crm_agent ORDER BY is_default DESC, position, name"))).fetch_all(pool).await?;
    Ok(rows.into_iter().map(AssistantAgent::from).collect())
}

/// An agent by id, or the default one (also when the id is gone).
pub async fn agent(pool: &SqlitePool, id: Option<&str>) -> Result<AssistantAgent> {
    ensure_default(pool).await?;
    if let Some(id) = id.filter(|i| !i.is_empty()) {
        let row: Option<AssistantAgentRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {AGENT_COLUMNS} FROM crm_agent WHERE id = ?"))).bind(id).fetch_optional(pool).await?;
        if let Some(r) = row {
            return Ok(r.into());
        }
    }
    let row: AssistantAgentRow = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {AGENT_COLUMNS} FROM crm_agent WHERE is_default = 1"))).fetch_one(pool).await?;
    Ok(row.into())
}

pub async fn save_agent(pool: &SqlitePool, input: &AssistantAgentInput) -> Result<AssistantAgent> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(invalid("An agent needs a name"));
    }
    if !input.model.is_empty() && !ollama::valid_name(&input.model) {
        return Err(invalid("No model by that name"));
    }
    let tools: Vec<&str> = tools::ALL.iter().copied().filter(|t| input.tools.iter().any(|x| x == t)).collect();
    let id = match input.id.as_deref().filter(|i| !i.is_empty()) {
        Some(id) => {
            let done = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE crm_agent SET name = ?, description = ?, instructions = ?, model = ?, tools_json = ?, updated_at = {NOW} WHERE id = ?")))
                .bind(name)
                .bind(input.description.trim())
                .bind(input.instructions.trim())
                .bind(&input.model)
                .bind(json!(tools).to_string())
                .bind(id)
                .execute(pool)
                .await?;
            if done.rows_affected() == 0 {
                return Err(anyhow!(AppError::not_found("No such agent")));
            }
            id.to_owned()
        }
        None => {
            let id = uuid::Uuid::now_v7().to_string();
            let (position,): (f64,) = sqlx::query_as("SELECT coalesce(max(position), 0.0) + 1.0 FROM crm_agent").fetch_one(pool).await?;
            sqlx::query("INSERT INTO crm_agent (id, name, description, instructions, model, tools_json, position) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .bind(&id)
                .bind(name)
                .bind(input.description.trim())
                .bind(input.instructions.trim())
                .bind(&input.model)
                .bind(json!(tools).to_string())
                .bind(position)
                .execute(pool)
                .await?;
            id
        }
    };
    agent(pool, Some(&id)).await
}

/// An agent goes; its threads stay, answered by the default one from then on.
pub async fn delete_agent(pool: &SqlitePool, id: &str) -> Result<()> {
    let known: Option<(bool,)> = sqlx::query_as("SELECT is_default FROM crm_agent WHERE id = ?").bind(id).fetch_optional(pool).await?;
    match known {
        None => Err(anyhow!(AppError::not_found("No such agent"))),
        Some((true,)) => Err(invalid("The default agent stays")),
        Some((false,)) => {
            sqlx::query("DELETE FROM crm_agent WHERE id = ?").bind(id).execute(pool).await?;
            Ok(())
        }
    }
}

/// An agent by id that must still be there (a workflow's step names one).
pub async fn agent_by_id(pool: &SqlitePool, id: &str) -> Result<AssistantAgent> {
    let row: Option<AssistantAgentRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {AGENT_COLUMNS} FROM crm_agent WHERE id = ?"))).bind(id).fetch_optional(pool).await?;
    row.map(AssistantAgent::from).ok_or_else(|| invalid("The step's agent was deleted — pick another"))
}

// ---- threads and messages -----------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Type)]
pub struct ChatThread {
    pub id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub title: String,
    /// A thread on a record: its object's singular name and id (empty otherwise).
    pub record_object: String,
    pub record_id: String,
    pub record_label: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct ChatThreadRow {
    id: String,
    agent_id: Option<String>,
    agent_name: Option<String>,
    title: String,
    record_object: String,
    record_id: String,
    updated_at: String,
}

const THREAD_SQL: &str = "SELECT t.id, t.agent_id, a.name AS agent_name, t.title, t.record_object, t.record_id, t.updated_at FROM crm_chat_thread t LEFT JOIN crm_agent a ON a.id = t.agent_id";

async fn record_label(pool: &SqlitePool, object: &str, id: &str) -> String {
    if object.is_empty() || id.is_empty() {
        return String::new();
    }
    let Ok(def) = records::object(pool, object).await else { return String::new() };
    match query::get(pool, &def, id, true).await {
        Ok(r) => tools::label_of(&def, &r),
        Err(_) => String::new(),
    }
}

async fn thread_of(pool: &SqlitePool, r: ChatThreadRow) -> ChatThread {
    let record_label = record_label(pool, &r.record_object, &r.record_id).await;
    ChatThread { id: r.id, agent_id: r.agent_id.unwrap_or_default(), agent_name: r.agent_name.unwrap_or_else(|| "Assistant".into()), title: r.title, record_object: r.record_object, record_id: r.record_id, record_label, updated_at: r.updated_at }
}

async fn thread_row(pool: &SqlitePool, id: &str) -> Result<ChatThreadRow> {
    let row: Option<ChatThreadRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{THREAD_SQL} WHERE t.id = ?"))).bind(id).fetch_optional(pool).await?;
    row.ok_or_else(|| anyhow!(AppError::not_found("No such chat")))
}

/// The chats, newest first — all, or a record's.
pub async fn threads(pool: &SqlitePool, record: Option<(&str, &str)>) -> Result<Vec<ChatThread>> {
    let rows: Vec<ChatThreadRow> = match record {
        Some((object, id)) => sqlx::query_as(sqlx::AssertSqlSafe(format!("{THREAD_SQL} WHERE t.record_object = ? AND t.record_id = ? ORDER BY t.updated_at DESC LIMIT 100"))).bind(object).bind(id).fetch_all(pool).await?,
        None => sqlx::query_as(sqlx::AssertSqlSafe(format!("{THREAD_SQL} ORDER BY t.updated_at DESC LIMIT 100"))).fetch_all(pool).await?,
    };
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(thread_of(pool, r).await);
    }
    Ok(out)
}

/// A new chat with an agent (the default when none), on a record or not.
pub async fn new_thread(pool: &SqlitePool, agent_id: Option<&str>, record_object: &str, record_id: &str) -> Result<ChatThread> {
    let agent = agent(pool, agent_id).await?;
    // A chat on a record: an object the assistant works on, kept by its singular name (what
    // `forget_visits` detaches by).
    let object = if record_object.is_empty() {
        String::new()
    } else {
        let def = tools::object_named(pool, record_object).await?;
        query::get(pool, &def, record_id, false).await?;
        def.meta.name_singular
    };
    let id = uuid::Uuid::now_v7().to_string();
    sqlx::query("INSERT INTO crm_chat_thread (id, agent_id, record_object, record_id) VALUES (?, ?, ?, ?)").bind(&id).bind(&agent.id).bind(&object).bind(if object.is_empty() { "" } else { record_id }).execute(pool).await?;
    Ok(thread_of(pool, thread_row(pool, &id).await?).await)
}

pub async fn delete_thread(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM crm_chat_thread WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ChatMessage {
    pub id: String,
    /// user · assistant · tool
    pub role: String,
    pub content: String,
    pub tool_name: String,
    /// assistant: the tool calls; tool: the result.
    pub tool: Json,
    /// A change the agent proposes (`{kind, object, id, fields, summary}`), or null.
    pub proposal: Json,
    /// "" · PROPOSED · APPLYING · APPLIED · DISMISSED
    pub proposal_status: String,
    /// What applying made (a record's id), or why it couldn't.
    pub proposal_result: String,
    pub model: String,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: String,
    role: String,
    content: String,
    tool_name: String,
    tool_json: String,
    proposal_json: String,
    proposal_status: String,
    proposal_result: String,
    model: String,
    created_at: String,
}

impl From<MessageRow> for ChatMessage {
    fn from(r: MessageRow) -> Self {
        let read = |s: &str| if s.is_empty() { Value::Null } else { serde_json::from_str(s).unwrap_or(Value::Null) };
        Self { id: r.id, role: r.role, content: r.content, tool_name: r.tool_name, tool: Json(read(&r.tool_json)), proposal: Json(read(&r.proposal_json)), proposal_status: r.proposal_status, proposal_result: r.proposal_result, model: r.model, created_at: r.created_at }
    }
}

const MESSAGE_COLUMNS: &str = "id, role, content, tool_name, tool_json, proposal_json, proposal_status, proposal_result, model, created_at";

pub async fn messages(pool: &SqlitePool, thread_id: &str) -> Result<Vec<ChatMessage>> {
    let rows: Vec<MessageRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {MESSAGE_COLUMNS} FROM crm_chat_message WHERE thread_id = ? ORDER BY created_at, rowid"))).bind(thread_id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(ChatMessage::from).collect())
}

async fn message(pool: &SqlitePool, id: &str) -> Result<ChatMessage> {
    let row: Option<MessageRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {MESSAGE_COLUMNS} FROM crm_chat_message WHERE id = ?"))).bind(id).fetch_optional(pool).await?;
    row.map(ChatMessage::from).ok_or_else(|| anyhow!(AppError::not_found("No such message")))
}

struct NewMessage<'a> {
    role: &'a str,
    content: &'a str,
    tool_name: &'a str,
    tool: Option<&'a Value>,
    proposal: Option<&'a Value>,
    model: &'a str,
}

async fn add(pool: &SqlitePool, thread_id: &str, m: NewMessage<'_>) -> Result<()> {
    sqlx::query("INSERT INTO crm_chat_message (id, thread_id, role, content, tool_name, tool_json, proposal_json, proposal_status, model) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(thread_id)
        .bind(m.role)
        .bind(m.content)
        .bind(m.tool_name)
        .bind(m.tool.map(Value::to_string).unwrap_or_default())
        .bind(m.proposal.map(Value::to_string).unwrap_or_default())
        .bind(if m.proposal.is_some() { "PROPOSED" } else { "" })
        .bind(m.model)
        .execute(pool)
        .await?;
    Ok(())
}

// ---- the turn -----------------------------------------------------------------------------

/// The model a turn talks to: the local one through the gateway, or a script in tests.
pub trait Model {
    fn reply(&self, messages: Vec<Value>, tools: &Value) -> impl Future<Output = Result<ChatReply>> + Send;
    fn name(&self) -> String;
}

/// The local model, on a turn of the AI queue.
pub struct Gateway<'a>(pub TextSession<'a>);

impl Model for Gateway<'_> {
    fn reply(&self, messages: Vec<Value>, tools: &Value) -> impl Future<Output = Result<ChatReply>> + Send {
        self.0.chat(messages, tools, MAX_TOKENS)
    }

    fn name(&self) -> String {
        self.0.model().to_owned()
    }
}

fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        format!("{}…", text.chars().take(max).collect::<String>())
    }
}

/// The system message: the agent's instructions, today, the ground rules, the objects, and the
/// record the chat is on.
async fn system(pool: &SqlitePool, agent: &AssistantAgent, on: Option<(&str, &str)>) -> Result<Value> {
    let mut text = format!("{}\n\nToday is {} ({}). {GROUND_RULES}\n\n{}", if agent.instructions.trim().is_empty() { DEFAULT_INSTRUCTIONS } else { agent.instructions.trim() }, Local::now().format("%A %-d %B %Y"), Local::now().format("%Y-%m-%d"), tools::catalogue(pool).await?);
    if let Some((object, id)) = on.filter(|(o, i)| !o.is_empty() && !i.is_empty()) {
        if let Ok(def) = records::object(pool, object).await {
            if let Ok(record) = query::get(pool, &def, id, false).await {
                let brief = tools::run(pool, &[String::from("open_record")], &ollama::ToolCall { name: "open_record".into(), arguments: json!({ "object": object, "id": id }) }).await.result;
                text.push_str(&format!("\nThis chat is about the {} {} — “this {}” means it: {}\n", def.meta.label_singular.to_lowercase(), tools::label_of(&def, &record), def.meta.label_singular.to_lowercase(), brief));
            }
        }
    }
    Ok(json!({ "role": "system", "content": text }))
}

/// The conversation the model reads: the system message, then the thread's latest messages.
async fn conversation(pool: &SqlitePool, thread: &ChatThreadRow, agent: &AssistantAgent) -> Result<Vec<Value>> {
    let mut out = vec![system(pool, agent, Some((&thread.record_object, &thread.record_id))).await?];
    let all = messages(pool, &thread.id).await?;
    let start = all.len().saturating_sub(HISTORY);
    // Never start the history on a tool's result: its call would be missing.
    let start = (start..all.len()).find(|&i| all[i].role == "user").unwrap_or(all.len());
    for m in &all[start..] {
        match m.role.as_str() {
            "user" => out.push(json!({ "role": "user", "content": m.content })),
            "assistant" => {
                let mut msg = json!({ "role": "assistant", "content": m.content });
                if let Some(calls) = m.tool.0.as_array().filter(|a| !a.is_empty()) {
                    msg["tool_calls"] = Value::Array(calls.clone());
                }
                out.push(msg);
            }
            "tool" => out.push(json!({ "role": "tool", "content": clip(&m.tool.0.to_string(), EARLIER_TOOL_CHARS), "tool_name": m.tool_name })),
            _ => {}
        }
    }
    Ok(out)
}

/// One turn: the model reads the thread and answers, looking things up and proposing changes on
/// the way (each call and result kept in the thread); after MAX_ROUNDS lookups it stops.
pub async fn turn(pool: &SqlitePool, thread_id: &str, model: &impl Model) -> Result<()> {
    let thread = thread_row(pool, thread_id).await?;
    let agent = agent(pool, thread.agent_id.as_deref()).await?;
    let schema = tools::schema(&agent.tools);
    let mut conversation = conversation(pool, &thread, &agent).await?;
    let name = model.name();
    for _ in 0..MAX_ROUNDS {
        let reply = model.reply(conversation.clone(), &schema).await?;
        if reply.tool_calls.is_empty() {
            let content = if reply.content.trim().is_empty() { "I have nothing to add." } else { reply.content.trim() };
            add(pool, thread_id, NewMessage { role: "assistant", content, tool_name: "", tool: None, proposal: None, model: &name }).await?;
            return Ok(());
        }
        let calls = Value::Array(reply.tool_calls.iter().map(|c| json!({ "function": { "name": c.name, "arguments": c.arguments } })).collect());
        add(pool, thread_id, NewMessage { role: "assistant", content: reply.content.trim(), tool_name: "", tool: Some(&calls), proposal: None, model: &name }).await?;
        conversation.push(json!({ "role": "assistant", "content": reply.content, "tool_calls": calls }));
        for call in &reply.tool_calls {
            let outcome = tools::run(pool, &agent.tools, call).await;
            add(pool, thread_id, NewMessage { role: "tool", content: "", tool_name: &call.name, tool: Some(&outcome.result), proposal: outcome.proposal.as_ref(), model: "" }).await?;
            conversation.push(json!({ "role": "tool", "content": clip(&outcome.result.to_string(), TOOL_CHARS), "tool_name": call.name }));
        }
    }
    let stopped = format!("I stopped after {MAX_ROUNDS} lookups without an answer — ask again, a little more narrowly.");
    add(pool, thread_id, NewMessage { role: "assistant", content: &stopped, tool_name: "", tool: None, proposal: None, model: &name }).await?;
    Ok(())
}

/// The owner asks: the question kept, the thread titled from it the first time, a turn on the
/// local model (the agent's, else the app's) in the lane the owner waits in. The thread after.
pub async fn ask(app: &AppHandle, pool: &SqlitePool, thread_id: &str, text: &str) -> Result<Vec<ChatMessage>> {
    let text = text.trim();
    if text.is_empty() {
        return Err(invalid("Ask something"));
    }
    let thread = thread_row(pool, thread_id).await?;
    let agent = agent(pool, thread.agent_id.as_deref()).await?;
    // The session first: when the model can't be reached, the question isn't kept — the panel
    // gives it back to ask again.
    let model = (!agent.model.is_empty()).then(|| agent.model.clone());
    let task = Task::new(format!("{} · {}", agent.name, clip(text, 40)));
    let mut session = ollama::session(app, pool.clone(), None, model, Lane::Interactive, task).await?;
    session.for_agent("assistant");
    add(pool, thread_id, NewMessage { role: "user", content: text, tool_name: "", tool: None, proposal: None, model: "" }).await?;
    if thread.title.is_empty() {
        sqlx::query("UPDATE crm_chat_thread SET title = ? WHERE id = ?").bind(clip(text, 60)).bind(thread_id).execute(pool).await?;
    }
    turn(pool, thread_id, &Gateway(session)).await?;
    messages(pool, thread_id).await
}

/// An agent's answer to one prompt, without a thread and with lookups only (a workflow's AI
/// step on an agent): the final words.
pub async fn answer(pool: &SqlitePool, agent: &AssistantAgent, model: &impl Model, prompt: &str) -> Result<String> {
    let allowed: Vec<String> = agent.tools.iter().filter(|t| tools::READ.contains(&t.as_str())).cloned().collect();
    let schema = tools::schema(&allowed);
    let mut conversation = vec![system(pool, agent, None).await?, json!({ "role": "user", "content": prompt })];
    for _ in 0..MAX_ROUNDS {
        let reply = model.reply(conversation.clone(), &schema).await?;
        if reply.tool_calls.is_empty() {
            return Ok(reply.content.trim().to_owned());
        }
        let calls: Vec<Value> = reply.tool_calls.iter().map(|c| json!({ "function": { "name": c.name, "arguments": c.arguments } })).collect();
        conversation.push(json!({ "role": "assistant", "content": reply.content, "tool_calls": calls }));
        for call in &reply.tool_calls {
            let outcome = tools::run(pool, &allowed, call).await;
            conversation.push(json!({ "role": "tool", "content": clip(&outcome.result.to_string(), TOOL_CHARS), "tool_name": call.name }));
        }
    }
    Err(invalid(format!("{} didn't answer within {MAX_ROUNDS} lookups", agent.name)))
}

// ---- proposals ----------------------------------------------------------------------------

/// The owner applies a proposal: taken once (a second click finds it gone), written through the
/// engine as the agent, its result kept; a refusal leaves it proposed, with the reason.
pub async fn apply(pool: &SqlitePool, message_id: &str) -> Result<ChatMessage> {
    let taken = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE crm_chat_message SET proposal_status = 'APPLYING', updated_at = {NOW} WHERE id = ? AND proposal_status = 'PROPOSED'"))).bind(message_id).execute(pool).await?;
    if taken.rows_affected() == 0 {
        return Err(invalid("This proposal was applied or dismissed already"));
    }
    let m = message(pool, message_id).await?;
    let agent_name: Option<(Option<String>,)> = sqlx::query_as("SELECT a.name FROM crm_chat_message m JOIN crm_chat_thread t ON t.id = m.thread_id LEFT JOIN crm_agent a ON a.id = t.agent_id WHERE m.id = ?").bind(message_id).fetch_optional(pool).await?;
    let actor = Actor::agent(&agent_name.and_then(|(n,)| n).unwrap_or_else(|| "Assistant".into()));
    match tools::apply(pool, &m.proposal.0, &actor).await {
        Ok(made) => {
            sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE crm_chat_message SET proposal_status = 'APPLIED', proposal_result = ?, updated_at = {NOW} WHERE id = ?"))).bind(made).bind(message_id).execute(pool).await?;
            message(pool, message_id).await
        }
        Err(e) => {
            sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE crm_chat_message SET proposal_status = 'PROPOSED', proposal_result = ?, updated_at = {NOW} WHERE id = ?"))).bind(format!("{e:#}")).bind(message_id).execute(pool).await?;
            Err(e)
        }
    }
}

pub async fn dismiss(pool: &SqlitePool, message_id: &str) -> Result<ChatMessage> {
    let done = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE crm_chat_message SET proposal_status = 'DISMISSED', updated_at = {NOW} WHERE id = ? AND proposal_status = 'PROPOSED'"))).bind(message_id).execute(pool).await?;
    if done.rows_affected() == 0 {
        return Err(invalid("This proposal was applied or dismissed already"));
    }
    message(pool, message_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::ai::ollama::ToolCall;
    use crate::kernel::hub::WorkspaceHub;
    use crate::kernel::records::Query;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    /// A model that answers from a script and keeps what it was shown.
    struct Scripted {
        replies: Mutex<VecDeque<ChatReply>>,
        seen: Mutex<Vec<Vec<Value>>>,
    }

    impl Scripted {
        fn new(replies: Vec<ChatReply>) -> Self {
            Self { replies: Mutex::new(replies.into()), seen: Mutex::new(Vec::new()) }
        }
    }

    impl Model for Scripted {
        fn reply(&self, messages: Vec<Value>, _tools: &Value) -> impl Future<Output = Result<ChatReply>> + Send {
            self.seen.lock().unwrap().push(messages);
            let next = self.replies.lock().unwrap().pop_front();
            async move { next.ok_or_else(|| anyhow!("no more replies")) }
        }

        fn name(&self) -> String {
            "scripted".into()
        }
    }

    fn call(name: &str, arguments: Value) -> ChatReply {
        ChatReply { content: String::new(), tool_calls: vec![ToolCall { name: name.into(), arguments }] }
    }

    fn say(text: &str) -> ChatReply {
        ChatReply { content: text.into(), tool_calls: Vec::new() }
    }

    async fn workspace() -> SqlitePool {
        let root = std::env::temp_dir().join(format!("newdawn-assistant-{}", uuid::Uuid::now_v7()));
        let hub = WorkspaceHub::open(root).await.unwrap();
        hub.create_workspace("VIFC", "test").await.unwrap();
        hub.pool("test").unwrap()
    }

    /// Plan 0.10.13's "done when": opportunities above 50k with no activity for 30 days come back
    /// with their links; "add a note to Keppel Land" is a proposal until Apply writes it, once.
    #[test]
    fn tools_find_quiet_deals_and_notes_wait_for_apply() {
        tauri::async_runtime::block_on(async {
            let pool = workspace().await;
            let actor = Actor::system("test");
            let opportunity = records::object(&pool, "opportunity").await.unwrap();
            let company = records::object(&pool, "company").await.unwrap();
            let keppel = query::create(&pool, &company, &json!({ "name": "Keppel Land" }), &actor).await.unwrap();
            let keppel_id = keppel["id"].as_str().unwrap().to_owned();
            let mut ids = HashMapLite::default();
            for (name, amount, stage) in [("Tower B lease", 120_000_i64, "PROPOSAL"), ("Parking deal", 20_000, "NEW"), ("Fit-out", 80_000, "MEETING")] {
                let made = query::create(&pool, &opportunity, &json!({ "name": name, "amount": { "amountMicros": amount * 1_000_000, "currencyCode": "USD" }, "stage": stage, "company": keppel_id }), &actor).await.unwrap();
                ids.0.push((name.to_owned(), made["id"].as_str().unwrap().to_owned()));
            }
            // Tower B and Parking: quiet since September; Fit-out was touched today.
            for name in ["Tower B lease", "Parking deal"] {
                sqlx::query("UPDATE timeline_activity SET happens_at = '2026-09-01T00:00:00.000Z' WHERE opportunity_id = ?").bind(ids.get(name)).execute(&pool).await.unwrap();
            }
            let all: Vec<String> = tools::ALL.iter().map(|t| (*t).to_owned()).collect();
            let run = |name: &str, args: Value| {
                let pool = pool.clone();
                let all = all.clone();
                let call = ToolCall { name: name.into(), arguments: args };
                async move { tools::run(&pool, &all, &call).await }
            };
            let found = run("find_records", json!({ "object": "opportunity", "rules": [{ "field": "amount", "op": "gt", "value": 50000 }], "inactive_days": 30 })).await;
            let records = found.result["records"].as_array().unwrap();
            assert_eq!(records.len(), 1, "{}", found.result);
            assert_eq!(records[0]["label"], "Tower B lease");
            assert_eq!(records[0]["link"], format!("/crm/opportunities/{}", ids.get("Tower B lease")));
            assert_eq!(records[0]["amount"], "120000 USD");
            assert_eq!(records[0]["stage"], "Proposal");
            assert_eq!(records[0]["lastActivity"], "2026-09-01");
            let by_label = run("count_records", json!({ "object": "opportunity", "rules": [{ "field": "stage", "op": "is", "value": "Meeting" }, { "field": "company", "op": "is", "value": "Keppel Land" }] })).await;
            assert_eq!(by_label.result["count"], 1, "a select by its label, a relation by its name: {}", by_label.result);
            let sum = run("aggregate", json!({ "object": "opportunity", "field": "amount", "op": "SUM" })).await;
            assert_eq!(sum.result["value"], 220000.0, "{}", sum.result);
            let wrong = run("find_records", json!({ "object": "opportunity", "rules": [{ "field": "size", "op": "gt", "value": 1 }] })).await;
            assert!(wrong.result["error"].as_str().unwrap().contains("no field “size”"), "a mistake is told back: {}", wrong.result);
            let refused = tools::run(&pool, &[String::from("find_records")], &ToolCall { name: "propose_note".into(), arguments: json!({}) }).await;
            assert!(refused.result["error"].is_string() && refused.proposal.is_none(), "a tool the agent may not use");

            // A note proposed through a turn: kept as a proposal; Apply writes it once.
            let thread = new_thread(&pool, None, "company", &keppel_id).await.unwrap();
            assert_eq!(thread.record_label, "Keppel Land");
            add(&pool, &thread.id, NewMessage { role: "user", content: "add a note to Keppel Land: call back about Tower B", tool_name: "", tool: None, proposal: None, model: "" }).await.unwrap();
            // The record named, not its id: the tools find it.
            let model = Scripted::new(vec![call("propose_note", json!({ "object": "company", "id": "Keppel Land", "title": "Call back about Tower B", "body": "Ask about the lease terms.\nThen the parking." })), say("I proposed the note — apply it to add it.")]);
            turn(&pool, &thread.id, &model).await.unwrap();
            let seen = model.seen.lock().unwrap().clone();
            let system = seen[0][0]["content"].as_str().unwrap().to_owned();
            assert!(system.contains("- opportunity (Opportunities): name (text), amount (amount)") && system.contains("This chat is about the company Keppel Land"), "{system}");
            let thread_messages = messages(&pool, &thread.id).await.unwrap();
            let roles: Vec<&str> = thread_messages.iter().map(|m| m.role.as_str()).collect();
            assert_eq!(roles, ["user", "assistant", "tool", "assistant"]);
            let proposal = &thread_messages[2];
            assert_eq!(proposal.proposal_status, "PROPOSED");
            assert_eq!(proposal.proposal.0["summary"], "Add a note to Keppel Land — “Call back about Tower B”");
            let notes = records::object(&pool, "note").await.unwrap();
            assert_eq!(query::list(&pool, &notes, &Query::default()).await.unwrap().total, 0, "nothing written before Apply");
            let applied = apply(&pool, &proposal.id).await.unwrap();
            assert_eq!(applied.proposal_status, "APPLIED");
            assert!(apply(&pool, &proposal.id).await.is_err(), "applied once");
            let made = query::get(&pool, &notes, &applied.proposal_result, false).await.unwrap();
            assert_eq!(made["title"], "Call back about Tower B");
            assert_eq!(made["createdBy"]["source"], "AGENT");
            assert!(made["body"].as_str().unwrap().contains("Then the parking."));
            let target: (String,) = sqlx::query_as("SELECT company_id FROM note_target WHERE note_id = ?").bind(&applied.proposal_result).fetch_one(&pool).await.unwrap();
            assert_eq!(target.0, keppel_id);

            // An update proposed and dismissed changes nothing.
            let changed = run("propose_update", json!({ "object": "opportunity", "id": ids.get("Fit-out"), "fields": { "stage": "Customer" } })).await;
            assert_eq!(changed.proposal.as_ref().unwrap()["fields"]["stage"], "CUSTOMER");
            add(&pool, &thread.id, NewMessage { role: "tool", content: "", tool_name: "propose_update", tool: Some(&changed.result), proposal: changed.proposal.as_ref(), model: "" }).await.unwrap();
            let pending = messages(&pool, &thread.id).await.unwrap().pop().unwrap();
            assert_eq!(dismiss(&pool, &pending.id).await.unwrap().proposal_status, "DISMISSED");
            assert!(apply(&pool, &pending.id).await.is_err());
            assert_eq!(query::get(&pool, &opportunity, ids.get("Fit-out"), false).await.unwrap()["stage"], "MEETING");

            // An amount changed by name keeps the record's currency; Apply never writes it twice.
            let amount = run("propose_update", json!({ "object": "opportunities", "id": "Tower B lease", "fields": { "amount": "150000" } })).await;
            let proposed = amount.proposal.clone().unwrap_or_else(|| panic!("{}", amount.result));
            assert_eq!(proposed["summary"], "Change Tower B lease — Amount: 150000 USD");
            assert_eq!(amount.result["record"]["link"], format!("/crm/opportunities/{}", ids.get("Tower B lease")), "the proposal gives the record's link");
            add(&pool, &thread.id, NewMessage { role: "tool", content: "", tool_name: "propose_update", tool: Some(&amount.result), proposal: Some(&proposed), model: "" }).await.unwrap();
            let pending = messages(&pool, &thread.id).await.unwrap().pop().unwrap();
            apply(&pool, &pending.id).await.unwrap();
            let tower = query::get(&pool, &opportunity, ids.get("Tower B lease"), false).await.unwrap();
            assert_eq!((tower["amount"]["amountMicros"].as_f64(), tower["amount"]["currencyCode"].as_str()), (Some(150_000_000_000.0), Some("USD")));
            let on_task = run("propose_note", json!({ "object": "task", "id": "x", "title": "No" })).await;
            assert!(on_task.result["error"].as_str().unwrap_or("").contains("can't be attached"), "{}", on_task.result);

            // The record deleted for good: the chat stays, detached.
            query::delete(&pool, &company, &keppel_id, &actor).await.unwrap();
            query::destroy(&pool, &company, &keppel_id).await.unwrap();
            let kept = threads(&pool, None).await.unwrap();
            assert_eq!((kept.len(), kept[0].record_object.as_str()), (1, ""));
        });
    }

    #[derive(Default)]
    struct HashMapLite(Vec<(String, String)>);

    impl HashMapLite {
        fn get(&self, name: &str) -> &str {
            &self.0.iter().find(|(n, _)| n == name).unwrap().1
        }
    }

    /// The default agent is made once; an agent's tools are what it may use; the default stays.
    #[test]
    fn agents_are_settings_with_a_default() {
        tauri::async_runtime::block_on(async {
            let pool = workspace().await;
            let (a, b) = futures_util::future::join(agents(&pool), agents(&pool)).await;
            assert_eq!((a.unwrap().len(), b.unwrap().len()), (1, 1), "two first asks make one default");
            let default = agent(&pool, None).await.unwrap();
            assert!(default.is_default && default.tools.len() == tools::ALL.len());
            let reader = save_agent(&pool, &AssistantAgentInput { id: None, name: "Reader".into(), description: String::new(), instructions: "Only look things up.".into(), model: String::new(), tools: vec!["find_records".into(), "nonsense".into()] }).await.unwrap();
            assert_eq!(reader.tools, ["find_records"]);
            assert!(delete_agent(&pool, &default.id).await.is_err());
            let thread = new_thread(&pool, Some(&reader.id), "", "").await.unwrap();
            delete_agent(&pool, &reader.id).await.unwrap();
            assert_eq!(threads(&pool, None).await.unwrap()[0].agent_name, "Assistant", "its chat goes on with the default: {}", thread.id);
        });
    }
}
