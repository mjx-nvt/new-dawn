//! The local API's docs page (D-071): one HTML page, no scripts — the endpoints, the query
//! syntax, each open workspace's objects and fields, Plane's paths (D-084), and curl examples on
//! this Mac's port.

use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records;

fn esc(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub async fn page(hub: &WorkspaceHub, port: u16) -> String {
    let base = format!("http://localhost:{port}");
    let mut objects = String::new();
    for ws in hub.list().await.unwrap_or_default().into_iter().filter(|w| !w.archived) {
        let Ok(pool) = hub.pool(&ws.slug) else { continue };
        objects.push_str(&format!("<h3>{} <small>(<code>{}</code> in the paths)</small></h3>", esc(&ws.name), esc(&ws.slug)));
        for meta in records::objects(&pool).await.unwrap_or_default().into_iter().filter(|m| m.is_active && !m.is_system) {
            let Ok(def) = records::object(&pool, &meta.name_singular).await else { continue };
            let fields: Vec<String> = def.fields.iter().filter(|f| f.is_active && !f.is_system).map(|f| format!("<code>{}</code> <small>{}</small>", esc(&f.name), esc(&f.r#type.as_str().to_lowercase()))).collect();
            objects.push_str(&format!("<p><b><code>/rest/{}</code></b> — {}<br>{}</p>", esc(&meta.name_plural), esc(&meta.label_plural), fields.join(", ")));
        }
    }
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><title>New Dawn — local API</title>
<style>body{{font:15px/1.6 -apple-system,system-ui,sans-serif;max-width:860px;margin:40px auto;padding:0 16px;color:#0f172a}}code,pre{{font-family:ui-monospace,Menlo,monospace;font-size:13px}}pre{{background:#f1f5f9;padding:12px;border-radius:8px;overflow:auto}}small{{color:#64748b}}h1{{font-size:28px}}@media(prefers-color-scheme:dark){{body{{background:#0f172a;color:#e2e8f0}}pre{{background:#1e293b}}}}</style></head><body>
<h1>New Dawn — local API</h1>
<p>Twenty's REST paths and JSON on this Mac only, at <code>{base}</code>. Every request sends an API key (Workspace → CRM → Developers, or Workspace → Projects → API tokens — one key opens both): <code>Authorization: Bearer nd_…</code>. The key names its workspace.</p>
<h2>Records</h2>
<pre>GET    /rest/&lt;objects&gt;?limit=60&amp;filter=…&amp;orderBy=…&amp;starting_after=…   → {{ "data": {{ "&lt;objects&gt;": [ … ] }}, "pageInfo": {{ "hasNextPage", "startCursor", "endCursor" }}, "totalCount" }}
GET    /rest/&lt;objects&gt;/&lt;id&gt;            → {{ "data": {{ "&lt;object&gt;": {{ … }} }} }}
POST   /rest/&lt;objects&gt;                 → {{ "data": {{ "create&lt;Object&gt;": {{ … }} }} }}
PATCH  /rest/&lt;objects&gt;/&lt;id&gt;            → {{ "data": {{ "update&lt;Object&gt;": {{ … }} }} }}
DELETE /rest/&lt;objects&gt;/&lt;id&gt;            → {{ "data": {{ "delete&lt;Object&gt;": {{ "id" }} }} }}   (to Deleted records)
POST   /rest/batch/&lt;objects&gt;           → {{ "data": {{ "create&lt;Objects&gt;": [ … ] }} }}   (60 at most)</pre>
<p><b>filter</b>: <code>field[op]:value</code>, joined by commas (all hold), grouped with <code>and(…)</code>, <code>or(…)</code>, <code>not(…)</code>; ops <code>eq neq in containsAny is gt gte lt lte startsWith like ilike</code> (<code>startsWith</code>, <code>like</code> and <code>ilike</code> match anywhere in the text, any case); <code>is</code> takes <code>NULL</code> or <code>NOT_NULL</code>; a composite's part as <code>name.firstName</code>, an amount as <code>amount.amountMicros</code>. <b>orderBy</b>: <code>field[AscNullsFirst]</code>, <code>field[DescNullsLast]</code>, … <b>limit</b>: 1–200 (60). A relation is written as <code>companyId</code> or <code>company</code>.</p>
<pre>curl -H "Authorization: Bearer $KEY" "{base}/rest/people?limit=2"
curl -H "Authorization: Bearer $KEY" "{base}/rest/opportunities?filter=amount.amountMicros%5Bgt%5D:50000000000&amp;orderBy=closeDate%5BAscNullsFirst%5D"
curl -X POST -H "Authorization: Bearer $KEY" -H "Content-Type: application/json" -d '{{"name":"Keppel Land"}}' "{base}/rest/companies"</pre>
<h2>Workflows</h2>
<p>A workflow with a webhook trigger: <code>POST {base}/webhooks/workflows/&lt;workspace&gt;/&lt;workflowId&gt;</code> with a JSON body — the body is the run's trigger data (<code>{{{{trigger.field}}}}</code>). No key; this Mac only.</p>
<h2>Webhooks</h2>
<p>Each record event (<code>person.created</code>, <code>opportunity.updated</code>, <code>company.deleted</code>, …) is posted to the webhook's address on this Mac as <code>{{ "event", "data", "timestamp" }}</code>, with <code>X-Twenty-Webhook-Timestamp</code> and <code>X-Twenty-Webhook-Signature</code> — the hex HMAC-SHA256 of <code>timestamp:body</code> with the webhook's secret.</p>
<h2>Projects — Plane's API</h2>
<p>Plane's v1 paths and JSON for the Projects module, under <code>{base}/api/v1/workspaces/&lt;workspace&gt;/</code>, with a key of that workspace sent as <code>X-API-Key: nd_…</code> — 60 requests a minute a key (<code>X-RateLimit-Remaining</code>, <code>X-RateLimit-Reset</code>). Lists come in Plane's pages: <code>per_page</code> (100 at most), <code>cursor</code> = <code>per_page:page:is_prev</code> as <code>next_cursor</code> and <code>prev_cursor</code> give it; <code>order_by</code> (a leading <code>-</code> for descending) and <code>fields</code>. <code>null</code> clears a field that can be empty (dates, parent, estimate, assignees, labels, a description); a priority is cleared with <code>"none"</code>. Colours are the app's names; you are <code>me</code>. With the Projects module off in a workspace, its paths answer 404.</p>
<pre>GET POST          projects/
GET PATCH DELETE  projects/&lt;id&gt;/                        POST DELETE  projects/&lt;id&gt;/archive/
GET POST          projects/&lt;id&gt;/issues/                  (or work-items/)
GET PATCH DELETE  projects/&lt;id&gt;/issues/&lt;id&gt;/
GET               issues/&lt;IDENTIFIER-N&gt;/              (or work-items/&lt;IDENTIFIER-N&gt;/)
GET POST          projects/&lt;id&gt;/states/  labels/  cycles/  modules/  intake-issues/
GET PATCH DELETE  projects/&lt;id&gt;/states/&lt;id&gt;/  (and the others' &lt;id&gt;/)
GET POST          projects/&lt;id&gt;/cycles/&lt;id&gt;/cycle-issues/      DELETE  …/cycle-issues/&lt;issue&gt;/
GET POST          projects/&lt;id&gt;/modules/&lt;id&gt;/module-issues/    DELETE  …/module-issues/&lt;issue&gt;/</pre>
<pre>curl -H "X-API-Key: $KEY" "{base}/api/v1/workspaces/&lt;workspace&gt;/projects/"
curl -X POST -H "X-API-Key: $KEY" -H "Content-Type: application/json" -d '{{"name":"Draft the rules","priority":"high"}}' "{base}/api/v1/workspaces/&lt;workspace&gt;/projects/&lt;id&gt;/issues/"</pre>
<p>Plane's webhooks (Workspace → Projects → Webhooks) get <code>{{ "event", "action", "webhook_id", "workspace_id", "data", "activity" }}</code> for projects, work items (<code>issue</code>), cycles, modules and comments (<code>issue_comment</code>), with <code>X-Plane-Event</code>, <code>X-Plane-Delivery</code> and <code>X-Plane-Signature</code> — the hex HMAC-SHA256 of the body with the webhook's secret.</p>
<h2>Objects</h2>
{objects}
</body></html>"#
    )
}
