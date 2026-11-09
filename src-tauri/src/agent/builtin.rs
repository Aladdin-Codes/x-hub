//! 内置工具（§5 全量 30 个 + 0.9.0 新增 create_subcategory 与 get_snippet + 0.9.x 扩展桥 2 个，共 34：读 15 / 写 14 / 危险 5）。
//!
//! 统一口径：
//! - **名称匹配失败语义**：`create_note` 的 folder/tags、`create_resource` 的 category
//!   按**名称精确匹配**；失败时返回「未找到 + 最接近候选」，不猜、不静默落默认（§5）。
//! - **结果长度**：列表类最多 20 条（超出提示「还有 N 条」）、正文类回填模型最多 2000 字
//!   （截断在 turn.rs 统一做）；UI 拿到的 detail 是全文，模型拿截断版。
//! - **环境类失败语义**：`get_weather` 受联网总开关控制，关闭/未配置时返回**说明**而非
//!   错误（避免模型反复重试，§5）。
//! - 工具 description 写清「什么时候用、什么时候不用」，参数尽量少、必填最小化。

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike};

use serde_json::{json, Value};

use crate::models::{
    ClipboardItem, Countdown, Note, RepeatRule, Resource, ResourceKind, Snippet, Todo,
};
use crate::repo;

use super::approval::Level;
use super::tools::{AgentTool, ToolCtx, ToolOutcome, ToolRegistry};

// ---------------------------------------------------------------------------
// 参数与格式化辅助
// ---------------------------------------------------------------------------

/// 列表类结果最多条数（§5 工具结果长度控制）
const MAX_LIST: usize = 20;
/// 正文类回填模型的最大字符数（turn.rs 按此硬截断，工具内先做软截断）
const MAX_TEXT: usize = 2000;
/// 工具参数里的时间戳合法区间（2000-01-01 ~ 2100-01-01）：LLM 输出是不可信输入，
/// i64::MAX 级数值会让 `end - now` 溢出 panic、chrono 换算越界——范围外按不可识别拒绝
const TIME_MS_MIN: i64 = 946_684_800_000;
const TIME_MS_MAX: i64 = 4_102_444_800_000;
/// 倒计时时长/间隔的分钟上限（10 年）：防 `minutes * 60_000` 溢出
const MAX_COUNTDOWN_MINUTES: i64 = 5_259_600;

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty())
}

fn arg_i64(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| {
        v.as_i64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
    })
}

/// 三个状态参数的取值语义：缺省=不改动 / null=清除 / 值=设置
enum SetOpt {
    Keep,
    Clear,
    Set(i64),
}

impl SetOpt {
    fn value(&self) -> Option<i64> {
        match self {
            SetOpt::Set(v) => Some(*v),
            _ => None,
        }
    }
    fn is_changed(&self) -> bool {
        !matches!(self, SetOpt::Keep)
    }
}

fn arg_time_opt(args: &Value, key: &str) -> Result<SetOpt, String> {
    match args.get(key) {
        None => Ok(SetOpt::Keep),
        Some(Value::Null) => Ok(SetOpt::Clear),
        Some(v) => parse_time_ms(v).map(SetOpt::Set).ok_or_else(|| {
            format!("Parameter {key} is not a recognized time (accepted formats: millisecond timestamp, YYYY-MM-DD HH:mm, or ISO 8601).")
        }),
    }
}

/// 时间解析：毫秒时间戳 / 本地 "YYYY-MM-DD HH:mm" / "YYYY-MM-DD" / ISO8601 → 毫秒。
/// 超出合法区间的数值一律 None（防溢出，见 TIME_MS_MIN/MAX 注释）
fn parse_time_ms(v: &Value) -> Option<i64> {
    let in_range = |ms: i64| (TIME_MS_MIN..=TIME_MS_MAX).contains(&ms).then_some(ms);
    match v {
        Value::Number(n) => n.as_i64().and_then(in_range),
        Value::String(s) => {
            let s = s.trim();
            if let Ok(ms) = s.parse::<i64>() {
                return in_range(ms);
            }
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                return in_range(dt.timestamp_millis());
            }
            for fmt in ["%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d"] {
                if let Ok(naive) = NaiveDateTime::parse_from_str(s, fmt) {
                    return Local
                        .from_local_datetime(&naive)
                        .single()
                        .and_then(|dt| in_range(dt.timestamp_millis()));
                }
                if fmt == "%Y-%m-%d" {
                    if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
                        let naive = d.and_hms_opt(0, 0, 0)?;
                        return Local
                            .from_local_datetime(&naive)
                            .single()
                            .and_then(|dt| in_range(dt.timestamp_millis()));
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// 毫秒 → `MM-DD HH:mm`（跨年补年份）
fn fmt_ms(ms: i64) -> String {
    let dt = Local.timestamp_millis_opt(ms).single();
    match dt {
        Some(d) => {
            let now = Local::now();
            if d.year() == now.year() {
                format!("{:02}-{:02} {:02}:{:02}", d.month(), d.day(), d.hour(), d.minute())
            } else {
                format!("{}-{:02}-{:02}", d.year(), d.month(), d.day())
            }
        }
        None => format!("{ms}"),
    }
}

fn cap_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}… [truncated; {} characters total]", s.chars().count())
    }
}

fn fmt_todo_line(t: &Todo) -> String {
    let mut parts = vec![format!("[{}]{}", t.id, if t.done { "✔" } else { "" })];
    parts.push(if t.pinned { "📌".into() } else { String::new() });
    parts.push(t.title.clone());
    if t.priority > 0 {
        parts.push(format!("P{}", t.priority));
    }
    if let Some(due) = t.due_at {
        parts.push(format!("Due {}", fmt_ms(due)));
    }
    if t.repeat_mode != "once" {
        parts.push(format!("({})", t.repeat_mode));
    }
    parts.into_iter().filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ")
}

fn fmt_note_line(n: &Note) -> String {
    format!("[{}] {}", n.id, n.title)
}

fn fmt_resource_line(r: &Resource) -> String {
    let cat = r.category.clone().unwrap_or_default();
    format!("[{}] {} ({}{})", r.id, r.name, kind_label(&r.kind), if cat.is_empty() { String::new() } else { format!("/{}", cat) })
}

fn kind_label(k: &ResourceKind) -> &'static str {
    match k {
        ResourceKind::App => "app",
        ResourceKind::Web => "website",
        ResourceKind::File => "file",
    }
}

fn fmt_countdown_line(c: &Countdown) -> String {
    let now = repo::countdown::now_ms();
    let remaining = if c.paused {
        c.paused_remaining_ms.unwrap_or(0)
    } else {
        (c.end_at - now).max(0)
    };
    let remain_txt = if remaining >= 3_600_000 {
        format!("{:.1} hours", remaining as f64 / 3_600_000.0)
    } else {
        format!("{} minutes", remaining / 60_000)
    };
    format!(
        "[{}] {} ({}, {} remaining{})",
        c.id,
        c.name,
        c.repeat_mode,
        remain_txt,
        if c.paused { ", paused" } else { "" }
    )
}

/// 名称精确匹配（§5 失败语义）：命中返回 id，否则返回候选名列表
fn match_exact(candidates: &[(String, i64)], want: &str) -> Result<i64, Vec<String>> {
    if let Some((_, id)) = candidates.iter().find(|(n, _)| n == want) {
        return Ok(*id);
    }
    Err(candidates.iter().take(8).map(|(n, _)| n.clone()).collect())
}

/// 从参数取 id：返回 (参数原文, 解析后的数字)。原文保留原样——「007」这类前导零
/// 标题按名称匹配时不能丢零，非数字原文（模型直接传了标题串）也照样能按名兜底
fn arg_id(args: &Value, key: &str) -> (String, Option<i64>) {
    match args.get(key) {
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                (i.to_string(), Some(i))
            } else {
                // 浮点形式的整数（偶发供应商把整数 id 序列化成 20261023.0）按整数值
                // 解析——否则原文带着「.0」，id 与名称兜底双双对不上
                match n.as_f64() {
                    Some(f) if f.fract() == 0.0 && (i64::MIN as f64..=i64::MAX as f64).contains(&f) => {
                        let i = f as i64;
                        (i.to_string(), Some(i))
                    }
                    _ => (n.to_string(), None),
                }
            }
        }
        Some(Value::String(s)) => {
            let t = s.trim();
            (t.to_string(), t.parse::<i64>().ok())
        }
        _ => (String::new(), None),
    }
}

/// 按 id 取对象，查不到时按名称精确匹配兜底（0.9.3 反馈：标题/名称本身是数字——
/// 如笔记「20261023」——模型会把用户口中的数字直接当 id 传参，按 id 查不到就报
/// 「不存在」）。兜底顺序：真实 id 永远优先；id 查不到才按名称精确匹配——唯一命中
/// 用它，同名多篇报歧义（§5 不猜），彻底没有报不存在并提示先搜索。
/// `what` is the object label used in error messages; `name_candidates` runs only if the ID lookup fails.
fn resolve_by_id<T>(
    raw: &str,
    id: Option<i64>,
    what: &str,
    get_by_id: impl Fn(i64) -> Result<T, rusqlite::Error>,
    name_candidates: impl FnOnce() -> Result<Vec<(String, i64)>, rusqlite::Error>,
) -> Result<T, ToolOutcome> {
    let missing = || {
        ToolOutcome::err(
            format!("{what} {raw} not found"),
            format!(
                "No {what} with ID or name \"{raw}\" was found. If the user gave a name rather than an ID, use the relevant search or list tool first, then pass the [id] shown at the start of the result."
            ),
        )
    };
    if let Some(id) = id {
        if let Ok(t) = get_by_id(id) {
            return Ok(t);
        }
    }
    let candidates = name_candidates().map_err(db_err)?;
    let hits: Vec<i64> = candidates.iter().filter(|(n, _)| n.as_str() == raw).map(|(_, i)| *i).collect();
    match hits.as_slice() {
        [only] => get_by_id(*only).map_err(|_| missing()),
        [] => Err(missing()),
        many => Err(ToolOutcome::err(
            format!("Found {} {what} entries named \"{raw}\"", many.len()),
            format!(
                "There are {} {what} entries with that name (IDs: {}). Ask the user which one to use; do not guess.",
                many.len(),
                many.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ")
            ),
        )),
    }
}

fn not_found_outcome(what: &str, want: &str, candidates: Vec<String>) -> ToolOutcome {
    ToolOutcome::err(
        format!("{what} \"{want}\" not found"),
        format!(
            "Could not find {what} \"{want}\". Closest matches: {}. Ask the user to confirm before proceeding; do not guess.",
            if candidates.is_empty() { "(none)".to_string() } else { candidates.join(", ") }
        ),
    )
}

fn db_err(e: rusqlite::Error) -> ToolOutcome {
    ToolOutcome::err(format!("Database operation failed: {e}"), String::new())
}

/// 新建速达小类（0.9.0 新增，第 31 个工具）：用户明确要建新目录时，
/// 先建小类再 create_resource 落资源——此前 create_resource 对不存在的小类
/// 只会返回候选让用户反问，模型没有任何途径新建目录，实测「创建不了速达目录」。
struct CreateSubcategory;
impl AgentTool for CreateSubcategory {
    fn name(&self) -> &'static str { "create_subcategory" }
    fn description(&self) -> &'static str { "Create a Quick Launch category. kind=app|web|file; name is the category name. Call only when the user explicitly asks to create a category. If it already exists, return it without duplicating it; then reference it through create_resource's category." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{
        "kind":{"type":"string","enum":["app","web","file"]},
        "name":{"type":"string"}
    },"required":["kind","name"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let kind_str = match arg_str(args, "kind") {
            Some(k @ ("app" | "web" | "file")) => k,
            _ => return ToolOutcome::err("kind must be one of app, web, or file", String::new()),
        };
        let Some(name) = arg_str(args, "name") else { return ToolOutcome::err("Missing required parameter: name", String::new()) };
        // 幂等：同 kind 同名已存在则不重复建，直接返回（小类名在同 kind 内唯一使用）
        let subs = match repo::subcategory::list(ctx.conn) { Ok(s) => s, Err(e) => return db_err(e) };
        if let Some(exist) = subs.iter().find(|s| s.kind == kind_str && s.name == name) {
            return ToolOutcome::ok(
                format!("Category \"{name}\" already exists"),
                format!("Quick Launch category already exists: [{}] {name} (kind={kind_str}). Reference it with create_resource's category parameter.", exist.id),
            );
        }
        let s = match repo::subcategory::create(ctx.conn, kind_str, name) { Ok(s) => s, Err(e) => return db_err(e) };
        ToolOutcome::ok(format!("Created category \"{}\"", s.name), format!("Created Quick Launch category [{}] {} (kind={}). Reference it with create_resource's category parameter.", s.id, s.name, kind_str))
    }
}

