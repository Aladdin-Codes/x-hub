# Converting an Existing HTML Page into an Extension

> **When to read this:** The user provides an existing webpage, single-file tool, or `.html` file and wants to turn it into an x-hub extension.

This is the most common conversion path. Follow these four steps in order.

## 1. Remove CDN dependencies (required)

The host webview may be restricted by CSP or offline, so the entry point **must not depend on external resources**:

- **Tailwind CDN** → identify the utility classes the page actually uses and write local replacement CSS (usually under 30 lines, including `sm:` / `md:` / `lg:` media queries).
  Remember Tailwind preflight resets: `body{margin:0}`, `h1..p{margin:0}`, reset `button`, `*{box-sizing:border-box}`, and `input,textarea{font-family:inherit}`.
- **Font Awesome and similar icon libraries** → use inline SVG (`<svg viewBox="0 0 24 24" fill="none" stroke="currentColor">`; set `svg{width:1em;height:1em}` in CSS to scale with the font).
  If the template has many icons, you can retain `fa-*` class names and use a MutationObserver to replace `<i class="fas fa-xxx">` with same-size SVG (`el.innerHTML = svg`, adding a `data-*` marker to prevent re-entry), instead of editing each template location.
- **Google Fonts** → remove them and use system fonts (for example, add a fallback stack such as `"Songti SC", SimSun, serif`).

## 2. Map the theme

Map the page's design tokens to `--xhub-*` using the “dual declaration fallback” approach in `theming.md`.

**Remove the page's own light/dark theme toggle and its JS**—the host owns the theme.

## 3. Migrate storage

Change synchronous `localStorage` reads to asynchronous `xhub.storage` reads. Use **boot mode**:

```js
// Read saved progress/data first, then buildNav / render the initial route—don't read synchronously at the top level
const saved = await window.xhub.storage.get('progress')
render(saved)
```

When saving, **prefer the bridge** and fall back to localStorage when `typeof window.xhub === 'undefined'` (standalone preview without the bridge):

```js
async function save(v) {
  if (window.xhub) await window.xhub.storage.set('progress', v)
  else localStorage.setItem('progress', JSON.stringify(v))
}
```

## 4. Split multiple surfaces (optional)

If a module summary card and the view homepage share data, extract it into `assets/data.js` (`window.XXX = {...}` on the global object; put pure calculations there too). Both entry points can share it with `<script src="../assets/data.js"></script>`, avoiding duplicated data.

## 5. External links (usually no changes needed)

Keep page `<a href="https://…">` links **as they are**: the host bridge intercepts external-link clicks and opens them according to the user's “Link opening behavior” setting (the in-app browser window by default). Do not change them to `target="_blank"` / `window.open` (they silently fail in the host); do not attach custom click logic to external-link `<a>` elements (the host takes over and swallows it). For custom click behavior, use a button and `xhub.openExternal(url)` (remember to declare the `open-url` permission in the manifest).

## Verify the conversion

Start a local preview (see `debug-deploy.md`), test interactions such as checking boxes, saving, and searching in a browser, then test in the actual host app.
