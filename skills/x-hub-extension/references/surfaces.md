# Surfaces and Multiple Workbench Module Variants

> **When to read this:** When choosing how an extension should appear or implementing a `module` entry point.

## Choosing among the four surfaces

| Surface | Use case | Container |
|---|---|---|
| `module` | Persistent workbench summary card | A cell in the host workbench grid. **The host wraps it in a glass card**; keep the extension page transparent. |
| `view` | Full tool page | Main content area (opened from sidebar navigation) |
| `window` | Side-by-side reference / dedicated workspace | Standalone floating window |
| `drawer` | Quick reference / use alongside current work | Slide-in panel; the host drawer already uses real backdrop blur and a glass background when wallpaper is active |

- **`window` / `drawer` share an entry point with `view`**: point `entry.window` directly to `./view/index.html`; don't duplicate the page.
- `module` cells are small, so design compact cards (a large number, one or two lines of explanation, and optionally one button).
- One extension can declare multiple surfaces, sharing the same `manifest.id` and `storage`.

## Persistent background for `view` surfaces (v0.9.4+)

By default, a `view` extension **stops when you leave it**: the host unloads its iframe when the user switches views, stopping timers, polling, and playback. It reloads when they return (and any partially completed form is lost).

In extension settings, users can enable **Keep running in background** (up to 5 persistent instances globally; each runs in its own renderer process) and **Start with app** (starts in the background when x-hub launches; this also implies persistent background operation). A running persistent instance has a **green dot** in the pinned sidebar area. Right-click it to **Refresh page** (reloads only the iframe), **Restart backend** (service extensions only: restarts the Node process; use this after changing backend code), or **Stop** (ends this persistent session without changing the user's settings).

**When to suggest persistent background operation:** For extensions that continuously maintain a **title-bar widget** (`titlebar.*` is displayed only for persistent instances; calls from non-persistent instances are silently ignored), poll or send reminders, or play audio. **Don't use persistent background operation as a backend**—put heavy work, scheduled jobs, and headless logic in the service runtime. Persistent mode only means “keep the page open.”

## Multiple workbench module variants (`variant`)

An extension that declares `manifest.moduleVariants` renders different content from **the same module HTML entry point**, depending on the current variant. There are three ways to read the variant:

### 1. URL query (initial render)

```js
const variant = new URLSearchParams(location.search).get('xhub-variant')
```

Available as soon as the entry point loads (the host appends it to the iframe `src` dynamically).

### 2. CSS / DOM attribute (recommended for CSS-only branching)

The host sets the current variant in the `<html>` element's `data-xhub-variant="<id>"` attribute and the `--xhub-variant` CSS variable. It updates immediately when the variant changes:

```css
:root[data-xhub-variant="compact"] .month-grid { display: none; }
:root[data-xhub-variant="month"] .today { display: block; }
```

⚠️ Branch using **attribute selectors only**. CSS does not support expressions like `var(--xhub-variant) === 'compact' ? … : …`; that kind of conditional expression belongs in JS.

### 3. Event (switch at runtime without reloading)

When the user clicks ⇄ to switch variants in the workbench editor, the host broadcasts `xhub:variant-changed`. The extension can subscribe and redraw:

```js
window.xhub.events.on('xhub:variant-changed', (variant) => render(variant))
```

### Entry-point conventions

By default, make module entries **adapt to multiple variants**: size to the current cell (using container-query units or `container-type`) and let the variant control content density and structure.

⚠️ **The viewport is the card content area (minus the host header, if present), and may be very short**: the host sets the iframe height to exactly the content area's height (a host bug in 0.6.3 once left it at the browser default of 150px and clipped short cards; that was fixed). **Extension cards have no host header by default**; only when the author sets `moduleOptions.defaultHideTitle: false` is a header of about 30px added at the top (users can also toggle this per card in the editor). Workbench card height is determined by the grid (minimum row height 36px); a 4×2 small cell in a narrow window may have only about 86px of content height, **even less if a header is enabled**. Don't assume the entry is at least 150px tall or set a fixed `min-height`; use `height: 100%` + `box-sizing: border-box` + `cqh` / `clamp()` for layout, and handle overflow with `overflow: hidden/auto`.

### Module card padding

**The host does not add padding to the iframe**—the card content area is the iframe viewport, so the extension controls all whitespace. Conventions:

- `body { padding: 12px; box-sizing: border-box }` (as in the template). The 12px matches built-in module cards so they align visually; `box-sizing: border-box` is required or `padding + height: 100%` will cause a scrollbar.
- When a header is present, it already has 12px of top padding, and the iframe begins **below** the header—so **don't add more top padding** to “clear the header.”
- In short cells, use `clamp()` / `cqh` to reduce font sizes and spacing rather than removing all padding (zero padding makes normal-sized cells look cramped).