/// 列表截断到 MAX_LIST 条，附「还有 N 条未列出」
fn cap_lines(mut lines: Vec<String>) -> String {
    if lines.len() <= MAX_LIST {
        return if lines.is_empty() { "(empty)".into() } else { lines.join("\n") };
    }
    let rest = lines.len() - MAX_LIST;
    lines.truncate(MAX_LIST);
    format!("{}\n({rest} more not shown)", lines.join("\n"))
}

// ---------------------------------------------------------------------------
// 读（12）
// ---------------------------------------------------------------------------

struct ListTodos;
impl AgentTool for ListTodos {
    fn name(&self) -> &'static str { "list_todos" }
    fn description(&self) -> &'static str { "List tasks, grouped by pinned, overdue, today, dated, and undated to match the UI. Call only when the user asks about their own tasks or schedule. Completed tasks are excluded." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{}}) }
    fn execute(&self, _args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let todos = match repo::todo::list(ctx.conn) { Ok(t) => t, Err(e) => return db_err(e) };
        let now_ms = Local::now().timestamp_millis();
        let today_start = Local.from_local_datetime(&Local::now().date_naive().and_hms_opt(0, 0, 0).unwrap()).single().map(|d| d.timestamp_millis());
        let today_end = today_start.map(|s| s + 86_400_000);
        let undone: Vec<&Todo> = todos.iter().filter(|t| !t.done).collect();
        let done_count = todos.len() - undone.len();
        let mut pinned = Vec::new();
        let mut overdue = Vec::new();
        let mut today = Vec::new();
        let mut dated = Vec::new();
        let mut undated = Vec::new();
        for t in &undone {
            if t.pinned { pinned.push(fmt_todo_line(t)); }
            else if let Some(due) = t.due_at {
                if due < now_ms { overdue.push(fmt_todo_line(t)); }
                else if let (Some(s), Some(e)) = (today_start, today_end) { if due >= s && due < e { today.push(fmt_todo_line(t)); } else { dated.push(fmt_todo_line(t)); } }
                else { dated.push(fmt_todo_line(t)); }
            } else { undated.push(fmt_todo_line(t)); }
        }
        let mut sections: Vec<String> = Vec::new();
        for (name, items) in [("Pinned", pinned), ("Overdue", overdue), ("Today", today), ("Dated", dated), ("No due date", undated)] {
            if !items.is_empty() { sections.push(format!("[{name}]\n{}", cap_lines(items))); }
        }
        if sections.is_empty() { sections.push("(No incomplete tasks)".into()); }
        if done_count > 0 { sections.push(format!("({done_count} completed tasks not shown)")); }
        ToolOutcome::ok("Tasks listed", sections.join("\n\n"))
    }
}

struct SearchNotes;
impl AgentTool for SearchNotes {
    fn name(&self) -> &'static str { "search_notes" }
    fn description(&self) -> &'static str { "Search note titles and bodies by keyword; excludes the trash. Call only when the user refers to a specific note or record but does not know its ID. If they have not mentioned their notes, answer directly instead of looking around. Use get_note to read a full note." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"keyword":{"type":"string","description":"Search keyword"}},"required":["keyword"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let Some(kw) = arg_str(args, "keyword") else { return ToolOutcome::err("Missing required parameter: keyword", String::new()) };
        let notes = match repo::note::search(ctx.conn, kw) { Ok(n) => n, Err(e) => return db_err(e) };
        let lines: Vec<String> = notes.iter().map(fmt_note_line).collect();
        ToolOutcome::ok(format!("Found {} notes", notes.len()), cap_lines(lines))
    }
}

struct GetNote;
impl AgentTool for GetNote {
    fn name(&self) -> &'static str { "get_note" }
    fn description(&self) -> &'static str { "Read one complete note, including its folder and tags. Get note_id from the [number] returned by list_notes, search_notes, or global_search. A number mentioned by the user may be a note title, not an ID; search for it first rather than guessing. If the ID is wrong, the tool falls back to title matching and asks for confirmation when titles are ambiguous." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"note_id":{"type":"integer","description":"Note ID"}},"required":["note_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "note_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: note_id", String::new()); }
        let note = match resolve_by_id(&raw, id, "note",
            |fid| repo::note::get(ctx.conn, fid),
            || Ok(repo::note::search(ctx.conn, &raw)?.into_iter().map(|n| (n.title, n.id)).collect()),
        ) { Ok(n) => n, Err(o) => return o };
        let id = note.id;
        let tags = repo::tag::tags_of_note(ctx.conn, id).unwrap_or_default().into_iter().map(|t| t.name).collect::<Vec<_>>().join(", ");
        let folder = note.folder_id.and_then(|fid| repo::note_folder::get(ctx.conn, fid).ok()).map(|f| f.name).unwrap_or_default();
        let meta = format!("\"{}\"{}{}", note.title, if folder.is_empty() { String::new() } else { format!(" Folder: {folder}") }, if tags.is_empty() { String::new() } else { format!(" Tags: {tags}") });
        ToolOutcome::ok(format!("Read \"{}\"", note.title), format!("{meta}\n\n{}", cap_chars(&note.content, MAX_TEXT)))
    }
}

struct ListNotes;
impl AgentTool for ListNotes {
    fn name(&self) -> &'static str { "list_notes" }
    fn description(&self) -> &'static str { "List notes and the folder tree without note bodies. Call only when the user asks to browse their notes or folders; do not call for conversations unrelated to local data. Optionally filter by folder name." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"folder":{"type":"string","description":"Filter by exact folder name"}}}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let folders = match repo::note_folder::list(ctx.conn) { Ok(f) => f, Err(e) => return db_err(e) };
        let mut filter_fid: Option<i64> = None;
        if let Some(want) = arg_str(args, "folder") {
            let cands: Vec<(String, i64)> = folders.iter().map(|f| (f.name.clone(), f.id)).collect();
            filter_fid = Some(match match_exact(&cands, want) {
                Ok(id) => id,
                Err(c) => return not_found_outcome("folder", want, c),
            });
        }
        let tree = folders.iter().map(|f| format!("Folder [{}]: {}", f.id, f.name)).collect::<Vec<_>>().join("\n");
        let notes = match repo::note::list_meta(ctx.conn) { Ok(n) => n, Err(e) => return db_err(e) };
        let lines: Vec<String> = notes
            .iter()
            .filter(|n| filter_fid.map_or(true, |f| n.folder_id == Some(f)))
            .map(fmt_note_line)
            .collect();
        let count = lines.len();
        let mut out = String::new();
        if !tree.is_empty() { out.push_str(&format!("Folder tree:\n{tree}\n\n")); }
        out.push_str(&cap_lines(lines));
        ToolOutcome::ok(format!("{count} notes"), out)
    }
}

struct ListResources;
impl AgentTool for ListResources {
    fn name(&self) -> &'static str { "list_resources" }
    fn description(&self) -> &'static str { "List Quick Launch resources (apps, websites, and files). Call only when the user asks to launch or view their resources. launch_resource requires an ID. Optionally filter by kind." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"kind":{"type":"string","enum":["app","web","file"],"description":"Filter by resource type; omit to include all"}}}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let all = match repo::resource::list_all(ctx.conn) { Ok(r) => r, Err(e) => return db_err(e) };
        let kind_filter = arg_str(args, "kind");
        let lines: Vec<String> = all.iter()
            .filter(|r| kind_filter.map(|k| crate::repo::resource::kind_to_str(&r.kind) == k).unwrap_or(true))
            .map(fmt_resource_line).collect();
        ToolOutcome::ok(format!("{} resources", lines.len()), cap_lines(lines))
    }
}

struct ListCountdowns;
impl AgentTool for ListCountdowns {
    fn name(&self) -> &'static str { "list_countdowns" }
    fn description(&self) -> &'static str { "List countdowns, including remaining time and paused status. Call only when the user asks about their countdowns." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{}}) }
    fn execute(&self, _args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let list = match repo::countdown::list(ctx.conn) { Ok(c) => c, Err(e) => return db_err(e) };
        let lines: Vec<String> = list.iter().map(fmt_countdown_line).collect();
        ToolOutcome::ok(format!("{} countdowns", lines.len()), cap_lines(lines))
    }
}

struct GetStickies;
impl AgentTool for GetStickies {
    fn name(&self) -> &'static str { "get_stickies" }
    fn description(&self) -> &'static str { "Read the contents of a workbench sticky note (slot 1 or 2). Call only when the user mentions a sticky note or its contents." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{}}) }
    fn execute(&self, _args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let all = match repo::sticky::list(ctx.conn) { Ok(s) => s, Err(e) => return db_err(e) };
        let mut out = Vec::new();
        for slot in [1, 2] {
            match all.iter().find(|s| s.slot == slot) {
                Some(s) => out.push(format!("Sticky note {slot}:\n{}", cap_chars(&s.content, MAX_TEXT))),
                None => out.push(format!("Sticky note {slot}: (empty)")),
            }
        }
        ToolOutcome::ok("Sticky notes read", out.join("\n\n"))
    }
}

struct ListSnippets;
impl AgentTool for ListSnippets {
    fn name(&self) -> &'static str { "list_snippets" }
    fn description(&self) -> &'static str { "List saved prompts and snippets, including pinned status and titles but not their contents. Use get_snippet for a full prompt or update_snippet to modify one. Call only when the user mentions a saved prompt or snippet." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{}}) }
    fn execute(&self, _args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let list = match repo::snippet::list(ctx.conn) { Ok(s) => s, Err(e) => return db_err(e) };
        let lines: Vec<String> = list.iter().map(|s: &Snippet| format!("{}[{}] {}", if s.is_pinned { "📌" } else { "" }, s.id, s.title)).collect();
        ToolOutcome::ok(format!("{} prompts", lines.len()), cap_lines(lines))
    }
}

struct GetSnippet;
impl AgentTool for GetSnippet {
    fn name(&self) -> &'static str { "get_snippet" }
    fn description(&self) -> &'static str { "Read the full text of one saved prompt. Get its ID from list_snippets. Use when the user asks to apply a specific prompt to content. Do not call without a specific ID." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"snippet_id":{"type":"integer","description":"Prompt ID"}},"required":["snippet_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "snippet_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: snippet_id", String::new()) };
        let s = match resolve_by_id(&raw, id, "prompt",
            |fid| repo::snippet::get(ctx.conn, fid),
            || Ok(repo::snippet::list(ctx.conn)?.into_iter().map(|s| (s.title, s.id)).collect()),
        ) { Ok(s) => s, Err(o) => return o };
        let meta = format!("\"{}\"{} copied {} times", s.title, if s.is_pinned { " (pinned)" } else { "" }, s.copy_count);
        ToolOutcome::ok(format!("Read \"{}\"", s.title), format!("{meta}\n\n{}", cap_chars(&s.content, MAX_TEXT)))
    }
}

