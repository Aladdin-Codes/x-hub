# Theme Integration and `--xhub-*` Variables

> **When to read this:** Before writing any CSS. Nearly all of an extension's **visual correctness** depends on this guide, and it covers the most common pitfalls.

The host injects all theme variables when loading the entry HTML and updates them immediately when the theme or accent color changes. Variable names are the camelCase fields from `theme.get().tokens` converted to kebab-case (`bgPage` → `--xhub-bg-page`).

## Variable reference

| CSS variable | Meaning |
|---|---|
| `--xhub-page-bg` | **Recommended page background for extensions**: the host's full-page background when there is no wallpaper; `transparent` when wallpaper is active |
| `--xhub-bg-page` | The host's **full-page background** (a gradient, **opaque with alpha=1**). ⚠️ **Don't use this as the extension page background**—with wallpaper it fully covers the wallpaper, and with transparent wallpaper the text is white, resulting in “white text on white” (known issue). It's also a gradient, so **it cannot be used as a `color:` value**. Use `--xhub-page-bg` for the page background. |
| `--xhub-bg-card` | Card background (~0.88 white in light mode; becomes `rgba(255,255,255,.2)` **when transparent over wallpaper**)—use as a “well” background for buttons and inputs |
| `--xhub-surface` | Glass surface (gradient, **includes the glass-opacity multiplier**)—**use it for extension cards and panels** |
| `--xhub-text-1` / `--xhub-text-2` / `--xhub-text-3` | Primary / secondary / muted text |
| `--xhub-border` | Border |
| `--xhub-accent` | Accent color (buttons, links, highlights, numbers) |
| `--xhub-brand` / `--xhub-brand-soft` | Primary brand color / soft brand color (translucent background) |
| `--xhub-green` / `--xhub-red` / `--xhub-yellow` / `--xhub-blue` / `--xhub-orange` | Semantic colors |
| `--xhub-radius-lg` | Large corner radius |

**Always write colors as `var(--xhub-*, fallback)`**—the fallback covers the first frame and previews without the host.

## Detecting light and dark themes

The host maintains `data-xhub-theme="light|dark"` on the entry point's `<html>` element. Use it directly in CSS:

```css
:root[data-xhub-theme="dark"] { … }
```

No JS is needed.

## Wallpaper state

The host reflects wallpaper state in three `<html>` attributes (set by the bridge script's `applyTheme`):

| Attribute | Meaning |
|---|---|
| `data-xhub-wallpaper="1"` | The user has set a wallpaper |
| `data-xhub-wallpaper-clear="1"` | **Actually transparent over the wallpaper** (glass opacity < 0.9 or immersive mode). The host turns **all** `--xhub-text-*` values white and darkens the wallpaper veil (white text on a dark background). |
| `data-xhub-immersive="1"` | Immersive mode |

**For transparent wallpaper mode, follow the host `.card` treatment**: remove the white border and keep only a neutral shadow (a 55% white border on a photo looks like a “chalk-white frame”); add a subtle black text glow. In a light theme, `--xhub-accent` is dark and may have low contrast on a darkened wallpaper, so use a `--xhub-brand-soft` background with primary text for small accent elements such as operators and icons.

## Rule: never use an opaque page background

The host's `view` / `window` container chain (`.view-extension` → `.extension-view` → `iframe[background:transparent]`) is transparent and **expects the extension to draw its own background**—if the extension draws an opaque background, the wallpaper is fully covered.

Use the dedicated page token. One declaration works for both states:

```css
body { background: var(--xhub-page-bg, transparent); }  /* No wallpaper = host page background; wallpaper = transparent */
```

Use the `--xhub-surface` glass token for content surfaces. `module` cards are transparent too (the host already wraps them in a glass card). For `drawer` surfaces, the host drawer already has real backdrop blur and a glass background when wallpaper is active, so use the same pattern.

## Pages with their own design tokens: dual-declaration fallbacks

When converting a page with its own tokens, map them to host variables. Set light-theme fallbacks in `:root` and dark-theme fallbacks in `:root[data-xhub-theme="dark"]`. When host variables exist, both blocks resolve to host values; standalone previews without the bridge use their respective fallbacks.

```css
:root {
  color-scheme: light;
  /* Use --xhub-page-bg for the page background (automatically transparent with wallpaper); use the host glass token for surfaces */
  --surface: var(--xhub-surface, oklch(0.97 0.005 40));
  --well: var(--xhub-bg-card, oklch(1 0 0));
  --ink: var(--xhub-text-1, oklch(0.24 0.014 40));
  --primary: var(--xhub-accent, oklch(0.51 0.142 40));
}
:root[data-xhub-theme="dark"] {
  color-scheme: dark;
  --surface: var(--xhub-surface, oklch(0.25 0.01 45));
  --well: var(--xhub-bg-card, oklch(0.20 0.01 45));
  --ink: var(--xhub-text-1, oklch(0.93 0.01 85));
  --primary: var(--xhub-accent, oklch(0.70 0.125 45));
}
```

A page with only one set of fallbacks (no dark block) **looks correct in the host**, but its dark-mode standalone preview will be wrong. Derived accent tokens (`--primary` / `--primary-text` / `--primary-soft`) are the easiest to miss.
