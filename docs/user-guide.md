# x-hub User Guide

> Applies to: v0.9.4 (Windows desktop)
> x-hub is a **local-first** personal productivity workspace with a Bento-style dashboard, to-dos, notes, quick launch (apps, websites, and files), an AI assistant, clipboard history, and an extension system. By default, all data stays on your device and is not uploaded to the cloud.

**How to read this guide**

- “设置 → 外观 → 主题模式” means: click “设置” at the bottom left of the sidebar → select the “外观” category in the left pane → find “主题模式.”
- “Right-click” means click the right mouse button; “hover” means move the pointer over an item without clicking.
- UI labels are quoted exactly as they appear, such as “新建待办.”

---

## Contents

1. [About x-hub](#1-about-x-hub)
2. [Global Shortcuts](#2-global-shortcuts)
3. [Dashboard](#3-dashboard)
4. [To-dos](#4-to-dos)
5. [Notes](#5-notes)
6. [Quick Launch](#6-quick-launch)
7. [AI Assistant](#7-ai-assistant)
8. [Clipboard History](#8-clipboard-history)
9. [Global Search](#9-global-search)
10. [Extension Center](#10-extension-center)
11. [Floating Ball and System Tray](#11-floating-ball-and-system-tray)
12. [Settings at a Glance](#12-settings-at-a-glance)
13. [Data and Maintenance](#13-data-and-maintenance)
14. [App Updates](#14-app-updates)
15. [FAQ](#15-faq)

---

## 1. About x-hub

### 1.1 Interface overview

| Area | Description |
|---|---|
| **Title bar** | Full-width bar at the top. Drag an empty area to move the window; double-click an empty area to maximize or restore it. Buttons on the right, from left to right: **Global Search**, **AI Chat**, **Always on Top** (toggle with the pin), **Minimize**, **Maximize/Restore**, and **Close**. |
| **Sidebar** | Vertical bar on the left, collapsed to 56px icon mode by default. Hover over an icon for about half a second to show its label. Navigation, top to bottom: **工作台 → 待办 → 速记 → 速达 → AI 助手**; pinned extension icons appear below, with **扩展中心** and **设置** (bottom left) in the fixed area at the bottom. |
| **Main area** | Content area for the current view. |
| **System tray** | Icon in the lower-right corner of the screen. x-hub continues running here after its window is closed. |

### 1.2 Window controls and exiting

- **Move the window**: Drag an empty area of the title bar.
- **Keep on top**: Click the pin in the title bar to keep the window above other windows.
- **Close does not mean exit**: Clicking “关闭” minimizes x-hub to the system tray (you’ll see a notice the first time). To exit completely, **right-click the tray icon → 退出**.
- **Tray icon**: Left-click to show or hide the main window; right-click for the menu: show the main window / hide the main window / restart / exit.

### 1.3 Where your data is stored

- **Standard edition**: `C:\Users\you\AppData\Roaming\x-hub` (move it from 设置 → 数据与关于 → 数据 → “更改”; see §13).
- **Portable edition**: The `data` folder next to the executable (cannot be moved).
- The database, note images, icons, wallpapers, and backups are stored there. Uninstalling the app does not delete this data folder.

---

## 2. Global Shortcuts

These system-wide shortcuts work **in any app**. Manage them under **设置 → 常规 → 快捷键**.

| Action | Default shortcut | What it does |
|---|---|---|
| Global shortcut (show/hide main window) | `Ctrl+Shift+Space` | Show or hide the main window |
| Open clipboard history | `` Ctrl+` `` | Open the clipboard history overlay (see §8) |
| Open search | `Ctrl+K` | Open Global Search (see §9) |
| Open AI chat | `Ctrl+Shift+K` | Open AI chat in a drawer or separate window (see §7.1) |
| Open Notes | `Ctrl+Shift+N` | Show the main window → open Notes → create a note |
| Screenshot | `Ctrl+Shift+A` | Open the full-screen selection overlay; release after drawing a selection to capture it |
| GIF recording | `Ctrl+Shift+G` | Open the full-screen selection overlay and record the selected area as a GIF |

**Change a shortcut**: Click the field on its row and press a new key combination, or click “录入” and press the keys while recording. Shortcuts are checked for conflicts before saving: conflicts with other x-hub shortcuts are identified, and shortcuts used by another app show “可能正被其它程序占用.”

**Temporarily disable a shortcut**: Use the toggle on the right of its row. Disabling unregisters the system hotkey but **keeps its assigned keys**. Turn it back on to restore it; you don’t need to record it again.

**Screenshot**: After you press the shortcut, the screen dims and **the window under the pointer is selected automatically** (move the pointer to select another). **Left-click to select the highlighted area**, which enters adjustment mode and shows the toolbar. To define a custom area, **drag to draw a selection**; releasing the pointer also enters adjustment mode. Drag a corner handle to fine-tune the selection, or **drag anywhere inside or outside the selection to move it**. The icon-only toolbar sits below the selection. Use the **pen / rectangle / circle** tools to add red annotations. **Double-click to capture** or click ✓ on the toolbar. The image is **copied to the clipboard only** (not saved to disk); the main window displays a brief “已复制” notice, and you can paste the image wherever you need it. Press `Esc` to cancel. The screenshot does not include the mouse pointer.

**Pin a screenshot**: In adjustment mode, click “钉住” to keep an annotated screenshot **on top at its original screen position** while also copying it to the clipboard. This lets you refer to it while working. Drag the pinned image to move it; hover over its upper-right corner and click **×** to close it. You can pin up to 3 images at once.

**GIF recording**: After pressing the shortcut, select the recording area automatically or by dragging. In the panel at the bottom, choose a **maximum duration** (10/15/30/60 seconds), then click ▶ to start. **The default frame rate is configured in Settings** (设置 → 功能 → GIF 录制 → “默认帧率”; default 20fps; lower it to reduce file size). The selection overlay disappears when recording starts so it won’t appear in the recording. A floating timer bar appears; click “停止” to finish, or recording stops automatically at the duration limit. When it’s done, the main window displays a brief notice; click “打开” to locate the file. Playback speed reflects the timing of the recorded actions for smooth playback. Change the save folder under 设置 → 功能 → GIF 录制 (the default is the `captures` folder in the data directory). Press `Esc` to cancel. GIFs include the mouse pointer.

---

## 3. Dashboard

The dashboard is a flexible Bento-style grid. You choose the position, size, and layout variant of every card.

### 3.1 Layout editor (core features)

**Open it**: 设置 → 工作台 → “自定义布局” → “打开编辑器”. If the dashboard is empty, the main view also displays a “自定义布局” button. Click “确认” when you’re done to return to the dashboard.

The canvas is a 12-column × 15-row grid. It shows a proportionally scaled preview of your actual dashboard; the clock and weather cards are rendered using the real components.

**Basic operations**

- **Add a card**: Drag an item from the module library on the left onto the canvas. New cards are placed at the variant’s **minimum size**. If the target area is occupied, the editor finds the nearest open space below it. For modules with multiple variants (clock/weather), select a variant chip on the item before dragging it.
- **Move**: Drag a card to a new position on the canvas. It is placed where you drop it.
- **Resize**: Drag the small handle in the card’s lower-right corner. The card cannot be smaller than the variant’s minimum size. Resizing is canceled if it would overlap another card.
- **Remove**: Hover over a card and click “×” in its upper-right corner, or drag the card back to the library on the left.
- **Switch variants**: Hover over a multi-variant card and click “⇄”. Each option in the popover has a thumbnail, name, and size indicator (✓ fits the current grid / ⚠ needs a larger grid). The card grows automatically if needed.
- **Set a custom title**: Hover over a card and click “Aa”. Change its title (up to 24 characters; leave it blank to restore the default) or select “不显示标题行” to hide the header.
- **Fit badges** (lower-left corner of a card; shown on hover, or always visible when “填充审计徽标” is selected at the top):
  - Green “正好铺满” = matches the recommended size; yellow “紧凑可读” = usable but smaller than recommended; blue “弹性空间” = larger than recommended; red “低于最小” = content will be clipped; consider enlarging the card.
- **System Monitor card**: Hover over the card and click the slider icon in its controls to open display settings (see §3.5).

**Top toolbar**

- “清空” clears the canvas; “推荐布局” restores the built-in recommended layout with 9 modules.
- “存为默认” saves the current canvas as the default snapshot and **takes effect immediately**. Clicking “取消” afterward will not undo it.
- “恢复默认” restores the snapshot. In the editor, this only changes the draft; click “确认” to apply it.
- **“确认” saves and returns**. Leaving without confirming discards all changes from this editing session.

**Two useful notes**

- Removing a card from the layout **does not delete its data**. Sticky notes, countdowns, and custom Quick Launch cards keep their contents and are restored when you add them again.
- When you remove the Countdown card from the dashboard, countdowns that are not in floating windows pause automatically (see §3.11).

### 3.2 Clock card (three variants)

| Variant | Contents | Minimum/recommended size |
|---|---|---|
| Large clock | Time + date and weekday + weather in the upper-right corner + quote at the bottom | 3×3 / 4×3 |
| Today’s calendar | Time + Gregorian date + lunar date + Chinese zodiac and sexagenary cycle | 2×2 / 3×3 |
| Minimal clock | Large hours and minutes + smaller seconds | 2×1 / 2×2 |

- Switch variants in the layout editor (see §3.1).
- **Click the quote** at the bottom of the large clock to see another one. Enter custom text (up to 50 characters) under 设置 → 工作台 → “时钟卡片语录” to keep it fixed. Set “名言来源” to online (random quotes, with a local fallback when offline) or local only.

### 3.3 Weather card (two variants)

- **Compact** (2×1 / 2×2): A horizontal row with an icon, temperature, conditions, and city.
- **Detailed** (3×3 / 4×4): The same header as above, plus feels-like temperature, humidity, wind speed, and conditions in four tiles.
- Weather data comes from the Open-Meteo API. **Enable internet access** (设置 → 功能 → 联网); the weather card is hidden when offline.
- **Set the city**: Go to 设置 → 功能 → 联网 → “天气城市,” enter a city name (major cities in China are included) and click “设置,” or click “自动定位” (IP-based location may be inaccurate).

### 3.4 System Monitor card

- Displays CPU, memory, network, and other metrics. Progress bars turn orange-red at 85% or higher. A green pulsing dot beside the title indicates active sampling (refreshes every 2 seconds in the foreground, every 5 seconds when unfocused, and pauses when out of view).
- Choose from 6 metrics: **Upload/Download** (↑ and ↓ values on one row), **CPU**, **Memory** (with smaller used/total values), **Total network speed**, **CPU threads**, and **Total traffic** (accumulated since this app was started; resets when the app restarts).
- **Choose and reorder metrics**: In the **layout editor**, click the slider icon in this card’s controls → “系统资源 · 显示设置.” Select the metrics, reorder them with ↑↓, or click “恢复默认” to show Upload/Download + CPU + Memory. Click “确定” to save; canceling discards changes.
- Content is clipped if the card is too small; make it taller to show more.

### 3.5 Sticky notes 1 and 2

- Type directly in the card. Notes **save automatically after 600ms**; no button is needed.
- Use the button at the right of the header: click “脱离为悬浮便签” to open the note in a separate desktop window (drag to move, use the pin to keep it on top, and click × to close). Once detached, the button changes to “聚焦浮窗”; click it to bring the floating window forward.
- When closing a floating note: an empty note is deleted. If the other sticky-note slot is empty, you’ll be asked “还原到主面板?” If both slots contain notes, you can only delete or cancel.
- Removing a Sticky Note card from the dashboard does not delete its contents. Add the same slot (Sticky Note 1 or 2) again to restore it.

### 3.6 Diary card (quick notes)

- Use the wide input field; **press Enter to save and Shift+Enter for a new line** (Enter during IME composition does not trigger a save).
- Entries are saved in the “日记” folder in Notes (created automatically if it doesn’t exist). The note title is its save time, for example, `2026年10月24日 15:30:02`.
- The input clears after a successful save. If saving fails, the text remains and you’ll be prompted to try again.

### 3.7 To-do card (dashboard)

- Type in the field at the top and **press Enter to add**. Text containing two or more numbered items is split into separate to-dos (see §4.4).
- Switch between the “待办 / 已完成” tabs. Use the tag filter row to filter by tag.
- To-dos are grouped as Pinned → Overdue → Today → Dated → No date. Empty groups are hidden.
- Click “待办浮窗” in the upper-right corner to open a separate window (see §4.10).
- Row actions work the same as in the To-dos view (see §4.6). Click a date badge on the card to open quick scheduling (Today / Tomorrow / Next Monday + mini calendar + reminder toggle).

### 3.8 Calendar card (read-only)

- The month calendar shows to-dos by date. Chips for incomplete items use the accent color, overdue items are red, and completed items are gray with strikethrough. Recurring to-dos appear as dashed virtual instances. Additional items are collapsed into “+N”; hover over a date to see the full list.
- **Click anywhere on the card to open the To-dos view.** Use the arrows to move to the previous or next month.

### 3.9 Overview cards

| Card | Contents | Opens |
|---|---|---|
| Notes overview | Note count / tag count / most recently edited note (relative time) | Upper-right corner → Notes |
| To-do overview | Completion-rate ring + “还有 N 项待完成” + total / added today / completed | Upper-right corner → To-dos |
| Quick Launch counts | Total resources + counts and proportion bars for apps / websites / files | Upper-right corner → Quick Launch |

### 3.10 Countdown card and circular floating windows

**Create one**: Click “＋” at the right of the card header. Enter a name in the dialog and choose one of four tabs:

1. **Duration**: Enter 1–1440 minutes to count down from now (for example, a 25-minute Pomodoro).
2. **Scheduled**: Choose a future date and time.
3. **Daily**: Enter a time of day (defaults to 15:00; if that time has passed today, it starts tomorrow).
4. **Interval**: Repeat a reminder every N minutes.

**List actions**: Hover over an item to pause/resume (⏸/▶), show/hide its floating window, or delete it. Finished items are dimmed and can only be deleted. You can have up to 6 countdowns.

**When a countdown ends**: A notification appears in the lower-right corner, along with a toast in the main window. **Sound is off by default**; enable it under 设置 → 工作台 → “倒计时到点提示音.” Reminders missed while the app is closed or the computer is asleep (more than 5 seconds late) are not sent. Daily and interval countdowns automatically advance to the next occurrence.

**Circular floating window**: The water level reflects the remaining time, with a two-layer wave animation. Hover to show the pause and hide buttons. Drag to move it; its position is remembered. A one-time countdown closes automatically when it ends.

### 3.11 Prompt card

- **Click a row to copy the entire prompt** (a toast appears and the copy count increases by 1). The list is ordered by pinned status, copy count, then most recently copied.
- Click “管理提示词” at the right of the header to open the manager. Click “＋” in the upper-left to add a prompt (title and content are required). Hover over a row to pin, edit, or delete it; a deletion toast includes “撤销.”
- Click “提示词浮窗” at the right of the header to open a separate floating window. It can be moved and kept on top, syncs with the main window in real time, and supports click-to-copy.

### 3.12 Recently used (full-width bar)

- Shows up to 10 Quick Launch resources, ordered by most recently launched. Click an icon to launch it; larger cards show more items.
- When empty, click “去速达添加” to add resources.

### 3.13 Custom Quick Launch 1–4 (suda1..suda4)

Pin any set of Quick Launch resources to a card:

1. Click “配置内容” in the card’s upper-right corner.
2. Choose a content source at the top (select one):
   - **Choose manually**: Search for and select any resources. **They appear in the order selected.**
   - **Apps / Websites / Files**: Automatically show all resources in the selected category and stay in sync with additions and removals in Quick Launch.
   - **Subcategory**: Select one subcategory to show all of its resources, including those in nested subcategories.
3. Click “保存.” Cancel or click the overlay to close without saving.

Click a card item to launch it. Removing the card does not delete its configuration; add that slot again to restore it.

---

## 4. To-dos

### 4.1 Where to find to-dos

- **Sidebar “待办”**: Full view. In a wide window, the list takes 40% on the left and the calendar 60% on the right. In a narrow window, the view switches to one column with “列表 / 日历” buttons.
- **Dashboard “待办” card**: Quick entry and grouped list (see §3.7).
- **Dashboard “日历” card**: Read-only monthly calendar (see §3.8).
- **To-do floating window**: Separate compact window (see §4.10).

### 4.2 Create a to-do

- Click “+ 新建待办” in the upper-right corner of the To-dos view.
- Double-click a day on the calendar, or click “加待办” in “当天明细” (the due date is set to 23:59 that day).
- Press Enter in the input field at the top of the dashboard To-do card or floating window.

### 4.3 Fields in the edit dialog

Double-click a to-do title or click its row to edit it. Fields:

| Field | Description |
|---|---|
| **Title** | Required. Press Enter to save. |
| **Description** | Up to 200 characters; supports lightweight Markdown. Hover over a row in the list to see the full description. Use subtasks for checklists. |
| **Tags** | A separate set of tags for to-dos (independent of note tags). Select multiple colored tag chips, or click “+ 新建标签” to create one. |
| **Pin** | Keeps the item at the top of the “置顶” group, regardless of its date. |
| **Repeat** | See §4.5. |
| **Due date** | In-app calendar and 24-hour time. Quick options: Today / Tomorrow / This Saturday / Next Monday / End of this month / Beginning of next month (each sets 23:59 on the selected day). Click × on the right to clear. |
| **Reminder** | Same date and time controls. Quick options: Same as due date / 1 hour before / 1 day before / Today at 09:00 / Tomorrow at 09:00. |
| **Reminder method** | Appears only when a reminder is set: popup + sound / popup only (default) / sound only / silent. |

In edit mode, the red “删除” button is at the lower-left. Clicking the overlay does not close the dialog, preventing accidental loss of changes.

### 4.4 Split numbered lists into multiple to-dos

In the add field on the dashboard To-do card or floating window, enter text with two or more numbered items and press Enter. The items are split into separate to-dos, and you’ll see “已拆成 N 条待办”:

```
1. Buy groceries 2. Mail a package 3. Review   →  split into 3 to-dos
```

Supported numbering includes `1.`, `1、`, and `1)`. Decimal numbers such as `3.14` are not split accidentally.

### 4.5 Recurring to-dos

Choose one recurrence option: **Does not repeat / Daily / Every weekday / Weekly / Monthly / Yearly / Custom…**

- **Weekly**: Select one or more days: “一 二 三 四 五 六 日.”
- **Custom**: Set an interval, “每 [1-99] [天/周/个月/年]” (every [1–99] days/weeks/months/years).
- **Monthly**: Choose a day of the month (1–31; if the month is shorter, its last day is used) or a weekday occurrence (for example, the second Tuesday).
- **End condition**: Never / On a specific date / After N occurrences. A blue “规则：…” preview updates below. Invalid settings are shown in red, such as a weekly schedule with no days selected.
- A recurring to-do must have a due date set as its recurrence anchor.
- **Checking it off completes the current occurrence**: It does not move to the completed list; its date advances to the next occurrence (a toast such as “已滚到 …” appears, with an undo option). When the recurrence ends, it becomes a regular one-time to-do. Missed past occurrences are not added retroactively.

### 4.6 To-do row actions

- **Complete**: Click the circle at the start of a row. If a parent to-do has incomplete subtasks, confirm “确认并勾选子项” first.
- **Priority**: Click the dot at the start of the row to cycle through gray (normal), yellow (important), and red (urgent).
- **Pin**: Pinned items have a pin icon before the title. Click it to unpin.
- **Badges**:
  - Date badge: overdue items are red (“逾期 M月D日”), today is orange, tomorrow uses the accent color, and later dates are gray. **Click** a badge to open quick scheduling (dashboard card) or the edit dialog (To-dos view).
  - Reminder badge: Green “提醒 HH:MM.” Click it to choose another reminder method from the menu.
  - Recurrence badge: Blue and read-only. Hover to see recurrence details and the total number of completed occurrences.
- **Subtasks**: Hover over a row and click “+”; enter a subtask and press Enter. Subtasks are one level deep. Parent rows show an n/m progress bar. You can complete, edit (double-click), delete, and reorder subtasks within a parent or move them between parents. **Drop a subtask in a blank area of the list to make it a top-level to-do** (a dashed guide says “松手升为顶级待办”; the toast has an undo option).
- **Inline editing**: Double-click the title. Press Enter to save, Esc to cancel, or click elsewhere to save.
- **Drag to reorder or nest**: Drag a row to show an accent-colored insertion line. Top-level items can be reordered within the same group, only in the “未完成” view. **Drop a top-level item in the middle of another to-do’s title row to make it a subtask**. Dropping near the top or bottom edge still reorders the item; the target subtask row is highlighted and the toast offers undo. A top-level item that already has subtasks cannot itself be nested under another item. Subtasks can be reordered under the same parent or moved between parents.
- **Delete and undo**: Hover and click the trash icon. Deleting a parent also deletes its subtasks. The toast includes “撤销”; click within 5 seconds to restore the entire group.

### 4.7 To-dos view toolbar

- **Date range**: Today / This week / This month / All (defaults to This week). Pinned, overdue, and undated items always remain visible.
- **Completed filter**: “看已完成 / 看未完成.”
- **Tag filter**: A row of tag chips with incomplete-item counts; select multiple tags to match. Hover over a chip and click “×” to delete the tag from all to-dos (cannot be undone).
- **Groups**: Pinned → Overdue → Today → This week → This month → Later → No date. Empty groups are hidden.

### 4.8 Month and week calendars

- Each date shows up to 2 chips; additional items collapse into “…+N” (click to select that date). Solid = incomplete; red = overdue; gray with strikethrough and ✓ = completed; **dashed border = virtual recurring instance** (click to see a prompt to edit its recurrence rule).
- **Click a date cell** to select it and show “当天明细” below. **Double-click** a cell to create a to-do for that date. **Click a chip** to edit the item. In “当天明细,” click the **dot at the start** of an item to complete or reopen it (same behavior as in the list: recurring items advance to the next occurrence, and completing a parent with subtasks requires confirmation); click the **title or time** to edit it.
- **Reschedule by dragging**: Drag a solid chip for an incomplete item to another date. Its due date changes to that day while keeping the original time; any reminder moves by the same amount.
- Overdue dates have a pale red background, which clears automatically when all items are completed. Use “月 / 周” to switch views, ‹ › to navigate, and “今天” to return to the current date.

### 4.9 Reminders and notifications

At the scheduled time, the selected method is used: **popup only** (notification card in the lower-right corner + toast in the main window), **popup + sound**, **sound only**, or **silent** (only marks the reminder as fired). Each reminder fires once. Reminders missed by more than 5 seconds while the app is closed or the computer is asleep are not sent later.

### 4.10 To-do floating window

- Open it with “待办浮窗” in the upper-right corner of the dashboard To-do card; click again to close it.
- The frameless compact window can be dragged by its title bar or any empty area and resized from any edge or corner. Click the title-bar pin to keep it on top.
- It contains an add field (supports splitting numbered lists) and a flat list of incomplete to-dos. Row interactions match the card, except there is no scheduling dialog, deletion has no undo, and top-level items cannot be reordered by dragging.

---

## 5. Notes

### 5.1 Interface

Open “速记” in the sidebar (or press `Ctrl+Shift+N` to open it and create a note). The view has two panes:

- **Left pane = folder tree** (the only navigation pane). The tree root contains “未归类” notes. Header buttons, left to right: search, expand/collapse all, **新建笔记**, **新建目录**, and **回收站**.
- **Right pane = editor**.

Selection is exclusive: click a folder to browse it (the editor returns to its empty state), click a note to open it, or click an empty area of the tree to clear the selection and return to the tree root.

### 5.2 Folder tree actions

| Action | How |
|---|---|
| Create a note/folder | Use the header buttons (uses the selected folder; if none is selected, creates at the root). Hover over a folder row and click “+” to create a subfolder. Right-click an empty area of the tree to create a note or folder. |
| Rename | Hover over a row and click the pencil, right-click and choose “重命名,” or select the row and press `F2`. |
| Delete a folder | Hover and click the trash icon, right-click, or select the folder and press `Delete`. In the confirmation dialog, note that **its notes and subfolders move up one level; they are not deleted**. |
| Move by dragging | Drag a note onto a folder to move it there, or to the tree header/bottom blank area to move it to the root. Drag a folder onto another folder to nest it (you cannot move a folder into its own descendant). Drag near the top or bottom edge of a row to reorder folders at the same level. |
| Reorder notes | Drag a note to the **top or bottom edge (about 25%)** of another note row to show an insertion line; release to reorder within the folder. Notes without a manual order are placed after sorted notes by creation time (oldest first). |
| Custom icon | Click the smiley to the left of the title in the editor and choose an emoji (right-click to clear it). It appears at the start of the tree row. |
| Built-in “剪藏” folder | Reserved for web clippings. It cannot be renamed or deleted, but can be exported. |

An empty note is automatically cleaned up if you leave it without entering any content, so it won’t leave behind an “无标题笔记.”

### 5.3 Search and filter by tag

- Click the search icon in the tree header to open the search field. It closes automatically when it loses focus with no query entered.
- Enter text to **search the full text across all folders** (titles and body). Results appear in a flat list with their folder paths.
- Enter `#` to filter by tag. A dropdown lists all tags and note counts; use the arrow keys and Enter to choose one. Right-click a tag in the dropdown to rename or delete it. Enter an exact `#tag-name` to filter by that tag.
- Press `Esc` or clear the query to exit search mode.

### 5.4 Recycle Bin

- Click the Recycle Bin icon in the tree header to see deleted notes in a flat list.
- Hover over a row to “还原” it to its original location or “永久删除” it (cannot be undone). Click “一键清空回收站” in the tree header and confirm to permanently delete everything in the Recycle Bin.
- Notes are soft-deleted (moved to the Recycle Bin). Set the retention period under 设置 → 数据与关于 → 数据 → “回收站保留天数” (keeps items forever by default).

### 5.5 Editor

- **Four modes** (switch with the icons in the upper-right; the mode is remembered per user, not per note): **Live Preview** (WYSIWYG) / **Split Preview** (source on the left, live preview on the right) / **Source** / **Preview Only** (read-only).
- **Title**: Enter up to 80 characters in the field at the top. If the title is blank or still “无标题笔记,” it is derived from the first line of the note. Once you edit it, the title is no longer generated automatically.
- **Auto-save**: Changes are saved after 600ms without typing. The lower-right corner shows “编辑中… / 已保存 时间.”
- **Slash menu**: Type `/` at the start of a line to open the insert menu (headings, lists, code blocks, tables, formulas, images, and more; menu is localized). Type `#`, `>`, `-`, `1.`, `---`, or `- [ ]` and press Enter to quickly create the corresponding structure.
- **Auto-format pasted code**: Plain code copied from elsewhere (without fenced-code markers) is automatically converted to a code block. Click “复制” in the upper-right corner of a code block to copy it.
- **Tags**: At the lower-left of the footer, click “+,” enter a tag (up to 20 characters), and press Enter. Click ✕ on a chip to remove it.
- **Source link**: Web clippings show the original source below the title. Click it to open the link in your default browser.

### 5.6 Spreadsheet notes (Excel)

Spreadsheets are a separate note type alongside text notes, and appear in the folder tree with a green spreadsheet icon:

- **Create**: Click the **spreadsheet icon** in the tree header → “新建空白表格,” or right-click a folder and choose “新建表格” to create one there.
- **Import**: Click the spreadsheet icon in the tree header → “导入 xlsx / csv…”. The filename (without its extension) becomes the note title. For legacy `.xls` files, open them in Excel and save as `.xlsx` first.
- **Edit**: Open a spreadsheet note to edit it in the pane on the right. The editor works offline. You can edit cells, formulas, fonts, fills, borders, merged cells, row heights, column widths, frozen panes, multiple sheets, and hidden sheets. Press `Ctrl+S` to save immediately; changes are also saved automatically after about 1 second without editing.
- **Save**: Changes are written back to the local file without changing its filename. Renaming the note only changes its title. If you permanently delete the note from the Recycle Bin, its file is removed by “孤儿附件清理.”
- **Limitations**: Advanced features such as charts, images, pivot tables, conditional formatting, and data validation are not supported yet and will be lost when opened. Keep a copy of the original file.

### 5.7 Images

- **Insert**: **Paste** an image, **drag it into** the body, or click **upload** in an image block. Supports png/jpg/jpeg/webp/bmp/gif, up to 10MB per image. Images are saved locally in the note image library, with identical content deduplicated automatically.
- **Resize**: Hover over an image to show the handle at its lower-right corner, then drag it. Narrow images can sit side by side; drag an image back close to full width to make it responsive again.
- **Caption**: Enter a caption below the image.
- **Preview**: Click an image to open it in a full-screen lightbox (press Esc or click the overlay to close).
- **Copy image**: Click “复制图片” in the upper-right corner of the lightbox, right-click the image and choose “复制图片,” or select the image and press Ctrl+C. This copies the actual bitmap, which you can paste into a chat, document, or drawing app.

### 5.8 Backlinks

- Type `[[` in the body to open note-title autocomplete (shows titles and folders; use ↑↓ to select and Enter to insert).
- Click a wiki link to open the referenced note. External `http(s)` links open in your default browser.
- Click the **Waypoints icon** to the right of the title to show or hide the full-height “双链面板” on the right. It lists **backlinks** (notes that reference this one) and **outgoing links** (notes referenced by this one); click an entry to open it. References to titles that do not yet exist appear as unresolved mentions and link automatically once the note is created.
- **Renaming a note** automatically updates all `[[old title]]` references across the library in the same transaction, so links remain intact.

### 5.9 One-click formatting and AI refinement

- **One-click formatting** (✨ Sparkles button, available in all four modes): Applies deterministic formatting, such as consistent list markers, spacing around headings, trailing-whitespace cleanup, and spacing between Chinese and English. Press Ctrl+Z once to undo. If the note is already tidy, you’ll see “格式已经很规整了.”
- **AI refinement** (🪄 WandSparkles button): **If text is selected, only the selection is refined; otherwise, the entire note is refined.** A dialog previews the streaming result. Its buttons are Cancel / Copy / **Apply to note** (“替换选中片段” when working with a selection). Closing the dialog discards the result. If the model changes image syntax, x-hub restores it automatically.
  - Configure the model under **设置 → 功能 → 速记 → “AI 深度整理模型.”** The default is “自动” (uses the platform when platform credits are enabled and you’re signed in; otherwise uses your default model). You can also choose platform load balancing or a specific model.
  - **Note content is sent to the model you configured**; the dialog identifies it at the top.

### 5.10 Export

- **Export a note or folder**: In the left pane, **right-click a note or folder → “导出…”** and choose **Markdown / HTML / PDF**. When you right-click a **spreadsheet note**, **Excel (.xlsx)** appears first and is selected by default (xlsx notes export with all content preserved; csv notes are converted to xlsx). Folder exports include subfolders and display progress; you can cancel.
- **Export or import the entire library**: Go to 设置 → 数据与关于 → 数据 (see §13). Exports are Markdown files organized by folder, with images included. Imports support only files exported by x-hub.

---

## 6. Quick Launch

Quick Launch provides one place to launch apps, open website bookmarks, and access files and folders.

### 6.1 Interface and tabs

The five main tabs at the top are **全部 / 常用 / 应用 / 网页 / 文件**.

- **常用** lists resources with launch history, most recently used first. You can’t manually reorder items in this tab; in the other tabs, press and hold a card to drag it into position.
- The header also has an icon-size menu (small/standard/large/extra large), batch management, three scan-and-import buttons, “修复图标缓存,” and “+ 添加.”

### 6.2 Filter by subcategory

- The “应用 / 网页 / 文件” tabs show a subcategory filter row. “全部” appears first, followed by “未归类” and the available subcategories. A ★ marks the **default subcategory**, which is automatically assigned to new resources unless another is selected.
- Subcategory names support `/`-separated paths (for example, “开发/前端”); imported browser bookmarks retain their folder hierarchy. The filter row shows only top-level subcategories. Hover over or click ▸ to drill down through the cascading menu. **Selecting a subcategory shows its resources and all resources in its descendants.**
- Switching to another main tab resets the filter.

### 6.3 Zones (custom groups in the “全部” tab)

Zones can contain resources from different categories and only group items visually in the “全部” tab; they are independent of subcategories in §6.2. **If no zones have been created, the “全部” tab remains a flat list.**

- **Create a zone**: Click “＋ 分区” on the right of the tab row. A zone is created with a name like “分区 N” and immediately enters rename mode.
- **Rename, reorder, or delete**: Double-click a zone name to rename it. Open “⋯” in its header or right-click it for options: set icon size (overrides the global size for this zone), rename, move up/down, or delete. Deleting a zone moves its resources to “未分区” and can be undone. Press and hold a zone header to drag it into a new order.
- **Resize a zone**: Drag the striped handle in the lower-right corner of the zone. Its size snaps to whole-card increments. The set dimensions are a minimum; the zone grows taller automatically when needed to fit its contents.
- **Move resources into a zone**: Press and hold a card and drag it to the zone, or right-click a resource and choose “移动到分区.” The dashed “未分区” area at the end of the page is a drop target for moving resources out of a zone.

### 6.4 Five ways to add resources

1. **Add manually**: Click “+ 添加” in the upper-right corner. If you’re in a category tab with a subcategory filter selected, the dialog preselects that category and subcategory.
2. **Drag and drop**: Drag an exe / lnk / web shortcut / any file / folder anywhere into the window. When the “释放以添加” overlay appears, release it. Names and icons are extracted automatically from exe and lnk files. The add dialog opens with fields prefilled; click “添加” to save.
3. **Scan installed apps**: Lists apps installed on this device, including Microsoft Store apps. Search, select all or none; already-added apps are dimmed. Click “添加选中（N）.”
4. **Scan the desktop**: Scans only the top level of the desktop. “加入速达只记一条快捷方式，不动原文件.” Optionally select “导入后清理桌面快捷方式” to delete only the corresponding .lnk/.url shortcuts.
5. **Import browser bookmarks**: Supports Chrome / Edge / Brave / Chromium, with a tree for selecting bookmarks. Subcategories are created from the browser’s folder hierarchy by default, and duplicate URLs are skipped.

### 6.5 Edit, delete, and batch manage

- Hover over a card and click “编辑” (pencil) or “删除” (trash), or right-click it. Deleting removes only the entry, not the file on disk, and can be undone.
- **Batch management** (ListChecks icon in the header): Select cards, select or clear all, and delete selected items (confirmation required; the batch can be undone). Combine with filters to narrow the list, select all, and delete a group.

**Fields in the edit dialog**:

| Field | Description |
|---|---|
| Field | Description |
|---|---|
| Type | Local app / website bookmark / file or folder (three chips) |
| Name | Required; up to 80 characters |
| Icon | Emoji or local image (ico/png/jpg/webp). App icons are extracted automatically; website favicons are fetched automatically. |
| Target | App path (for Store apps, `shell:AppsFolder\...`) / URL / file path. Creates a link only; the source file is not changed. |
| Launch arguments | Local apps only, for example `--new-window` |
| Subcategory | Select one, or choose “未归类” |
| Zone | Dropdown (appears only if you’ve created a zone) |
| Description | Up to 120 characters; displayed when you hover over the card |
| Note | The label on the left can be customized (for example, “账号密码”). The value on the right is **masked with dots**; click the eye to reveal it. **Encrypted with system-level protection and stored locally**; other system accounts cannot read it. |

### 6.6 Launch and open options

- **Click a card to launch it.** A small green dot appears beside a running app’s name. Click again to try to bring its existing window to the foreground.
- **Choose how to open a website** temporarily by right-clicking its resource:
  - **在内嵌面板打开**: Open in the browser panel embedded in the main window’s Quick Launch view (see §6.8).
  - **在独立窗口打开**: Open in the in-app browser in a separate window with multiple tabs.
  - **用某浏览器打开**: Choose an installed browser (Chrome/Edge/…).
  - Set the default under **设置 → 功能 → 速达 → “网页默认打开方式”** (embedded panel / separate browser window / system default browser).
- **Run as administrator**: For local apps only. Right-click and select the option, then confirm the UAC prompt (not supported for Store apps).
- **Files and folders**: Click to open using the system default. Right-click “打开文件所在位置” to locate the item in File Explorer.
- Addresses such as `smb://` and `ftp://` can be saved as website resources; clicking them passes them to the operating system.

### 6.7 Manage icons

- Drop or select an exe or lnk to extract a high-resolution app icon automatically. Website favicons are fetched in the background.
- If an icon is blurry or corrupted, click “**修复图标缓存**” in the header to repair all extracted icons. Manually imported icons and favicons are not affected.
- To change an icon manually, use the “图标” field in the edit dialog.

### 6.8 In-app browser

- **Embedded panel** (in the main window): Toolbar buttons for Back / Forward / Reload / address bar (press Enter to navigate; `https` is added automatically) / Open in separate window / Open in system browser / Close. Show or hide the toolbar under 设置 → 功能 → 速达 → “内嵌面板显示工具栏” (hidden by default so the website fills the panel). New-window requests from a website open in the same panel.
- **Separate browser window**: The top bar has a tab strip (shown with 2 or more pages; click × or **middle-click** to close a tab) and a toolbar. New pop-up windows open as new tabs.

---

## 7. AI Assistant

### 7.1 Three modes and how to open them

| Entry point | Behavior |
|---|---|
| Sidebar “AI 助手” | Full view: 260px session list on the left and chat on the right |
| AI button in the title bar / `Ctrl+Shift+K` | Opens a drawer in the main window, or a separate compact window if that mode is enabled |
| Floating ball “AI 对话” | Same behavior as the title-bar button |

- **Embedded drawer** (default): Slides out from the edge of the main window. Set its position under 设置 → AI 助手 → 面板 → “AI 对话面板位置” (right/left/top/bottom). Drag the inner handle to resize it. “AI 对话面板透明度” ranges from 50% to 100%. Click outside the drawer to close it.
- **Separate window**: Enable “以独立窗口打开 AI 对话” under 设置 → AI 助手 → 面板. The compact window has a glass effect, can be dragged by the title bar, and can be kept on top with its pin. Closing it only hides it; it opens instantly next time. The two modes are mutually exclusive.

### 7.2 Configure models

**Option 1: x-hub platform credits (no API key required)**

- Go to 设置 → AI 助手 → 额度 and enable “使用 x-hub 平台额度.” First sign in and redeem an invitation code for credits under 设置 → 账号.
- Once enabled, the model menu in chat has one “**x-hub 平台**” option. You don’t need to choose among the platform’s models: if there is one, it’s always used; if there are multiple, requests are load-balanced among them.
- Your remaining credits appear in the chat header and Settings. If you run out (429), the chat displays a card with options to redeem credits or configure your own API key.

**Option 2: Your own provider (OpenAI-compatible)**

Go to 设置 → AI 助手 → 供应商.

1. Click “添加供应商” to create a card. Use a “快速填充” chip to fill in a preset for DeepSeek / OpenAI / Moonshot / Alibaba Cloud Bailian / Ollama, and more.
2. Enter “供应商名称,” “Base URL,” and “API Key.” (**Keys are stored only in the system keychain, never as plaintext on disk.** The UI masks them; use the eye icon to view, copy, or replace a key.)
3. Click “测试连通” to verify the URL and key. Click “获取模型” to fetch a model list and select models to add in a batch. If a proxy provider doesn’t support listing models, use “手动添加模型” and press Enter.
4. Models appear as chips on each card; you can copy or remove them. **Click “保存配置” to save your changes** (click the trash icon in a card header to delete the entire provider).

The model menu in chat lists models grouped by provider. A status dot appears beside each model; hover over a gray dot to see why it’s unavailable (for example, you’re not signed in to the platform).

### 7.3 Chat controls

- In the input field, **press Enter to send and Shift+Enter for a new line**. The field grows automatically. During generation, the button changes to “停止”; click it to stop the current turn.
- Replies are rendered as Markdown. Code blocks have a language label and “复制” button. Hover over a message to copy it. When you scroll up, “跳到底部” appears.
- At the bottom of each turn, see input/output/cached tokens, cache hit rate, and speed (tok/s).
- The “+” menu lets you create, switch, or delete conversations. It is shared by the separate window and drawer; rename conversations in the left pane of the AI Assistant view.

### 7.4 Agent tools and permissions

x-hub includes an Agent that can use tools to read and write local data, such as checking to-dos, creating notes, and launching apps. It includes 34 built-in tools, plus tools provided by extensions.

- **Tool cards**: When the AI uses a tool, a card appears in the conversation. The colored dot indicates its level (read = green / write = orange / dangerous = red). The card shows the tool name and status (running / awaiting approval / complete / failed / denied); expand it to see arguments and results. The model’s intermediate commentary appears in a collapsible “中间过程” block.
- **Approval**: Cards for actions requiring approval show three buttons: “**允许**” (this time), “**本会话允许**” (don’t ask again for this type of action in the current conversation), and “**拒绝**.” Parameters for dangerous operations are expanded by default to help prevent blind approval.
- **Permission level**: Choose “仅可查看 / 每次都问 / 完全权限” from the quick menu in the input area. Set the default under 设置 → AI 助手 → 助手 → “AI 助手默认权限档.” If you select “完全权限,” you can also enable “危险操作仍需确认” as a safeguard.
- **Allow extension tools**: Enable “允许调用扩展提供的工具” under 设置 → AI 助手 → 助手. To enable or revoke tools for a specific extension, use the “AI 助手” section of that extension’s settings dialog.
- **“关于我”**: Under 设置 → AI 助手 → 助手, enter up to 1,000 characters about your work, preferences, and so on. This context is included in conversations to tailor responses to you.

---

## 8. Clipboard History

In the background, x-hub automatically records copied **text / images / files**, deduplicating identical content.

- **Open it**: Press the global shortcut `` Ctrl+` `` (default). The separate overlay can be moved by dragging its top bar and closes automatically when it loses focus.
- **Use it**:
  - **Click an item (or select it and press Enter) to paste it back** into the previous input location. Use ↑↓ to select and Esc to close.
  - Type in the search field at the top to filter items.
  - Hover over an item for actions: copy, **存为速记** (text), **加入提示词库** (text), or delete.
  - **Right-click** to view an image at full size, **保存图片** (BMP images are converted to PNG automatically), or pin/unpin an item (pinned items are excluded from automatic cleanup).
- The bottom bar has a “暂停记录” toggle (copies are not saved while paused) and “清空历史.”
- **Retention settings**: Go to 设置 → 功能 → 剪贴板 to set the item limit (500 by default; 20–5,000), retention period (7 days by default), and **paste method** (Auto / Ctrl+V / Ctrl+Shift+V / Shift+Insert; Auto or Ctrl+Shift+V is recommended when pasting into a terminal).

---

## 9. Global Search

- **Open it**: Press `Ctrl+K` (default) or click the Search button in the title bar.
- Search across **Quick Launch resources** (launch directly), **Notes** (title and body; opens the note), and **To-dos** (returns to the dashboard and highlights the item).
- Use ↑↓ to select a result, Enter to open it, and Esc to close. Search runs 300ms after you stop typing, with results grouped by type.

---

## 10. Extension Center

Open “扩展中心” at the bottom of the sidebar. It has four tabs: **已安装 / 可升级 / 市场 / 我的扩展**. Use the category filter at the top (AI / productivity / developer tools / multimedia / games / learning / other) and look for the official badge.

### 10.1 Installed

- Cards show an icon, name, version, surface labels, and a green dot for extensions running in the background. Actions include “更新 vX” (when an update is available), “打开所在目录,” ▶ to open the extension, or “查看详情.”
- Set what happens when you click a row under 设置 → 扩展 → “点击扩展行时” (opens details by default; can be changed to open the extension directly).
- Clicking an unavailable, dependency-missing, or disabled extension shows the reason.

### 10.2 Upgradable

Lists installed extensions that have a newer version in the market. A red dot appears on the tab, and updates show a progress bar.

### 10.3 Market

- Cards show official status, category, install count, and update notes. Click “详情” for an expanded view with screenshot lightbox, capabilities, changelog, user feedback board, and sha256; click “安装” to install with progress.
- The header shows “上次更新 + 刷新.” Click “导入扩展包” in the upper-right to install a local `.xhpack` file.
- If the host app is too old, the Install button is disabled and the required version is shown.

### 10.4 My Extensions (for developers)

- Click “选择目录” to mount a local extension source directory containing `manifest.json`. **It loads as soon as it’s registered; code changes reload automatically in about 1.5 seconds.** Files are not copied, and the extension is not included in market updates.
- Row actions: publish (requires developer verification; quotas appear in the Extension Center header: remaining pending submissions, submissions remaining today, and additional published extensions allowed), open the source directory, or remove (unmounts the directory but does not delete source files).
- The publish dialog includes release notes, release version, and screenshots (up to 5, max 2MB each). If the extension already has a published release, **its screenshots from the previous release are added automatically**. Publish without changes to keep them, or remove and reselect to replace them.
- Click “用户反馈” (red dot indicates unread feedback) to open the developer inbox. View contact and diagnostic details, post a public reply, mark feedback as resolved, or hide it.

### 10.5 Open modes and sidebar pins

In the extension details dialog, open the “侧边栏” section:

- **Open mode**: In-app (full page in the main area) / separate window / drawer (slides out 480px from the right). Extensions that declare a module surface can also be added as **cards** to the dashboard layout; they appear in the layout editor’s module library.
- **Pin this extension to the sidebar**: Click its pinned icon to open it using the selected mode. A green dot appears under an extension running in the background. **Right-click** its icon for Refresh page / Restart backend (service extensions) / End background session.
- **Link behavior**: Links in extensions open in the in-app browser by default; you can change this to the system browser.

### 10.6 Permissions and background operation

- In an extension’s settings dialog, open “权限” to see plain-language toggles for capabilities such as reading host data, accessing the network, and running a local backend.
- **Keep running in the background**: Enable “后台运行” in the details dialog or manage extensions under 设置 → 扩展 → “常驻扩展.” The extension keeps running and retains its state when you navigate away. Up to 5 extensions can run in the background. “随应用启动” starts the extension in the background whenever x-hub opens (and implicitly keeps it running). Under 设置 → 扩展, “自动信任 service 扩展” skips per-extension authorization for local backends of service extensions (a security warning is shown).

### 10.7 Title-bar widgets

An extension with the required capability that is running in the background can add **media controls** (cover art/title/previous · play · next) or an **info bar** to the main window’s title bar. Configure its position (left/center/right) and border under **设置 → 外观**. Button clicks are forwarded to the extension.

### 10.8 Feedback

You can submit feedback from the market details, extension settings, or extension error view. Choose a type (bug / suggestion / question), enter up to 500 characters, optionally provide contact information (visible only to the developer), and optionally include diagnostics (app version and system information only; no notes, account details, or paths). **Feedback is posted publicly**; developers can reply in their inbox.

### 10.9 Skills (extension development skill)

Go to 设置 → 扩展 → Skills and install the built-in “x-hub 扩展开发技能包” in one click to the skills folder of a local AI coding assistant (Claude Code / ZCode / Codex / 豆包, etc.). Existing folders are detected automatically; you can also choose a custom folder. After installation, ask your assistant “用 x-hub-extension 给我做一个 XX 扩展” to build an extension from scratch.

---

## 11. Floating Ball and System Tray

### Floating ball

Configure the floating ball under **设置 → 常规 → 悬浮球**:

- “桌面悬浮球” master toggle: By default, the ball appears on the desktop when the main window is hidden or minimized.
- Controls: **Single-click** to open the radial shortcut menu (click an empty area to close it); **double-click** to show the main window; **right-click** to open the same menu as the system tray; **press and drag** to move the ball.
- “悬浮球贴边自动隐藏”: After you drag the ball to a screen edge and release it, only half remains visible. Hover over it to slide it fully into view.
- “与主窗口同时显示”: Keep the ball on the desktop while the main window is open.
- “静止时保持转动”: Turn this off to pause the spinning animation when the ball is idle and save power.
- **Radial menu buttons**: Choose and reorder up to 8 from Dashboard / Notes / Quick Launch / AI Chat / Extension Center / Settings / Global Search / Clipboard / New Note / Main Window. Move items up/down or remove them.

### System tray

Left-click the tray icon to show or hide the main window. Right-click for the menu: show the main window / hide the main window / restart / exit.

---

## 12. Settings at a Glance

Open Settings at the **bottom-left of the sidebar**. Use the **settings search box** at the top of the left pane to search for a keyword (for example, “壁纸” or “快捷键”). Click a result to open the relevant panel and highlight the setting.

Navigation (click a category to expand its indented sections):

| Category | Sections |
|---|---|
| General | General / Floating Ball / Shortcuts |
| Appearance | Appearance |
| Dashboard | Dashboard |
| AI Assistant | Assistant / Panel / Credits / Providers |
| Features | Quick Launch / Notes / Clipboard / Internet / Performance |
| Extensions | Extensions / Skills |
| Account | Account |
| Data and About | Data / About |

### 12.1 General

- **Start at login** (toggle, off by default): Start x-hub in the system tray after you sign in to Windows. If Windows or security software disables it, a yellow warning appears here with a one-click “重新启用” action.
- **Notification duration**: Number of seconds a notification card stays in the lower-right corner (1–60; default 5). Click a notification to dismiss it immediately.
- See §11 for the floating ball and §2 for shortcuts.

### 12.2 Appearance

- **Expand sidebar** (off by default): Adds an expand/collapse button at the bottom of the sidebar (220px labeled mode ↔ 56px icon mode).
- **Title-bar widgets**: Toggle the border; choose a position from the dropdown (shown only while a widget is active, with one row per widget: left/center/right).
- **Theme mode**: Light / Dark / Follow system.
- **Theme presets**: 10 solid colors (Indigo, Green, Morandi, Midnight Blue, Rose, Amber, Teal, Violet, Sky Blue, Graphite) and 10 gradients (Starry Night, Sunset, Aurora, Rose, Forest, Gilded, Deep Sea, Grape, Fireworks, Graphite).
- **Accent color**: Choose from 8 preset swatches or use the color picker. “重置” appears after setting a custom color.
- **Wallpaper**: Import with “选择图片 / 更换图片” (png/jpg/webp/bmp, up to 30MB; animation is not supported). Click “清除壁纸” to restore the gradient background. Related settings:
  - **Wallpaper overlay** (0–85%): Applies a tint based on the current theme to improve text contrast over the wallpaper.
  - **Immersive mode**: Uses true frosted glass for cards and reveals the wallpaper clearly through gaps between them (may cause frame drops on low-end integrated graphics).
  - **Background blur**: Softens the entire wallpaper (mutually exclusive with immersive mode).
  - If the wallpaper file is deleted outside the app, x-hub automatically falls back to the gradient background.
- **Card glass opacity** (40–100%): Controls transparency across all cards.
- **Font size**: Global slider (85–130%) and per-module sliders (Sticky Notes / Notes / Prompts / To-dos).

### 12.3 Dashboard

- **Custom layout**: “打开编辑器” and “恢复默认布局” buttons (see §3.1).
- **Countdown sound** (off by default).
- **Clock card quote** (up to 50 characters; leave blank to show a random quote) and **quote source** (online/local).

### 12.4 AI Assistant

- **Assistant**: Default permission level / require approval for dangerous operations / allow extension tools / “关于我” (see §7.4).
- **Panel**: Separate-window toggle / panel opacity / panel position (see §7.1).
- **Credits**: Platform credits toggle and balance (see §7.2).
- **Providers**: Manage your own models (see §7.2).

### 12.5 Features

- **Quick Launch**: Default website open mode / show the embedded-panel toolbar / **subcategory management** (rename inline, reorder by dragging, set the default with the star, use `/` in names for nesting, confirm deletions in two steps, and delete in a batch).
- **Notes**: AI refinement model (two-level dropdown; see §5.9).
- **Clipboard**: Item limit / retention period / paste method / pause recording / clear history (see §8).
- **Internet**: “联网功能” master toggle for weather and online quotes; **weather city** (enter a city and click “设置,” or use “自动定位”).
- **Performance**: “隐藏窗口时降低内存占用” releases memory while windows (chat, clipboard, notifications, browser, etc.) are hidden and restores it before they are shown.

### 12.6 Extensions

- **service runtime strategy**: Detect automatically (recommended) / always use the bundled runtime / always use the system runtime.
- **When clicking an extension row**: Open details / open directly.
- **Automatically trust service extensions** (off by default; includes a security warning).
- **Background extensions**: Manage extensions enabled for background operation (Keep running / Start with app).
- **Skills**: See §10.9.

### 12.7 Account

> Sign-in is only required to apply to become an extension developer and to publish extensions. You don’t need an account to install extensions or chat using your own API key.

- **Sign in with GitHub**: Device-code flow—copy the code and authorize in your browser. The result appears here.
- **Sign in with an email code**: Enter your email to receive a code (60-second resend cooldown; limit of 5 emails per hour per address), then enter the 6-digit code.
- **Redeem an invitation code**: Receive AI credits and become eligible to apply as a developer.
- **Extension developer**: Check your status or submit an application (reason must be at least 10 characters).
- **My devices**: Up to 5 devices can be signed in to the same account. Remotely “撤销” other devices, or “退出登录” to sign out this device.

### 12.8 Data and About

See §13 for the “数据” section and §14 for “关于.”

---

## 13. Data and Maintenance

Find all options under **设置 → 数据与关于 → 数据**:

| Action | Description |
|---|---|
| **Data location** | Shows the current location (read-only). Click “更改,” choose a new folder, and confirm the move. x-hub restarts automatically to apply it (not available in Portable edition). |
| **Back up data** | Choose a destination folder to package the database and icons into a zip file. |
| **Restore data** | Select a backup zip. The button requires two-step confirmation: click once to show “确认恢复?” and again within 3 seconds to proceed. Restart x-hub to apply the restore. |
| **Recycle Bin retention** | Keep items forever (default), or for 7 days / 30 days / 90 days / 1 year. Expired items are removed automatically. |
| **Clean up orphaned images** | Click “扫描” to find images not referenced by any note, then “清理 N 张” to delete them. Notes in the Recycle Bin still count as references, so restoring one later won’t leave it without its images. |
| **Export notes** | Export the entire library as Markdown organized by folder, including front matter and images. |
| **Import notes** | Imports only folders exported by x-hub. Notes with the same source are skipped by default. A summary shows the number imported, skipped, and failed. |

By default, all data is stored locally and is not uploaded to the cloud.

---

## 14. App Updates

- **Automatic checks**: x-hub checks silently every 4 hours after startup. The master toggle is under 设置 → 数据与关于 → 关于 → “自动检查更新” (on by default). Manual “检查更新” is available regardless of the toggle and ignores any “跳过此版本” setting.
- **New-version dialog**: Shows the version, package size, and full release notes, with three actions:
  - “**立即更新**”: Downloads in the background with resume support and automatic retries. Progress is shown in real time; closing the dialog does not stop the download.
  - “**稍后再提示**”: Snoozes notifications for 30 minutes, then automatically checks again.
  - “**跳过此版本**”: Don’t notify you about this version again.
- **Update ready**: Click “立即重启” to finish the update (automatic replacement with rollback on failure). If you don’t restart now, installation completes the next time you start x-hub.

---

## 15. FAQ

**Q: I clicked Close, but the app is still running in the background.**
A: That’s expected: Close minimizes x-hub to the tray. To exit completely, right-click the tray icon and choose “退出.”

**Q: A global shortcut doesn’t do anything.**
A: Check these in order: ① Is it disabled under 设置 → 常规 → 快捷键 (toggle off)? ② Does another app use the same shortcut? Try a different combination. ③ If changing a shortcut shows “与『搜索』快捷键键值相同,” another x-hub shortcut is assigned the same keys.

**Q: The weather isn’t showing.**
A: Make sure “联网功能” is enabled under 设置 → 功能 → 联网 and a city is set under “天气城市.” Online content is hidden while offline.

**Q: A countdown isn’t running or didn’t notify me.**
A: When the Countdown card is removed from the dashboard layout, countdowns without floating windows pause automatically; those in floating windows are unaffected. Add the card back to resume them. Sound is off by default; enable it under 设置 → 工作台. Reminders missed while x-hub is closed or the computer is asleep aren’t sent later.

**Q: The AI says “平台额度未开启” or that I don’t have enough credits.**
A: To use platform models, enable the option under 设置 → AI 助手 → 额度, then sign in and redeem an invitation code under 设置 → 账号. If you’re out of credits, choose “去兑换积分” in the conversation or switch to your own API key (provider).

**Q: An extension page is blank.**
A: This can happen when the renderer is reclaimed after the window has been in the background for a while. Switch to another sidebar view and back, or right-click the extension’s sidebar icon and choose “刷新页面.” For service extensions, try “重启后端.”

**Q: I can’t paste an image into a note.**
A: Supported formats are png/jpg/jpeg/webp/bmp/gif, up to 10MB per image.

**Q: An app icon in Quick Launch is blurry or looks corrupted.**
A: Click “修复图标缓存” in the Quick Launch header. The full repair takes a few seconds to a few dozen seconds.

**Q: I copied note content elsewhere, but the image didn’t come with it.**
A: Use “复制图片” from §5.7 (lightbox button / right-click / Ctrl+C). This copies the actual bitmap, which you can paste directly into a chat or document.

**Q: What should I do with my data when switching computers or reinstalling Windows?**
A: First export a zip under 设置 → 数据与关于 → 数据 → “备份.” After installing x-hub on the new device, use “恢复” to import the backup, or copy the entire data folder (§1.3) to the same location.

**Q: Do Sticky Notes, To-dos, and Prompts have separate windows?**
A: Yes. Use “脱离为悬浮便签” on a Sticky Note card, “待办浮窗” at the upper-right of the To-do card, or “提示词浮窗” in the Prompt card header. Floating windows can be moved and kept on top; their positions and open/closed state are remembered.

**Q: A website (such as a bank or intranet site) won’t open.**
A: The in-app browser has limitations with some websites. Right-click the resource and choose “用系统浏览器打开,” or click “用系统浏览器打开” in the panel toolbar.