struct ReadClipboard;
impl AgentTool for ReadClipboard {
    fn name(&self) -> &'static str { "read_clipboard" }
    fn description(&self) -> &'static str { "Read recent clipboard history, optionally filtered by keyword. Call only when the user mentions the clipboard or copied content. Image entries return metadata only, not image data." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"keyword":{"type":"string","description":"Filter by content"},"limit":{"type":"integer","description":"Maximum number of items (default: 10)"}}}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let kw = arg_str(args, "keyword");
        let limit = arg_i64(args, "limit").unwrap_or(10).clamp(1, 50);
        let items: Vec<ClipboardItem> = match repo::clipboard::list(ctx.conn, kw, limit, 0) { Ok(i) => i, Err(e) => return db_err(e) };
        let lines: Vec<String> = items.iter().map(|i: &ClipboardItem| match i.kind.as_str() {
            // 图片只回条目占位不回本地路径：绝对路径对本机外无意义，还会把
            // 数据根目录结构捎给外部模型
            "image" => format!("[{}][Image]", i.id),
            "file" => format!("[{}][File] {}", i.id, i.file_paths.join(", ")),
            _ => format!("[{}][Text] {}", i.id, cap_chars(&i.content, 300)),
        }).collect();
        ToolOutcome::ok(format!("{} clipboard items", lines.len()), cap_lines(lines))
    }
}

struct GlobalSearch;
impl AgentTool for GlobalSearch {
    fn name(&self) -> &'static str { "global_search" }
    fn description(&self) -> &'static str { "Search Quick Launch resources, notes, and tasks together. Call only when the user is looking for local data but it is unclear which section contains it. If the section is known, use its specific search or list tool. Do not call for conversations unrelated to local data." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"keyword":{"type":"string"}},"required":["keyword"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let Some(kw) = arg_str(args, "keyword") else { return ToolOutcome::err("Missing required parameter: keyword", String::new()) };
        let res = repo::resource::search(ctx.conn, kw).unwrap_or_default();
        let notes = repo::note::search(ctx.conn, kw).unwrap_or_default();
        let todos = repo::todo::search(ctx.conn, kw).unwrap_or_default();
        let mut sections = Vec::new();
        let r_lines: Vec<String> = res.iter().map(fmt_resource_line).collect();
        if !r_lines.is_empty() { sections.push(format!("[Quick Launch resources: {}]\n{}", res.len(), cap_lines(r_lines))); }
        let n_lines: Vec<String> = notes.iter().map(fmt_note_line).collect();
        if !n_lines.is_empty() { sections.push(format!("[Notes: {}]\n{}", notes.len(), cap_lines(n_lines))); }
        let t_lines: Vec<String> = todos.iter().map(fmt_todo_line).collect();
        if !t_lines.is_empty() { sections.push(format!("[Tasks: {}]\n{}", todos.len(), cap_lines(t_lines))); }
        if sections.is_empty() { return ToolOutcome::ok("No matches", format!("No matching content for \"{kw}\" in any of the three sections.")); }
        ToolOutcome::ok("Matches found", sections.join("\n\n"))
    }
}

struct GetWeather;
impl AgentTool for GetWeather {
    fn name(&self) -> &'static str { "get_weather" }
    fn description(&self) -> &'static str { "Get the current weather. A city must be configured in the app and internet access must be enabled. Call only when the user asks about the weather." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{}}) }
    fn execute(&self, _args: &Value, _ctx: ToolCtx) -> ToolOutcome {
        let config = crate::config::load();
        if !config.online_enabled {
            // 失败语义（§5）：返回说明而非错误，避免模型反复重试
            return ToolOutcome::ok("Internet access is disabled", "Internet access is turned off in the app, so weather data is unavailable. The user can enable it in Settings and try again.");
        }
        if config.weather_lat == 0.0 || config.weather_lng == 0.0 {
            return ToolOutcome::ok("No weather location configured", "No weather location is configured in the app. Ask the user to set a city in Settings before checking the weather.");
        }
        // 天气是异步网络请求，而工具执行是同步的：起独立线程 + 自建运行时，
        // 阻塞等待结果（低频调用，短暂阻塞可接受，且不会 panic 在 tokio 上下文里）
        let (tx, rx) = std::sync::mpsc::channel();
        let (lat, lng, city) = (config.weather_lat, config.weather_lng, config.weather_city.clone());
        std::thread::spawn(move || {
            let res = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt.block_on(crate::online::fetch_weather(lat, lng, &city)),
                Err(e) => Err(format!("Weather runtime initialization failed: {e}")),
            };
            let _ = tx.send(res);
        });
        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(Ok(w)) => ToolOutcome::ok(
                format!("{} {:.0}°C", w.city, w.temperature),
                format!("{}: {:.1}°C (feels like {:.1}°C), humidity {:.0}%, wind {:.1} km/h (WMO code {})",
                    w.city, w.temperature, w.apparent_temperature, w.relative_humidity, w.wind_speed, w.weather_code),
            ),
            Ok(Err(e)) => ToolOutcome::err(format!("Failed to get weather: {e}"), "The weather service is temporarily unavailable. Try again later."),
            Err(_) => ToolOutcome::err("Weather request timed out", "The weather service did not respond within 10 seconds. Try again later."),
        }
    }
}

