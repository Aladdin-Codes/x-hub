//! System prompt 组装（计划 §4 已定，#t44）：四段——身份边界 / 当前时间 / 行为准则 / 关于我。
//!
//! 当前时间**注入而不是做成工具**：做成工具模型每轮都得先花一次调用问"今天几号"，
//! 费额度又慢；时间属于「每轮都需要的背景信息」。故首批工具表里没有日期工具。

use chrono::{Datelike, Timelike};

/// 组装 system prompt。`now` 用本地时间；`about_me` 为设置「AI 助手 → 关于我」的
/// 用户自填档案（首期只读注入，模型不写回；空/None 时整段省略）。
pub fn build_system_prompt(now: chrono::DateTime<chrono::Local>, about_me: Option<&str>) -> String {
    let mut s = String::new();

    // 1. 身份与边界
    s.push_str(
        "You are the built-in assistant for x-hub, a personal productivity workspace. You can use tools to work with local notes, tasks, prompts, Quick Launch resources, sticky notes, clipboard history, and countdowns. Installed service extensions can declare their own data and capabilities through extension_tool_list (for example, querying an extension's records). List available tools before using them when needed; never invent extension capabilities. Be transparent about capabilities you do not have (such as web search, writing code, accessing the file system, or memory across sessions). Never claim to have done something you did not do.\n\n",
    );

    // 2. 当前时间（动态注入）
    s.push_str(&format!("Current local time: {}.\n\n", time_text(now)));

    // 3. 行为准则
    //    第 1 条是「要不要查」的前置闸门，必须有：旧版只有「需要事实时必须查证」这一条正向
    //    强制、没有反向门槛，flash 级模型会泛化成「任何消息都先查一圈再说」（用户反馈：
    //    随便说点什么都去翻笔记/待办）。闸门只约束**读探查**；写操作仍归第 5 条铁律管，
    //    第 1 条末句把两者切开，防止模型从「事事都查」摆到「该查也不查、直接编」。
    s.push_str(
        "Behavior guidelines:\n\
1. Decide whether to look something up before choosing what to search. Ask whether the answer depends on local data:\n\
   - If it does, use the relevant tool. Examples: \"What tasks do I have?\", \"What does that note say?\", or \"Update the item I wrote down yesterday.\" Report what the tool finds.\n\
   - If it does not, answer directly. **Do not** call read tools such as list_todos, search_notes, or global_search just to take a look. Greetings, general knowledge questions, explanations, writing, editing, translation, and brainstorming do not depend on local data.\n\
   - If unsure, answer directly or ask a follow-up question; do not call a tool just to test whether it might help.\n\
   This rule applies only to read-only exploration. It does not change write operations: when asked to create a task, note, or prompt, you must still call the corresponding write tool.\n\
2. Do not search endlessly when there are no results. If a search finds nothing, try at most once more with a broader term. If that also finds nothing, say so plainly; do not switch tools and fetch the entire notes library or task list.\n\
3. If you decide a lookup is needed, actually perform it. Use tools to verify facts (for example, \"What tasks do I have?\" or \"What does that note say?\") instead of making them up from memory.\n\
4. Before a write operation, briefly tell the user what you are going to do.\n\
5. A write operation is complete only after you **actually call the corresponding tool**. Never claim that something was saved, created, or completed unless the tool was called. If the tool fails, is unavailable, or lacks permission, explain the failure honestly.\n\
6. Do only what the user asked; do not expand the scope on your own.\n\
7. Reply in the user's language.\n\
8. If an exact name match fails for a folder, tag, category, or similar parameter, show the closest candidates and ask the user to confirm. Do not guess or silently use a default location.\n\
9. A number the user mentions may be an item's title rather than its ID (for example, a note titled \"20261023\"). Search for the number as a keyword first, then use the returned [id]. Do not pass the number directly as an ID. If multiple matches exist, show the candidates and ask; if none exist, report that plainly.\n",
    );

    // 4. 用户档案（可选）
    if let Some(text) = about_me.map(str::trim).filter(|t| !t.is_empty()) {
        s.push_str(&format!(
            "\nUser profile (background information provided by the user):\n{text}\n"
        ));
    }
    s
}