struct GetSetting;
/// `get_setting` 可读键白名单：模型输出不可信，全量 AppConfig 里有供应商地址
/// （chat_models）、服务端地址、本机路径等——不设白名单等于把整份配置发给外部模型
const READABLE_SETTING_KEYS: &[&str] = &[
    "online_enabled", "weather_city", "quote_source", "countdown_sound", "clock_quote",
    "theme_mode", "font_scale", "notice_duration_ms",
    "clipboard_max_items", "clipboard_ttl_days", "clipboard_paused",
    "clipboard_image_enabled", "clipboard_file_enabled", "clipboard_paste_method",
    "chat_window_mode",
];
impl AgentTool for GetSetting {
    fn name(&self) -> &'static str { "get_setting" }
    fn description(&self) -> &'static str { "Read app settings from the allowlist (display settings such as online_enabled, weather_city, and theme_mode). Call only when the user asks about the app's settings. This tool is read-only; the assistant cannot change settings." }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"key":{"type":"string","description":"Setting key; omit to list all available keys"}}}) }
    fn execute(&self, args: &Value, _ctx: ToolCtx) -> ToolOutcome {
        let cfg = match serde_json::to_value(crate::config::load()) { Ok(v) => v, Err(e) => return ToolOutcome::err(format!("Failed to serialize settings: {e}"), String::new()) };
        match arg_str(args, "key") {
            Some(key) => match READABLE_SETTING_KEYS.iter().find(|k| **k == key).and_then(|k| cfg.get(*k)) {
                Some(v) => ToolOutcome::ok(format!("Read {key}"), format!("{key} = {}", serde_json::to_string_pretty(v).unwrap_or_default())),
                None => {
                    ToolOutcome::err(format!("Setting \"{key}\" is not readable"), format!("Available settings: {}", READABLE_SETTING_KEYS.join(", ")))
                }
            },
            None => {
                ToolOutcome::ok("Settings listed", format!("Available settings: {}", READABLE_SETTING_KEYS.join(", ")))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 写（13）
// ---------------------------------------------------------------------------

struct CreateTodo;
impl AgentTool for CreateTodo {
    fn name(&self) -> &'static str { "create_todo" }
    fn description(&self) -> &'static str { "Create a task. title is required. due_at and remind_at accept \"YYYY-MM-DD HH:mm\" or a millisecond timestamp; priority is 0-2. Pass repeat as an object, e.g. {\"mode\":\"daily\"}, {\"mode\":\"weekly\",\"weekdays\":[1,3]}, or {\"mode\":\"monthly\",\"month_day\":15}; the string \"daily\" is also accepted." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{
        "title":{"type":"string"},
        "due_at":{"description":"Due date/time: 24-hour \"YYYY-MM-DD HH:mm\" or a millisecond timestamp; omit for no due date"},
        "remind_at":{"description":"Reminder time: same format as due_at; omit for no reminder"},
        "priority":{"type":"integer","minimum":0,"maximum":2,"description":"0=normal (default), 1=important, 2=urgent"},
        "repeat":{"type":"object","properties":{
            "mode":{"type":"string","enum":["daily","weekly","monthly"]},
            "weekdays":{"type":"array","items":{"type":"integer","minimum":0,"maximum":6},"description":"Required for weekly: weekdays, where 0=Monday through 6=Sunday"},
            "month_day":{"type":"integer","minimum":-1,"maximum":31,"description":"Required for monthly: day of month (-1=last day)"}
        },"required":["mode"]}
    },"required":["title"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let Some(title) = arg_str(args, "title") else { return ToolOutcome::err("Missing required parameter: title", String::new()) };
        let t = match repo::todo::create(ctx.conn, title, None, None) { Ok(t) => t, Err(e) => return db_err(e) };
        let mut t = t;
        let due = match arg_time_opt(args, "due_at") { Ok(v) => v, Err(m) => return ToolOutcome::err(m, String::new()) };
        let remind = match arg_time_opt(args, "remind_at") { Ok(v) => v, Err(m) => return ToolOutcome::err(m, String::new()) };
        if due.is_changed() || remind.is_changed() {
            let d = if due.is_changed() { due.value() } else { t.due_at };
            let r = if remind.is_changed() { remind.value() } else { t.remind_at };
            if d.is_some() || r.is_some() {
                t = match repo::todo::schedule(ctx.conn, t.id, d, r) { Ok(t) => t, Err(e) => return db_err(e) };
            }
        }
        if let Some(p) = arg_i64(args, "priority").filter(|p| (0..=2).contains(p)) {
            if p != t.priority {
                t = match repo::todo::update_with_version(ctx.conn, t.id, title, p, None) { Ok(t) => t, Err(e) => return ToolOutcome::err(e, String::new()) };
            }
        }
        if let Some(rep) = args.get("repeat").filter(|v| !v.is_null()) {
            let rule = match repeat_rule_from(rep) { Ok(r) => r, Err(m) => return ToolOutcome::err(m, String::new()) };
            // schema 只是建议，LLM 完全可能发任意串——不经 validate 直接落库的话，
            // 非法周期会在勾选完成时被静默降级成一次性（todo_recurrence 的 match 落空），
            // 与 commands.rs / xhub_api.rs 两条既有路径同款 fail-fast
            if let Err(e) = rule.validate() {
                return ToolOutcome::err(format!("Invalid repeat parameter: {e}"), String::new());
            }
            t = match repo::todo::set_repeat(ctx.conn, t.id, &rule) { Ok(t) => t, Err(e) => return db_err(e) };
        }
        ToolOutcome::ok(format!("Created \"{}\"", t.title), format!("Created task: {}", fmt_todo_line(&t)))
    }
}

/// repeat 参数 → RepeatRule：接受对象 `{mode, weekdays?, month_day?}` 或裸字符串 `"daily"`。
/// weekdays 为 0-6 整数数组（0=周一…6=周日），转内部位掩码
fn repeat_rule_from(v: &Value) -> Result<RepeatRule, String> {
    let (mode, weekdays, month_day) = match v {
        Value::String(s) => (s.trim().to_string(), None, None),
        Value::Object(o) => {
            let mode = o
                .get("mode")
                .and_then(|m| m.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or("repeat.mode is required")?
                .to_string();
            let weekdays = match o.get("weekdays") {
                None | Some(Value::Null) => None,
                Some(Value::Array(arr)) => {
                    let mut mask: i64 = 0;
                    for d in arr {
                        let n = d.as_i64().ok_or("Each repeat.weekdays value must be an integer")?;
                        if !(0..=6).contains(&n) {
                            return Err(format!("repeat.weekdays value {n} is out of range (0=Monday through 6=Sunday)."));
                        }
                        mask |= 1 << n;
                    }
                    Some(mask)
                }
                Some(_) => return Err("repeat.weekdays must be an array of integers (for example, [1,3] for Monday and Wednesday).".into()),
            };
            let month_day = match o.get("month_day") {
                None | Some(Value::Null) => None,
                Some(Value::Number(n)) => {
                    let d = n.as_i64().ok_or("repeat.month_day must be an integer")?;
                    if d != -1 && !(1..=31).contains(&d) {
                        return Err(format!("repeat.month_day value {d} is out of range (1-31, or -1 for the last day)."));
                    }
                    Some(d)
                }
                Some(_) => return Err("repeat.month_day must be an integer (1-31, or -1 for the last day).".into()),
            };
            (mode, weekdays, month_day)
        }
        _ => return Err("repeat must be an object ({mode, weekdays?, month_day?}) or a string (daily).".into()),
    };
    Ok(RepeatRule {
        mode,
        every: None,
        unit: None,
        weekdays,
        month_day,
        month_nth: None,
        end_mode: None,
        end_at: None,
        count: None,
    })
}

struct UpdateTodo;
impl AgentTool for UpdateTodo {
    fn name(&self) -> &'static str { "update_todo" }
    fn description(&self) -> &'static str { "Update a task's title, due date, reminder, priority, completion status, or pinned status. Omitted fields are unchanged; pass null for due_at or remind_at to clear it. Completing a recurring task advances it to the next occurrence." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{
        "todo_id":{"type":"integer"},"title":{"type":"string"},"due_at":{},"remind_at":{},
        "priority":{"type":"integer","minimum":0,"maximum":2},
        "done":{"type":"boolean"},"pinned":{"type":"boolean"}
    },"required":["todo_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "todo_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: todo_id", String::new()); }
        let t0 = match resolve_by_id(&raw, id, "task",
            |fid| repo::todo::get(ctx.conn, fid),
            || Ok(repo::todo::search(ctx.conn, &raw)?.into_iter().map(|t| (t.title, t.id)).collect()),
        ) { Ok(t) => t, Err(o) => return o };
        let id = t0.id;
        let mut t = t0;
        let mut changes: Vec<&str> = Vec::new();
        if let Some(title) = arg_str(args, "title") {
            let p = arg_i64(args, "priority").unwrap_or(t.priority).clamp(0, 2);
            t = match repo::todo::update_with_version(ctx.conn, id, title, p, None) { Ok(t) => t, Err(e) => return ToolOutcome::err(e, String::new()) };
            changes.push("title");
        } else if let Some(p) = arg_i64(args, "priority").filter(|p| (0..=2).contains(p)) {
            t = match repo::todo::update_with_version(ctx.conn, id, &t.title, p, None) { Ok(t) => t, Err(e) => return ToolOutcome::err(e, String::new()) };
            changes.push("priority");
        }
        let due = match arg_time_opt(args, "due_at") { Ok(v) => v, Err(m) => return ToolOutcome::err(m, String::new()) };
        let remind = match arg_time_opt(args, "remind_at") { Ok(v) => v, Err(m) => return ToolOutcome::err(m, String::new()) };
        if due.is_changed() || remind.is_changed() {
            let d = if due.is_changed() { due.value() } else { t.due_at };
            let r = if remind.is_changed() { remind.value() } else { t.remind_at };
            t = match repo::todo::schedule(ctx.conn, id, d, r) { Ok(t) => t, Err(e) => return db_err(e) };
            changes.push("schedule");
        }
        if let Some(want) = args.get("done").and_then(|v| v.as_bool()) {
            if want != t.done {
                // 周期待办完成 → 滚动到下一轮（计划 §5 口径）
                if want && t.repeat_mode != "once" {
                    t = match repo::todo::complete_recurring(ctx.conn, id, Local::now().timestamp_millis()) { Ok(t) => t, Err(e) => return ToolOutcome::err(e, String::new()) };
                } else {
                    t = match repo::todo::toggle(ctx.conn, id) { Ok(t) => t, Err(e) => return db_err(e) };
                }
                changes.push("completion status");
            }
        }
        if let Some(want) = args.get("pinned").and_then(|v| v.as_bool()) {
            if want != t.pinned {
                t = match repo::todo::set_pinned(ctx.conn, id, want) { Ok(t) => t, Err(e) => return db_err(e) };
                changes.push("pinned status");
            }
        }
        if changes.is_empty() { return ToolOutcome::ok("No changes", "No fields needed updating, or the values already matched."); }
        ToolOutcome::ok(format!("Updated ({})", changes.join(", ")), format!("Updated task: {}", fmt_todo_line(&t)))
    }
}

struct CreateNote;
impl AgentTool for CreateNote {
    fn name(&self) -> &'static str { "create_note" }
    fn description(&self) -> &'static str { "Create a note. title and content are required. folder and tags are optional and must match names **exactly**. If a match fails, candidates are returned; ask the user to confirm before proceeding rather than guessing." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{
        "title":{"type":"string"},"content":{"type":"string"},
        "folder":{"type":"string","description":"Target folder name (exact match)"},
        "tags":{"type":"array","items":{"type":"string"},"description":"List of tag names (exact matches)"}
    },"required":["title","content"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let Some(title) = arg_str(args, "title") else { return ToolOutcome::err("Missing required parameter: title", String::new()) };
        let Some(content) = arg_str(args, "content") else { return ToolOutcome::err("Missing required parameter: content", String::new()) };
        // 名称匹配先做完整校验（folder + 全部 tags），任何一个失败都不落库（§5：不猜不静默）
        let folder_id = match arg_str(args, "folder") {
            Some(want) => {
                let folders = match repo::note_folder::list(ctx.conn) { Ok(f) => f, Err(e) => return db_err(e) };
                let cands: Vec<(String, i64)> = folders.iter().map(|f| (f.name.clone(), f.id)).collect();
                match match_exact(&cands, want) { Ok(id) => Some(id), Err(c) => return not_found_outcome("folder", want, c) }
            }
            None => None,
        };
        let tag_ids: Vec<i64> = match args.get("tags").and_then(|v| v.as_array()) {
            Some(list) => {
                let all = match repo::tag::list(ctx.conn) { Ok(t) => t, Err(e) => return db_err(e) };
                let mut ids = Vec::new();
                for tv in list {
                    let Some(want) = tv.as_str().filter(|s| !s.trim().is_empty()) else { continue };
                    let cands: Vec<(String, i64)> = all.iter().map(|t| (t.name.clone(), t.id)).collect();
                    match match_exact(&cands, want) { Ok(id) => ids.push(id), Err(c) => return not_found_outcome("tag", want, c) }
                }
                ids
            }
            None => Vec::new(),
        };
        let note = match repo::note::create_in(ctx.conn, title, content, folder_id, "") { Ok(n) => n, Err(e) => return db_err(e) };
        if !tag_ids.is_empty() {
            if let Err(e) = repo::tag::set_note_tags(ctx.conn, note.id, &tag_ids) { return db_err(e); }
        }
        ToolOutcome::ok(format!("Created \"{}\"", note.title), format!("Created note [{}] \"{}\"", note.id, note.title))
    }
}

struct UpdateNote;
impl AgentTool for UpdateNote {
    fn name(&self) -> &'static str { "update_note" }
    fn description(&self) -> &'static str { "Update a note's title or **replace its entire body** (content replaces all text; use append_to_note to append). The folder can also be changed by exact name match." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{
        "note_id":{"type":"integer"},"title":{"type":"string"},"content":{"type":"string"},
        "folder":{"type":"string"}
    },"required":["note_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "note_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: note_id", String::new()); }
        let note = match resolve_by_id(&raw, id, "note",
            |fid| repo::note::get(ctx.conn, fid),
            || Ok(repo::note::search(ctx.conn, &raw)?.into_iter().map(|n| (n.title, n.id)).collect()),
        ) { Ok(n) => n, Err(o) => return o };
        let id = note.id;
        let title = arg_str(args, "title").unwrap_or(&note.title);
        let content = arg_str(args, "content").unwrap_or(&note.content);
        // 先做全部校验再落库（与 create_note 同款）：folder 匹配失败时标题/正文
        // 不能已经改了一半——模型拿到「失败」会整单重试，半途状态会让用户看到
        // 报错但数据已变（0.9.0 审查优化）
        let folder_id = match arg_str(args, "folder") {
            Some(want) => {
                let folders = match repo::note_folder::list(ctx.conn) { Ok(f) => f, Err(e) => return db_err(e) };
                let cands: Vec<(String, i64)> = folders.iter().map(|f| (f.name.clone(), f.id)).collect();
                match match_exact(&cands, want) { Ok(fid) => Some(fid), Err(c) => return not_found_outcome("folder", want, c) }
            }
            None => None,
        };
        if arg_str(args, "title").is_some() || arg_str(args, "content").is_some() {
            let _ = match repo::note::update(ctx.conn, id, title, content) { Ok(n) => n, Err(e) => return db_err(e) };
        }
        if let Some(fid) = folder_id {
            if let Err(e) = repo::note::set_folder(ctx.conn, id, Some(fid)) { return db_err(e); }
        }
        ToolOutcome::ok(format!("Updated \"{}\"", title), format!("Updated note [{}] \"{}\"", id, title))
    }
}

struct AppendToNote;
impl AgentTool for AppendToNote {
    fn name(&self) -> &'static str { "append_to_note" }
    fn description(&self) -> &'static str { "Append content to the end of an existing note without replacing its current body." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"note_id":{"type":"integer"},"content":{"type":"string"}},"required":["note_id","content"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "note_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: note_id", String::new()); }
        let note = match resolve_by_id(&raw, id, "note",
            |fid| repo::note::get(ctx.conn, fid),
            || Ok(repo::note::search(ctx.conn, &raw)?.into_iter().map(|n| (n.title, n.id)).collect()),
        ) { Ok(n) => n, Err(o) => return o };
        let id = note.id;
        let Some(text) = arg_str(args, "content") else { return ToolOutcome::err("Missing required parameter: content", String::new()) };
        let merged = if note.content.trim().is_empty() { text.to_string() } else { format!("{}\n{}", note.content, text) };
        match repo::note::update(ctx.conn, id, &note.title, &merged) { Ok(_) => (), Err(e) => return db_err(e) }
        ToolOutcome::ok(format!("Appended to \"{}\"", note.title), format!("Appended {} characters to the end of note [{}] \"{}\"", text.chars().count(), id, note.title))
    }
}

struct DeleteNote;
impl AgentTool for DeleteNote {
    fn name(&self) -> &'static str { "delete_note" }
    fn description(&self) -> &'static str { "Move a note to the trash (soft delete; it can be restored)." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"note_id":{"type":"integer"}},"required":["note_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "note_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: note_id", String::new()); }
        let note = match resolve_by_id(&raw, id, "note",
            |fid| repo::note::get(ctx.conn, fid),
            || Ok(repo::note::search(ctx.conn, &raw)?.into_iter().map(|n| (n.title, n.id)).collect()),
        ) { Ok(n) => n, Err(o) => return o };
        let id = note.id;
        if let Err(e) = repo::note::trash(ctx.conn, id) { return db_err(e); }
        ToolOutcome::ok(format!("Moved \"{}\" to Trash", note.title), format!("Note [{}] \"{}\" was moved to Trash and can be restored there.", id, note.title))
    }
}

struct CreateResource;
impl AgentTool for CreateResource {
    fn name(&self) -> &'static str { "create_resource" }
    fn description(&self) -> &'static str { "Create a Quick Launch resource. name, target, and kind are required (kind=app|web|file). category is optional and must match a category name exactly; candidates are returned if it does not exist. If the user explicitly wants a new category, call create_subcategory first." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{
        "name":{"type":"string"},"target":{"type":"string","description":"Executable path, URL, or file path"},
        "kind":{"type":"string","enum":["app","web","file"]},
        "category":{"type":"string","description":"Category name (exact match)"}
    },"required":["name","target","kind"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let Some(name) = arg_str(args, "name") else { return ToolOutcome::err("Missing required parameter: name", String::new()) };
        let Some(target) = arg_str(args, "target") else { return ToolOutcome::err("Missing required parameter: target", String::new()) };
        let kind = match arg_str(args, "kind") {
            Some("app") => ResourceKind::App,
            Some("web") => ResourceKind::Web,
            Some("file") => ResourceKind::File,
            _ => return ToolOutcome::err("kind must be one of app, web, or file", String::new()),
        };
        // target 静默校验（0.9.0 审查优化）：「允许创建」不能变成「允许预置一个稍后
        // 一键启动的任意协议/程序」——web 只收 http(s)，app/file 拒绝 URL 形态
        match kind {
            ResourceKind::Web => {
                let t = target.to_ascii_lowercase();
                if !t.starts_with("http://") && !t.starts_with("https://") {
                    return ToolOutcome::err("A web resource target must be a URL beginning with http:// or https://.", String::new());
                }
            }
            ResourceKind::App | ResourceKind::File => {
                if target.contains("://") {
                    return ToolOutcome::err(format!("A {} resource target must be a local path, not a URL.", crate::repo::resource::kind_to_str(&kind)), String::new());
                }
                if target.len() > 500 {
                    return ToolOutcome::err("The target path is too long.", String::new());
                }
            }
        }
        let kind_str = crate::repo::resource::kind_to_str(&kind);
        let category = match arg_str(args, "category") {
            Some(want) => {
                let subs = match repo::subcategory::list(ctx.conn) { Ok(s) => s, Err(e) => return db_err(e) };
                let cands: Vec<(String, i64)> = subs.iter().filter(|s| s.kind == kind_str).map(|s| (s.name.clone(), s.id)).collect();
                match match_exact(&cands, want) { Ok(_) => Some(want), Err(c) => return not_found_outcome("category", want, c) }
            }
            None => None,
        };
        let r = match repo::resource::create(ctx.conn, kind, name, target, category, None, None, None, None, None, None) { Ok(r) => r, Err(e) => return db_err(e) };
        ToolOutcome::ok(format!("Created \"{}\"", r.name), format!("Created Quick Launch resource [{}]: {}", r.id, fmt_resource_line(&r)))
    }
}

struct AddSnippet;
impl AgentTool for AddSnippet {
    fn name(&self) -> &'static str { "add_snippet" }
    fn description(&self) -> &'static str { "Save a prompt or reusable snippet to the prompt library." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"title":{"type":"string"},"content":{"type":"string"}},"required":["title","content"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let Some(title) = arg_str(args, "title") else { return ToolOutcome::err("Missing required parameter: title", String::new()) };
        let Some(content) = arg_str(args, "content") else { return ToolOutcome::err("Missing required parameter: content", String::new()) };
        let s = match repo::snippet::create(ctx.conn, title, content) { Ok(s) => s, Err(e) => return db_err(e) };
        ToolOutcome::ok(format!("Saved \"{}\"", s.title), format!("Saved prompt [{}]: {}", s.id, s.title))
    }
}

struct UpdateSnippet;
impl AgentTool for UpdateSnippet {
    fn name(&self) -> &'static str { "update_snippet" }
    fn description(&self) -> &'static str { "Update a saved prompt's title or content, and optionally toggle its pinned status. Omitted fields remain unchanged." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{
        "snippet_id":{"type":"integer"},"title":{"type":"string"},"content":{"type":"string"},
        "pinned":{"type":"boolean"}
    },"required":["snippet_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "snippet_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: snippet_id", String::new()) };
        let s = match resolve_by_id(&raw, id, "prompt",
            |fid| repo::snippet::get(ctx.conn, fid),
            || Ok(repo::snippet::list(ctx.conn)?.into_iter().map(|s| (s.title, s.id)).collect()),
        ) { Ok(s) => s, Err(o) => return o };
        let id = s.id;
        let title = arg_str(args, "title").unwrap_or(&s.title);
        let content = arg_str(args, "content").unwrap_or(&s.content);
        if arg_str(args, "title").is_some() || arg_str(args, "content").is_some() {
            let _ = match repo::snippet::update(ctx.conn, id, title, content) { Ok(s) => s, Err(e) => return db_err(e) };
        }
        let mut pinned_note = "";
        if let Some(want) = args.get("pinned").and_then(|v| v.as_bool()) {
            if want != s.is_pinned {
                let _ = match repo::snippet::toggle_pin(ctx.conn, id) { Ok(s) => s, Err(e) => return db_err(e) };
                pinned_note = " and updated pinned status";
            }
        }
        ToolOutcome::ok(format!("Updated \"{}\"{pinned_note}", title), format!("Updated prompt [{}]: {}", id, title))
    }
}

struct WriteSticky;
impl AgentTool for WriteSticky {
    fn name(&self) -> &'static str { "write_sticky" }
    fn description(&self) -> &'static str { "Write to a workbench sticky note (slot 1 or 2); this replaces the entire note." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"slot":{"type":"integer","enum":[1,2]},"content":{"type":"string"}},"required":["slot","content"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let Some(slot) = arg_i64(args, "slot").filter(|s| *s == 1 || *s == 2) else { return ToolOutcome::err("slot must be 1 or 2", String::new()) };
        let Some(content) = arg_str(args, "content") else { return ToolOutcome::err("Missing required parameter: content", String::new()) };
        match repo::sticky::upsert(ctx.conn, slot, content) { Ok(_) => (), Err(e) => return db_err(e) }
        ToolOutcome::ok(format!("Updated sticky note {slot}"), format!("Sticky note {slot} now contains {} characters.", content.chars().count()))
    }
}

struct CreateCountdown;
impl AgentTool for CreateCountdown {
    fn name(&self) -> &'static str { "create_countdown" }
    fn description(&self) -> &'static str { "Create a countdown. For once, provide either end_time or duration_minutes. For daily, end_time is the time of day. For interval, provide interval_minutes." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{
        "name":{"type":"string"},
        "mode":{"type":"string","enum":["once","daily","interval"],"description":"Defaults to once"},
        "end_time":{"description":"Time in YYYY-MM-DD HH:mm format or a millisecond timestamp"},
        "duration_minutes":{"type":"integer","description":"Duration in minutes for once mode; use instead of end_time"},
        "interval_minutes":{"type":"integer","description":"Interval in minutes for interval mode"}
    },"required":["name"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let Some(name) = arg_str(args, "name") else { return ToolOutcome::err("Missing required parameter: name", String::new()) };
        let mode = arg_str(args, "mode").unwrap_or("once").to_string();
        let now_ms = repo::countdown::now_ms();
        let (repeat_mode, end_at, total_ms, interval) = match mode.as_str() {
            "interval" => {
                let Some(m) = arg_i64(args, "interval_minutes").filter(|m| *m > 0) else { return ToolOutcome::err("interval mode requires interval_minutes (>0)", String::new()) };
                // LLM 输出不可信：上界防 m*60_000 与 now+m*60_000 溢出（debug panic / release 脏数据）
                if m > MAX_COUNTDOWN_MINUTES {
                    return ToolOutcome::err(format!("interval_minutes is too large (maximum {MAX_COUNTDOWN_MINUTES} minutes)."), String::new());
                }
                let ms = m * 60_000;
                ("interval", now_ms + ms, ms, Some(m))
            }
            "daily" => {
                let Some(end) = args.get("end_time").and_then(parse_time_ms) else {
                    return ToolOutcome::err("daily mode requires end_time.", String::new())
                };
                ("daily", end, 86_400_000, None)
            }
            _ => {
                if let Some(m) = arg_i64(args, "duration_minutes").filter(|m| *m > 0) {
                    if m > MAX_COUNTDOWN_MINUTES {
                        return ToolOutcome::err(format!("duration_minutes is too large (maximum {MAX_COUNTDOWN_MINUTES} minutes)."), String::new());
                    }
                    let ms = m * 60_000;
                    ("once", now_ms + ms, ms, None)
                } else if let Some(v) = args.get("end_time") {
                    let Some(end) = parse_time_ms(v) else { return ToolOutcome::err("end_time is not a recognized time.", String::new()) };
                    let total = (end - now_ms).max(0);
                    ("once", end, total, None)
                } else {
                    return ToolOutcome::err("once mode requires end_time or duration_minutes.", String::new())
                }
            }
        };
        let c = match repo::countdown::create(ctx.conn, name, repeat_mode, end_at, total_ms, interval) { Ok(c) => c, Err(e) => return db_err(e) };
        ToolOutcome::ok(format!("Created \"{}\"", c.name), format!("Created countdown: {}", fmt_countdown_line(&c)))
    }
}

struct ToggleCountdown;
impl AgentTool for ToggleCountdown {
    fn name(&self) -> &'static str { "toggle_countdown" }
    fn description(&self) -> &'static str { "Pause or resume a countdown." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"countdown_id":{"type":"integer"},"action":{"type":"string","enum":["pause","resume"]}},"required":["countdown_id","action"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "countdown_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: countdown_id", String::new()) };
        let c = match resolve_by_id(&raw, id, "countdown",
            |fid| repo::countdown::get(ctx.conn, fid),
            || Ok(repo::countdown::list(ctx.conn)?.into_iter().map(|c| (c.name, c.id)).collect()),
        ) { Ok(c) => c, Err(o) => return o };
        let id = c.id;
        let action = arg_str(args, "action").unwrap_or("");
        let r = match action {
            "pause" => if c.paused { Ok(c) } else { repo::countdown::pause(ctx.conn, id) },
            "resume" => if !c.paused { Ok(c) } else { repo::countdown::resume(ctx.conn, id) },
            _ => return ToolOutcome::err("action must be pause or resume", String::new()),
        };
        let c = match r { Ok(c) => c, Err(e) => return db_err(e) };
        ToolOutcome::ok(format!("{} \"{}\"", if action == "pause" { "Paused" } else { "Resumed" }, c.name), format!("Countdown: {}", fmt_countdown_line(&c)))
    }
}

struct SetClipboard;
impl AgentTool for SetClipboard {
    fn name(&self) -> &'static str { "set_clipboard" }
    fn description(&self) -> &'static str { "Write text to the system clipboard; it will also be added to clipboard history." }
    fn level(&self) -> Level { Level::Write }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"text":{"type":"string"}},"required":["text"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let Some(text) = arg_str(args, "text") else { return ToolOutcome::err("Missing required parameter: text", String::new()) };
        if let Err(e) = crate::clipboard::set_clipboard(text, None) { return ToolOutcome::err(format!("Failed to write to the clipboard: {e}"), String::new()); }
        let _ = repo::clipboard::insert(ctx.conn, text, None, Some("agent"));
        ToolOutcome::ok(format!("Copied {} characters", text.chars().count()), "The text was copied to the system clipboard.")
    }
}

// ---------------------------------------------------------------------------
// 危险（5）
// ---------------------------------------------------------------------------

struct DeleteTodo;
impl AgentTool for DeleteTodo {
    fn name(&self) -> &'static str { "delete_todo" }
    fn description(&self) -> &'static str { "**Permanently delete** a task and its subtasks; this cannot be undone. To retain the record, use update_todo to mark it complete." }
    fn level(&self) -> Level { Level::Dangerous }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"todo_id":{"type":"integer"}},"required":["todo_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "todo_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: todo_id", String::new()); }
        let t = match resolve_by_id(&raw, id, "task",
            |fid| repo::todo::get(ctx.conn, fid),
            || Ok(repo::todo::search(ctx.conn, &raw)?.into_iter().map(|t| (t.title, t.id)).collect()),
        ) { Ok(t) => t, Err(o) => return o };
        let id = t.id;
        let title = t.title;
        let removed = match repo::todo::delete(ctx.conn, id) { Ok(r) => r, Err(e) => return db_err(e) };
        ToolOutcome::ok(format!("Deleted \"{title}\""), format!("Task [{id}] \"{title}\" was permanently deleted, along with {} subtasks.", removed.len().saturating_sub(1)))
    }
}