/// Time line: `2026-08-29 Saturday 14:32`
fn time_text(now: chrono::DateTime<chrono::Local>) -> String {
    let weekday = match now.weekday() {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    };
    format!(
        "{:04}-{:02}-{:02} {} {:02}:{:02}",
        now.year(),
        now.month(),
        now.day(),
        weekday,
        now.hour(),
        now.minute()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn prompt_has_four_sections_and_injected_time() {
        let now = chrono::Local.with_ymd_and_hms(2026, 8, 29, 14, 32, 0).unwrap();
        let p = build_system_prompt(now, None);
        assert!(p.contains("built-in assistant for x-hub"));
        assert!(p.contains("capabilities you do not have"));
        assert!(p.contains("Current local time: 2026-08-29 Saturday 14:32"));
        assert!(p.contains("instead of making them up from memory"));
        assert!(p.contains("Never claim that something was saved, created, or completed"));
        assert!(p.contains("closest candidates"));
        assert!(!p.contains("User profile"));
    }

    /// 「要不要查」的前置闸门（2026-10-21 修「随便说什么都去翻笔记/待办」）：
    /// 必须同时存在三样东西，缺一即回归——
    /// ① 判定标准是「答案是否依赖本机数据」；② 明确禁止无关消息顺手调读工具；
    /// ③ 闸门只管读探查、写工具照旧必须调用（防从「事事都查」摆到「该查也不查」）
    #[test]
    fn prompt_gates_read_tools_but_keeps_write_rules() {
        let now = chrono::Local.with_ymd_and_hms(2026, 10, 21, 9, 0, 0).unwrap();
        let p = build_system_prompt(now, None);
        assert!(p.contains("whether the answer depends on local data"), "missing decision criterion: {p}");
        assert!(p.contains("Do not") && p.contains("just to take a look"), "missing negative gate: {p}");
        assert!(p.contains("Greetings, general knowledge questions"), "missing examples of unrelated requests: {p}");
        assert!(
            p.contains("This rule applies only to read-only exploration")
                && p.contains("must still call the corresponding write tool"),
            "闸门必须显式排除写操作，否则会吓退该调的写工具：{p}"
        );
        // 搜刮收敛：一次换词重试，再无结果如实说没找到
        assert!(p.contains("Do not search endlessly when there are no results"), "missing no-result convergence rule: {p}");
        assert!(p.contains("try at most once more with a broader term"), "missing retry limit: {p}");
        // 反谎报铁律与闸门并存（两条都在，谁也没被谁挤掉）
        assert!(p.contains("Never claim that something was saved, created, or completed"));
        assert!(p.contains("instead of making them up from memory"));
    }

    /// Rust 字符串用 `\` 续行时**会吞掉下一行的前导空白**——断点选在符号中间就会把
    /// 两个词粘成一个（实测踩过：`search_notes、` 断在行尾后 `global_search` 接续，
    /// 渲染成 `search_notes、global_search` 是对的，但断在 `/` 后就会粘成 `notes /global_search`）。
    /// 这里直接在渲染结果上断言，别靠读源码看。
    #[test]
    fn prompt_line_continuations_do_not_glue_words() {
        let now = chrono::Local.with_ymd_and_hms(2026, 10, 21, 9, 0, 0).unwrap();
        let p = build_system_prompt(now, None);
        // 断点落在斜杠后面会渲染出 " /global_search" 这种粘连
        assert!(!p.contains("/global_search"), "续行把工具名粘在一起了：{p}");
        // 三个工具名必须各自独立出现
        for name in ["list_todos", "search_notes", "global_search"] {
            assert!(p.contains(name), "缺少工具名 {name}：{p}");
        }
    }

    #[test]
    fn about_me_appended_when_present() {        let now = chrono::Local.with_ymd_and_hms(2026, 1, 5, 8, 5, 0).unwrap();
        let p = build_system_prompt(now, Some("I'm a backend developer; please keep replies concise."));
        assert!(p.contains("User profile"));
        assert!(p.contains("I'm a backend developer; please keep replies concise."));
        // 空白档案视同未填
        let p2 = build_system_prompt(now, Some("   "));
        assert!(!p2.contains("User profile"));
    }
}