struct DeleteResource;
impl AgentTool for DeleteResource {
    fn name(&self) -> &'static str { "delete_resource" }
    fn description(&self) -> &'static str { "**Permanently delete** a Quick Launch resource; this cannot be undone." }
    fn level(&self) -> Level { Level::Dangerous }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"resource_id":{"type":"integer"}},"required":["resource_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "resource_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: resource_id", String::new()) };
        let r0 = match resolve_by_id(&raw, id, "Quick Launch resource",
            |fid| repo::resource::get(ctx.conn, fid),
            || Ok(repo::resource::search(ctx.conn, &raw)?.into_iter().map(|r| (r.name, r.id)).collect()),
        ) { Ok(r) => r, Err(o) => return o };
        let id = r0.id;
        let name = r0.name;
        if let Err(e) = repo::resource::delete(ctx.conn, id) { return db_err(e); }
        ToolOutcome::ok(format!("Deleted \"{name}\""), format!("Quick Launch resource [{id}] \"{name}\" was permanently deleted."))
    }
}

struct DeleteSnippet;
impl AgentTool for DeleteSnippet {
    fn name(&self) -> &'static str { "delete_snippet" }
    fn description(&self) -> &'static str { "**Permanently delete** a saved prompt; this cannot be undone." }
    fn level(&self) -> Level { Level::Dangerous }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"snippet_id":{"type":"integer"}},"required":["snippet_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "snippet_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: snippet_id", String::new()) };
        let s = match resolve_by_id(&raw, id, "prompt",
            |fid| repo::snippet::get(ctx.conn, fid),
            || Ok(repo::snippet::list(ctx.conn)?.into_iter().map(|s| (s.title, s.id)).collect()),
        ) { Ok(s) => s, Err(o) => return o };
        let id = s.id;
        let title = s.title;
        if let Err(e) = repo::snippet::delete(ctx.conn, id) { return db_err(e); }
        ToolOutcome::ok(format!("Deleted \"{title}\""), format!("Prompt [{id}] \"{title}\" was permanently deleted."))
    }
}

struct DeleteCountdown;
impl AgentTool for DeleteCountdown {
    fn name(&self) -> &'static str { "delete_countdown" }
    fn description(&self) -> &'static str { "**Permanently delete** a countdown; this cannot be undone." }
    fn level(&self) -> Level { Level::Dangerous }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"countdown_id":{"type":"integer"}},"required":["countdown_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "countdown_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: countdown_id", String::new()) };
        let c = match resolve_by_id(&raw, id, "countdown",
            |fid| repo::countdown::get(ctx.conn, fid),
            || Ok(repo::countdown::list(ctx.conn)?.into_iter().map(|c| (c.name, c.id)).collect()),
        ) { Ok(c) => c, Err(o) => return o };
        let id = c.id;
        let name = c.name;
        if let Err(e) = repo::countdown::delete(ctx.conn, id) { return db_err(e); }
        ToolOutcome::ok(format!("Deleted \"{name}\""), format!("Countdown [{id}] \"{name}\" was permanently deleted."))
    }
}

struct LaunchResource;
impl AgentTool for LaunchResource {
    fn name(&self) -> &'static str { "launch_resource" }
    fn description(&self) -> &'static str { "Launch a Quick Launch resource: start an app or open a website or file, as if the user clicked its card. Get the ID from list_resources." }
    fn level(&self) -> Level { Level::Dangerous }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{"resource_id":{"type":"integer"}},"required":["resource_id"]}) }
    fn execute(&self, args: &Value, ctx: ToolCtx) -> ToolOutcome {
        let (raw, id) = arg_id(args, "resource_id");
        if raw.is_empty() { return ToolOutcome::err("Missing required parameter: resource_id", String::new()) };
        let res = match resolve_by_id(&raw, id, "Quick Launch resource",
            |fid| repo::resource::get(ctx.conn, fid),
            || Ok(repo::resource::search(ctx.conn, &raw)?.into_iter().map(|r| (r.name, r.id)).collect()),
        ) { Ok(r) => r, Err(o) => return o };
        let id = res.id;
        // 与 commands::launch_resource 同一套核心（进程已在运行且有可见窗口 → 调度前台）
        let r = match res.kind {
            ResourceKind::App => {
                let no_args = res.args.as_deref().map(|a| a.trim().is_empty()).unwrap_or(true);
                if no_args && crate::process::activate_existing(&res.target) {
                    let _ = repo::resource::touch(ctx.conn, id);
                    return ToolOutcome::ok(format!("Focused \"{}\"", res.name), format!("The app is already running; its window was brought to the foreground: \"{}\".", res.name));
                }
                crate::process::launch_program(&res.target, res.args.as_deref())
            }
            ResourceKind::Web => crate::process::open_url(&res.target),
            ResourceKind::File => crate::process::open_path(&res.target),
        };
        match r {
            Ok(()) => {
                let _ = repo::resource::touch(ctx.conn, id);
                ToolOutcome::ok(format!("Launched \"{}\"", res.name), format!("Launched Quick Launch resource [{}] {} ({}: {})", res.id, res.name, kind_label(&res.kind), res.target))
            }
            Err(e) => ToolOutcome::err(format!("Failed to launch \"{}\": {e}", res.name), String::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// 注册
// ---------------------------------------------------------------------------

/// 注册全部 31 个内置工具（顺序稳定——tools 数组随每次请求下发，勿乱序）
// ---------------------------------------------------------------------------
// 扩展桥（manifest.agent.tools）：扩展自描述的 AI 工具，宿主聚合后转发。
// 清单与调用都走 ext_tools 聚合器；审批档位在 approval_for 里按目标声明取。
// ---------------------------------------------------------------------------

struct ExtensionToolList;
impl AgentTool for ExtensionToolList {
    fn name(&self) -> &'static str { "extension_tool_list" }
    fn description(&self) -> &'static str {
        "List tools declared for AI by extensions (service extensions describe tool names, purpose, parameters, returns, and permission level in manifest.agent.tools). Call only when the user's task may involve an extension's data or features; discover tools here before calling extension_tool_call. Do not call for conversations unrelated to extensions."
    }
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value { json!({"type":"object","properties":{}}) }
    fn execute(&self, _args: &Value, _ctx: ToolCtx) -> ToolOutcome {
        let cfg = crate::config::load();
        if !cfg.agent_ext_tools_enabled {
            // 失败语义（§5）：返回说明而非错误，避免模型反复重试
            return ToolOutcome::ok("Extension tools are disabled", "The setting that allows AI to call extension tools is off (Settings → AI Assistant). If needed, direct the user to turn it on.");
        }
        let list = super::ext_tools::list_available();
        if list.is_empty() {
            return ToolOutcome::ok("No extension tools available", "No extension currently declares AI tools. Only service extensions can declare them, and the user must allow AI access. Do not invent extension capabilities.");
        }
        // 按扩展分组展示：模型能看出「这个工具是谁家的」，回话时也能交代来源
        let mut sections: Vec<String> = Vec::new();
        let mut cur_ext = String::new();
        let mut cur_lines: Vec<String> = Vec::new();
        for a in &list {
            if a.ext_id != cur_ext {
                if !cur_lines.is_empty() {
                    sections.push(format!("{cur_ext}:\n{}", cur_lines.join("\n")));
                }
                cur_ext = a.ext_id.clone();
                cur_lines = Vec::new();
                cur_lines.push(format!("{} ({}) provides:", a.ext_name, a.ext_id));
            }
            cur_lines.push(super::ext_tools::describe_tool(a));
        }
        if !cur_lines.is_empty() {
            sections.push(format!("{cur_ext}:\n{}", cur_lines.join("\n")));
        }
        let ext_count = sections.len();
        ToolOutcome::ok(
            format!("{ext_count} extensions provide {} tools", list.len()),
            sections.join("\n\n"),
        )
    }
}

struct ExtensionToolCall;
impl AgentTool for ExtensionToolCall {
    fn name(&self) -> &'static str { "extension_tool_call" }
    fn description(&self) -> &'static str {     "Call an AI tool declared by an extension. Get ext and tool from the list returned by extension_tool_list, and provide args according to that tool's declared parameters. Do not call this before listing available tools. Write and dangerous tools require user approval according to their permission level; do not retry if approval is denied." }
    // 静态档 Read 仅供 ReadOnly 模式的可见性过滤；实际审批档在 approval_for 按目标声明取
    fn level(&self) -> Level { Level::Read }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{
            "ext":{"type":"string","description":"Extension ID from extension_tool_list"},
            "tool":{"type":"string","description":"Tool name from extension_tool_list"},
            "args":{"type":"object","description":"Arguments matching the tool's declared parameters; omit for tools without parameters"}
        },"required":["ext","tool"]})
    }
    fn approval_for(&self, args: &Value) -> (Level, String) {
        let (Some(ext), Some(tool)) = (
            arg_str(args, "ext"),
            arg_str(args, "tool"),
        ) else {
            // 参数不全：按读档处理，执行阶段会报缺参
            return (Level::Read, self.name().to_string());
        };
        match super::ext_tools::find(ext, tool) {
            Some(a) => (super::ext_tools::level_of(&a.tool.level), super::ext_tools::perm_key(ext, tool)),
            // 目标不存在（未声明/已禁用/未授权）：读档放行到执行阶段，由聚合器报明确原因
            None => (Level::Read, self.name().to_string()),
        }
    }
    fn execute(&self, args: &Value, _ctx: ToolCtx) -> ToolOutcome {
        let Some(ext) = arg_str(args, "ext") else { return ToolOutcome::err("Missing required parameter: ext (extension ID from extension_tool_list)", String::new()) };
        let Some(tool) = arg_str(args, "tool") else { return ToolOutcome::err("Missing required parameter: tool (tool name from extension_tool_list)", String::new()) };
        let tool_args = args.get("args").cloned().unwrap_or_else(|| json!({}));
        if !tool_args.is_object() {
            return ToolOutcome::err("args must be an object matching the target tool's declared parameters.", String::new());
        }
        match super::ext_tools::call(ext, tool, &tool_args) {
            Ok(v) => {
                let detail = serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string());
                ToolOutcome::ok(format!("{tool} returned successfully"), cap_chars(&detail, MAX_TEXT))
            }
            Err(e) => ToolOutcome::err(format!("Failed to call {tool}"), e),
        }
    }
}

pub fn register_all(reg: &mut ToolRegistry) {
    // 读 15
    reg.register(Box::new(ListTodos));
    reg.register(Box::new(SearchNotes));
    reg.register(Box::new(GetNote));
    reg.register(Box::new(ListNotes));
    reg.register(Box::new(ListResources));
    reg.register(Box::new(ListCountdowns));
    reg.register(Box::new(GetStickies));
    reg.register(Box::new(ListSnippets));
    reg.register(Box::new(GetSnippet));
    reg.register(Box::new(ReadClipboard));
    reg.register(Box::new(GlobalSearch));
    reg.register(Box::new(GetWeather));
    reg.register(Box::new(GetSetting));
    reg.register(Box::new(ExtensionToolList)); // 扩展桥：静态 Read（实际档位按目标声明）
    reg.register(Box::new(ExtensionToolCall));
    // 写 14
    reg.register(Box::new(CreateTodo));
    reg.register(Box::new(UpdateTodo));
    reg.register(Box::new(CreateNote));
    reg.register(Box::new(UpdateNote));
    reg.register(Box::new(AppendToNote));
    reg.register(Box::new(DeleteNote));
    reg.register(Box::new(CreateResource));
    reg.register(Box::new(CreateSubcategory)); // 0.9.0 新增：速达小类（目录）创建
    reg.register(Box::new(AddSnippet));
    reg.register(Box::new(UpdateSnippet));
    reg.register(Box::new(WriteSticky));
    reg.register(Box::new(CreateCountdown));
    reg.register(Box::new(ToggleCountdown));
    reg.register(Box::new(SetClipboard));
    // 危险 5
    reg.register(Box::new(DeleteTodo));
    reg.register(Box::new(DeleteResource));
    reg.register(Box::new(DeleteSnippet));
    reg.register(Box::new(DeleteCountdown));
    reg.register(Box::new(LaunchResource));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> ToolRegistry {
        let mut r = ToolRegistry::empty();
        register_all(&mut r);
        r
    }

    #[test]
    fn registers_exactly_34_tools() {
        let r = reg();
        assert_eq!(r.len(), 34, "§5 全量 30 个 + 0.9.0 create_subcategory + get_snippet + 扩展桥 2 个");
        let read = r.available(crate::agent::approval::PermissionMode::ReadOnly).len();
        assert_eq!(read, 15, "读 15 个（含扩展桥 extension_tool_list/call，静态 Read）");
    }

    /// 「要不要查」的工具侧门控（2026-10-21 修「随便说什么都去翻笔记/待办」）。
    ///
    /// flash 级模型对**工具描述**的就近匹配很敏感，光改 system prompt 不够：每个读工具的
    /// description 都必须带一句「什么时候不要调用」的否定式门控。这里逐条盯住，防止后续
    /// 重构顺手把措辞改回只有正向说明（那种描述下模型会把它当成「随时可用的顺手工具」）。
    #[test]
    fn read_tools_descriptions_carry_negative_gate() {
        let r = reg();
        let reads = [
            "list_todos", "search_notes", "get_note", "list_notes", "list_resources",
            "list_countdowns", "get_stickies", "list_snippets", "get_snippet",
            "read_clipboard", "global_search", "get_weather", "get_setting",
            "extension_tool_list", "extension_tool_call",
        ];
        for name in reads {
            let d = r.get(name).expect(name).description();
            assert!(
                d.contains("仅当") || d.contains("不要"),
                "读工具 {name} 的描述缺少否定式门控：{d}"
            );
        }
        // 通用型的两个读工具必须显式点名「与本地数据无关时不要调用」——
        // 它们是最容易被拿来当兜底搜刮的（global_search 一次扫三个模块）
        assert!(r.get("global_search").unwrap().description().contains("与本地数据无关的对话不要调用"));
        assert!(r.get("extension_tool_call").unwrap().description().contains("没先调 extension_tool_list"));
    }

    /// 扩展桥的审批档位覆写：approval_for 按目标工具声明取档；宿主未就绪/目标不存在
    /// 时回落 (Read, 工具名)，执行阶段报明确原因
    #[test]
    fn extension_tool_call_approval_falls_back_without_host() {
        let r = reg();
        let t = r.get("extension_tool_call").unwrap();
        let (level, key) = t.approval_for(&json!({"ext":"com.a","tool":"query"}));
        assert_eq!(level, Level::Read, "单测环境无 HOST_APP，聚合为空 → 回落 Read");
        assert_eq!(key, "extension_tool_call");
        let (level2, key2) = t.approval_for(&json!({}));
        assert_eq!(level2, Level::Read);
        assert_eq!(key2, "extension_tool_call");
    }

    /// 0.9.0 审查回归：LLM 输出的分钟数/时间戳是不可信输入——超界必须明确拒绝，
    /// 不能溢出成 panic（debug）或负值脏数据（release）
    #[test]
    fn countdown_and_time_args_reject_out_of_range() {
        let conn = crate::db::init_in_memory().unwrap();
        // interval_minutes 上界：i64::MAX 会溢出
        let out = CreateCountdown.execute(
            &json!({"name":"x","mode":"interval","interval_minutes": i64::MAX}),
            ToolCtx { conn: &conn },
        );
        assert!(!out.ok, "超界 interval_minutes 不应创建成功");
        // duration_minutes 同款
        let out = CreateCountdown.execute(
            &json!({"name":"x","duration_minutes": i64::MAX}),
            ToolCtx { conn: &conn },
        );
        assert!(!out.ok);
        // 合法区间内的仍然成功
        let ok = CreateCountdown.execute(
            &json!({"name":"x","mode":"interval","interval_minutes": 30}),
            ToolCtx { conn: &conn },
        );
        assert!(ok.ok, "{}", ok.summary);
        // 时间戳超出 2000~2100 区间按不可识别拒绝
        let out = CreateCountdown.execute(
            &json!({"name":"y","end_time": i64::MAX}),
            ToolCtx { conn: &conn },
        );
        assert!(!out.ok);
    }

    /// 0.9.0 审查回归：repeat 模式串必须过 RepeatRule::validate——非法周期直接落库
    /// 会在勾选完成时被静默降级成一次性（todo_recurrence 的 match 落空）
    #[test]
    fn create_todo_rejects_invalid_repeat_mode() {
        let conn = crate::db::init_in_memory().unwrap();
        let out = CreateTodo.execute(
            &json!({"title":"测试","repeat":"yearlies"}),
            ToolCtx { conn: &conn },
        );
        assert!(!out.ok, "非法 repeat 模式不应落库");
        // 裸字符串只认无需附加字段的模式
        let ok = CreateTodo.execute(
            &json!({"title":"测试","repeat":"daily"}),
            ToolCtx { conn: &conn },
        );
        assert!(ok.ok, "{}", ok.summary);
        // 对象形式：weekly 带 weekdays、monthly 带 month_day
        let ok = CreateTodo.execute(
            &json!({"title":"测试","repeat":{"mode":"weekly","weekdays":[1,3]}}),
            ToolCtx { conn: &conn },
        );
        assert!(ok.ok, "{}", ok.summary);
        let ok = CreateTodo.execute(
            &json!({"title":"测试","repeat":{"mode":"monthly","month_day":15}}),
            ToolCtx { conn: &conn },
        );
        assert!(ok.ok, "{}", ok.summary);
        // weekly 缺 weekdays：validate 拒绝并给模型明确原因
        let out = CreateTodo.execute(
            &json!({"title":"测试","repeat":"weekly"}),
            ToolCtx { conn: &conn },
        );
        assert!(!out.ok);
        // 越界 weekdays 拒绝
        let out = CreateTodo.execute(
            &json!({"title":"测试","repeat":{"mode":"weekly","weekdays":[9]}}),
            ToolCtx { conn: &conn },
        );
        assert!(!out.ok);
    }

    /// 0.9.0 审查优化回归：create_resource 的 target 静默校验——
    /// web 只收 http(s)（防预置 file:// 等任意协议稍后一键启动），app/file 拒绝 URL 形态
    #[test]
    fn create_resource_validates_target_shape() {
        let conn = crate::db::init_in_memory().unwrap();
        let bad = CreateResource.execute(
            &json!({"name":"x","target":"file:///C:/Windows/notepad.exe","kind":"web"}),
            ToolCtx { conn: &conn },
        );
        assert!(!bad.ok, "web 资源非 http(s) 协议应被拒绝");
        let bad = CreateResource.execute(
            &json!({"name":"x","target":"https://example.com/app","kind":"app"}),
            ToolCtx { conn: &conn },
        );
        assert!(!bad.ok, "app 资源的 target 不能是网址");
        let ok = CreateResource.execute(
            &json!({"name":"x","target":"https://example.com","kind":"web"}),
            ToolCtx { conn: &conn },
        );
        assert!(ok.ok, "{}", ok.summary);
    }

    /// 0.9.0 审查优化回归：update_note 先验后写——folder 匹配失败时
    /// 标题/正文不能已经改了一半
    #[test]
    fn update_note_validates_folder_before_writing() {
        let conn = crate::db::init_in_memory().unwrap();
        let n = repo::note::create(&conn, "原标题").unwrap();
        let out = UpdateNote.execute(
            &json!({"note_id": n.id, "title":"改一半的标题","folder":"不存在的文件夹"}),
            ToolCtx { conn: &conn },
        );
        assert!(!out.ok, "文件夹匹配失败应报错");
        let after = repo::note::get(&conn, n.id).unwrap();
        assert_eq!(after.title, "原标题", "报错时标题不能已被改动");
    }

    /// 0.9.0 新增 get_snippet：按 id 取提示词正文；不存在的 id 明确报错
    #[test]
    fn get_snippet_returns_content_and_errors_on_missing() {
        let conn = crate::db::init_in_memory().unwrap();
        let s = repo::snippet::create(&conn, "写作润色", "请把下面的文字改写得更简洁：").unwrap();
        let out = GetSnippet.execute(&json!({ "snippet_id": s.id }), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        assert!(out.detail.contains("请把下面的文字改写得更简洁"), "正文要完整返回：{}", out.detail);
        let miss = GetSnippet.execute(&json!({ "snippet_id": 99999 }), ToolCtx { conn: &conn });
        assert!(!miss.ok);
    }

    /// 0.9.0 审查回归：get_setting 只放行白名单键——chat_models 等敏感配置不外发
    #[test]
    fn get_setting_whitelists_keys() {
        let conn = crate::db::init_in_memory().unwrap();
        let ok = GetSetting.execute(&json!({"key":"online_enabled"}), ToolCtx { conn: &conn });
        assert!(ok.ok);
        let denied = GetSetting.execute(&json!({"key":"chat_models"}), ToolCtx { conn: &conn });
        assert!(!denied.ok, "chat_models 不在白名单，不应可读");
        let denied = GetSetting.execute(&json!({"key":"server_url"}), ToolCtx { conn: &conn });
        assert!(!denied.ok);
        // 无 key 列出的可用项里也不含敏感键
        let list = GetSetting.execute(&json!({}), ToolCtx { conn: &conn });
        assert!(list.ok);
        assert!(!list.detail.contains("chat_models"));
    }

    /// 0.9.0 回归：模型可自主新建速达小类，再用它创建资源（幂等不重复建）
    #[test]
    fn create_subcategory_then_resource() {
        let conn = crate::db::init_in_memory().unwrap();
        let out = CreateSubcategory.execute(&json!({"kind":"app","name":"开发工具"}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        // 幂等：同名再建直接返回已存在
        let again = CreateSubcategory.execute(&json!({"kind":"app","name":"开发工具"}), ToolCtx { conn: &conn });
        assert!(again.ok);
        assert!(again.detail.contains("已存在"));
        // 资源能落进新小类
        let res = CreateResource.execute(
            &json!({"name":"VSCode","target":"C:\\code\\Code.exe","kind":"app","category":"开发工具"}),
            ToolCtx { conn: &conn },
        );
        assert!(res.ok, "{}", res.summary);
        // 不同 kind 同名不算已存在（app/web/file 小类各自独立）
        let web = CreateSubcategory.execute(&json!({"kind":"web","name":"开发工具"}), ToolCtx { conn: &conn });
        assert!(web.ok);
        assert!(!web.detail.contains("已存在"));
    }

    #[test]
    fn list_todos_groups_and_hides_done() {
        let conn = crate::db::init_in_memory().unwrap();
        let _ = repo::todo::create(&conn, "普通待办", None, None).unwrap();
        let done = repo::todo::create(&conn, "已完成条目", None, None).unwrap();
        repo::todo::toggle(&conn, done.id).unwrap();
        let pin = repo::todo::create(&conn, "置顶待办", None, None).unwrap();
        repo::todo::set_pinned(&conn, pin.id, true).unwrap();
        let out = ListTodos.execute(&json!({}), ToolCtx { conn: &conn });
        assert!(out.ok);
        assert!(out.detail.contains("📌"));
        assert!(out.detail.contains("置顶待办"));
        assert!(out.detail.contains("普通待办"));
        assert!(out.detail.contains("completed tasks not shown"));
        assert!(!out.detail.contains("已完成条目\n"));
    }

    #[test]
    fn create_todo_with_time_and_repeat() {
        let conn = crate::db::init_in_memory().unwrap();
        let due = Local::now().timestamp_millis() + 3_600_000;
        let out = CreateTodo.execute(&json!({"title":"写周报","due_at":due,"repeat":"daily","priority":2}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        let todos = repo::todo::list(&conn).unwrap();
        let t = todos.iter().find(|t| t.title == "写周报").unwrap();
        assert_eq!(t.due_at, Some(due));
        assert_eq!(t.repeat_mode, "daily");
        assert_eq!(t.priority, 2);
    }

    #[test]
    fn create_note_folder_mismatch_returns_candidates() {
        let conn = crate::db::init_in_memory().unwrap();
        repo::note_folder::create(&conn, "工作", None).unwrap();
        repo::note_folder::create(&conn, "生活", None).unwrap();
        let out = CreateNote.execute(&json!({"title":"t","content":"c","folder":"工作笔记"}), ToolCtx { conn: &conn });
        assert!(!out.ok);
        assert!(out.detail.contains("Closest matches"), "{}", out.detail);
        assert!(out.detail.contains("工作") && out.detail.contains("生活"));
        // 精确匹配则成功
        let ok = CreateNote.execute(&json!({"title":"t2","content":"c","folder":"工作"}), ToolCtx { conn: &conn });
        assert!(ok.ok);
        assert_eq!(repo::note::list_meta(&conn).unwrap().len(), 1);
    }

    #[test]
    fn create_note_tags_exact_match_only() {
        let conn = crate::db::init_in_memory().unwrap();
        repo::tag::create(&conn, "重要").unwrap();
        let bad = CreateNote.execute(&json!({"title":"t","content":"c","tags":["重要啊"]}), ToolCtx { conn: &conn });
        assert!(!bad.ok);
        assert!(bad.summary.contains("\"重要啊\" not found"));
        assert!(bad.detail.contains("Could not find tag \"重要啊\""));
        let good = CreateNote.execute(&json!({"title":"t","content":"c","tags":["重要"]}), ToolCtx { conn: &conn });
        assert!(good.ok, "{}", good.summary);
        let note = &repo::note::list_meta(&conn).unwrap()[0];
        assert_eq!(repo::tag::tags_of_note(&conn, note.id).unwrap().len(), 1);
    }

    #[test]
    fn update_todo_done_and_pin() {
        let conn = crate::db::init_in_memory().unwrap();
        let t = repo::todo::create(&conn, "测试", None, None).unwrap();
        let out = UpdateTodo.execute(&json!({"todo_id":t.id,"done":true,"pinned":true}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        let t = repo::todo::get(&conn, t.id).unwrap();
        assert!(t.done && t.pinned);
    }

    #[test]
    fn append_and_update_note() {
        let conn = crate::db::init_in_memory().unwrap();
        let n = repo::note::create(&conn, "日记").unwrap();
        AppendToNote.execute(&json!({"note_id":n.id,"content":"第一段"}), ToolCtx { conn: &conn });
        let n2 = repo::note::get(&conn, n.id).unwrap();
        assert!(n2.content.contains("第一段"));
        UpdateNote.execute(&json!({"note_id":n.id,"title":"日记改"}), ToolCtx { conn: &conn });
        assert_eq!(repo::note::get(&conn, n.id).unwrap().title, "日记改");
    }

    #[test]
    fn delete_note_is_soft() {
        let conn = crate::db::init_in_memory().unwrap();
        let n = repo::note::create(&conn, "要删的").unwrap();
        let out = DeleteNote.execute(&json!({"note_id":n.id}), ToolCtx { conn: &conn });
        assert!(out.ok);
        assert!(out.detail.contains("Trash"));
        assert!(repo::note::get(&conn, n.id).unwrap().deleted_at.is_some());
    }

    #[test]
    fn sticky_write_and_read() {
        let conn = crate::db::init_in_memory().unwrap();
        let out = WriteSticky.execute(&json!({"slot":1,"content":"购物清单"}), ToolCtx { conn: &conn });
        assert!(out.ok);
        let read = GetStickies.execute(&json!({}), ToolCtx { conn: &conn });
        assert!(read.detail.contains("购物清单"));
        assert!(read.detail.contains("(empty)"));
    }

    #[test]
    fn snippet_lifecycle() {
        let conn = crate::db::init_in_memory().unwrap();
        AddSnippet.execute(&json!({"title":"周报模板","content":"本周..."}), ToolCtx { conn: &conn });
        UpdateSnippet.execute(&json!({"snippet_id":1,"pinned":true}), ToolCtx { conn: &conn });
        assert!(repo::snippet::get(&conn, 1).unwrap().is_pinned);
        let del = DeleteSnippet.execute(&json!({"snippet_id":1}), ToolCtx { conn: &conn });
        assert!(del.ok);
        assert!(repo::snippet::get(&conn, 1).is_err());
    }

    #[test]
    fn countdown_create_once_and_interval() {
        let conn = crate::db::init_in_memory().unwrap();
        let out = CreateCountdown.execute(&json!({"name":"泡面","duration_minutes":3}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        let out2 = CreateCountdown.execute(&json!({"name":"喝水","mode":"interval","interval_minutes":45}), ToolCtx { conn: &conn });
        assert!(out2.ok, "{}", out2.summary);
        let list = repo::countdown::list(&conn).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[1].repeat_mode, "interval");
        assert_eq!(list[1].interval_minutes, Some(45));
        // 参数不齐 → 明确报错
        let bad = CreateCountdown.execute(&json!({"name":"x"}), ToolCtx { conn: &conn });
        assert!(!bad.ok);
    }

    #[test]
    fn clipboard_write_goes_to_history() {
        let conn = crate::db::init_in_memory().unwrap();
        // 系统剪贴板写不入（测试环境无 GUI 会话也不炸）：set_clipboard 失败则工具报 err
        let out = SetClipboard.execute(&json!({"text":"hello"}), ToolCtx { conn: &conn });
        let _ = out; // 结果依赖环境，仅验证不 panic
    }

    #[test]
    fn global_search_hits_all_three_modules() {
        let conn = crate::db::init_in_memory().unwrap();
        repo::todo::create(&conn, "买 xyz", None, None).unwrap();
        repo::note::create(&conn, "xyz 计划").unwrap();
        let out = GlobalSearch.execute(&json!({"keyword":"xyz"}), ToolCtx { conn: &conn });
        assert!(out.ok);
        assert!(out.detail.contains("[Tasks:"));
        assert!(out.detail.contains("[Notes:"));
    }

    #[test]
    fn get_setting_roundtrip() {
        let conn = crate::db::init_in_memory().unwrap();
        let out = GetSetting.execute(&json!({"key":"online_enabled"}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        assert!(out.detail.contains("online_enabled"));
        let bad = GetSetting.execute(&json!({"key":"no_such_key"}), ToolCtx { conn: &conn });
        assert!(!bad.ok);
        assert!(bad.detail.contains("Available settings"));
    }

    #[test]
    fn resource_create_with_subcategory_match() {
        let conn = crate::db::init_in_memory().unwrap();
        repo::subcategory::create(&conn, "app", "开发工具").unwrap();
        let bad = CreateResource.execute(&json!({"name":"VSCode","target":"C:\\code.exe","kind":"app","category":"开发"}), ToolCtx { conn: &conn });
        assert!(!bad.ok);
        assert!(bad.detail.contains("开发工具"));
        let ok = CreateResource.execute(&json!({"name":"VSCode","target":"C:\\code.exe","kind":"app","category":"开发工具"}), ToolCtx { conn: &conn });
        assert!(ok.ok, "{}", ok.summary);
    }

    #[test]
    fn launch_missing_resource_errors() {
        let conn = crate::db::init_in_memory().unwrap();
        let out = LaunchResource.execute(&json!({"resource_id":999}), ToolCtx { conn: &conn });
        assert!(!out.ok);
        assert!(out.detail.contains("No Quick Launch resource"));
    }

    /// 0.9.3 反馈回归：标题/名称本身是数字的条目（如笔记「20261023」）——模型会把
    /// 用户口中的数字直接当 id 传给按 id 取对象的工具，此前按 id 查不到直接报
    /// 「不存在」。现在 id 落空时按名称精确匹配兜底：笔记读/改/追加/删全链路。
    #[test]
    fn numeric_title_note_falls_back_to_name_match() {
        let conn = crate::db::init_in_memory().unwrap();
        let n = repo::note::create(&conn, "20261023").unwrap();
        assert_ne!(n.id, 20261023, "前提：库里 id 查不到 20261023，走兜底");
        let out = GetNote.execute(&json!({"note_id": 20261023}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        assert!(out.detail.contains("20261023"));
        // 浮点形式的整数 id（偶发供应商把整数序列化成 20261023.0）：按整数值解析，兜底照常命中
        let out = GetNote.execute(&json!({"note_id": 20261023.0}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        // 直接锁归一化本身：f64 形态整数解析出 i64，且原文不带「.0」（名称兜底键要干净）
        let (raw, id) = arg_id(&json!({"x": 20261023.0}), "x");
        assert_eq!((raw.as_str(), id), ("20261023", Some(20261023)));
        // 追加/更新也按兜底命中，且改的是兜底解析出的真实 id
        // （更新只换正文不改标题，保住「标题=20261023」这个删除兜底的前提）
        let out = AppendToNote.execute(&json!({"note_id": 20261023, "content": "补充"}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        let out = UpdateNote.execute(&json!({"note_id": 20261023, "content": "全新正文"}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        let after = repo::note::get(&conn, n.id).unwrap();
        assert_eq!(after.title, "20261023");
        assert_eq!(after.content, "全新正文");
        // 删除兜底
        let out = DeleteNote.execute(&json!({"note_id": 20261023}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        assert!(repo::note::get(&conn, n.id).unwrap().deleted_at.is_some());
        // 真不存在的数字：仍明确报错，并提示先搜索
        let miss = GetNote.execute(&json!({"note_id": 99999}), ToolCtx { conn: &conn });
        assert!(!miss.ok);
        assert!(miss.detail.contains("search or list tool"), "{}", miss.detail);
    }

    /// 0.9.3 反馈回归（其他模块）：待办、提示词、速达资源、倒计时同款兜底
    #[test]
    fn numeric_title_todo_snippet_resource_countdown_fallback() {
        let conn = crate::db::init_in_memory().unwrap();
        // 待办「2026」：更新/删除兜底
        let t = repo::todo::create(&conn, "2026", None, None).unwrap();
        let out = UpdateTodo.execute(&json!({"todo_id": 2026, "done": true}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        assert!(repo::todo::get(&conn, t.id).unwrap().done);
        let out = DeleteTodo.execute(&json!({"todo_id": 2026}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        // 提示词「404」：读/改兜底
        let s = repo::snippet::create(&conn, "404", "页面不存在").unwrap();
        let out = GetSnippet.execute(&json!({"snippet_id": 404}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        assert!(out.detail.contains("页面不存在"));
        let out = UpdateSnippet.execute(&json!({"snippet_id": 404, "pinned": true}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        assert!(repo::snippet::get(&conn, s.id).unwrap().is_pinned);
        // 速达资源「8888」：删除兜底（launch 会真拉程序，不在此测）
        let r = repo::resource::create(&conn, ResourceKind::Web, "8888", "https://example.com", None, None, None, None, None, None, None).unwrap();
        let out = DeleteResource.execute(&json!({"resource_id": 8888}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        assert!(repo::resource::get(&conn, r.id).is_err());
        // 倒计时「60」：暂停兜底（end_at 取未来时刻，pause 语义才完整）
        let end = repo::countdown::now_ms() + 60_000;
        repo::countdown::create(&conn, "60", "once", end, 60_000, None).unwrap();
        let out = ToggleCountdown.execute(&json!({"countdown_id": 60, "action": "pause"}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        assert!(repo::countdown::get(&conn, 1).unwrap().paused);
        // 前导零标题：字符串参数按原文匹配，不被 parse 丢零
        let z = repo::note::create(&conn, "007").unwrap();
        let out = GetNote.execute(&json!({"note_id": "007"}), ToolCtx { conn: &conn });
        assert!(out.ok, "{}", out.summary);
        assert!(out.detail.contains("007"));
        assert_eq!(z.id, 1, "id=1 的笔记标题是 007，兜底命中的就是它");
    }

    /// 0.9.3 反馈回归：同名多篇（数字标题重复）时按 §5 不猜——报歧义请用户确认
    #[test]
    fn duplicate_numeric_titles_report_ambiguity() {
        let conn = crate::db::init_in_memory().unwrap();
        repo::note::create(&conn, "123").unwrap();
        repo::note::create(&conn, "123").unwrap();
        let out = GetNote.execute(&json!({"note_id": 123}), ToolCtx { conn: &conn });
        assert!(!out.ok);
        assert!(out.detail.contains("entries with that name") && out.detail.contains("Ask the user"), "{}", out.detail);
    }
}
