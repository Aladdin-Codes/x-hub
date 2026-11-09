# Validation, In-App Testing, and Publishing

> **When to read this:** After writing the extension code, when running it to check the results, preparing a delivery checklist, or when the user asks about publishing.

## You can complete the entire workflow without scaffolding

The scaffolding repository (`npm run new/validate/preview/deploy/pack`) is **available only in the internal development environment**; external developers don't have it. This skill includes `templates/`, and the host can mount a local source directory, so the complete workflow is still available:

| Step | Without scaffolding (default) | With scaffolding (internal development environment) |
|---|---|---|
| Scaffold | Copy and edit files from this skill's `templates/` | `npm run new` |
| Validate | Checklist below + platform checks + **test in the actual app** (host scanner errors are authoritative) | `npm run validate` |
| Local preview | Open entry HTML directly in a browser (no bridge, uses fallback branches and theme colors) | `npm run preview` (mock bridge + injected theme) |
| Load in host | Mount from **My Extensions** (recommended: code changes auto-reload in 1.5 seconds) | `npm run deploy` |
| Package for distribution | **Publish** in Extension Center (packages automatically) or **Import Extension Package** | `npm run pack` |

**For a browser preview to match the host**, use the dual-declaration fallbacks from `theming.md`, and use the boot-mode storage pattern from `convert-html.md` (prefer the bridge and fall back to localStorage when `typeof window.xhub === 'undefined'`). Both patterns are designed to work without the host.

## Recommended: Mount from the Host's “My Extensions” Tab

1. Extension Center → **My Extensions** tab → add a local extension source directory (must contain `manifest.json`).
2. **Adding it loads it immediately; no toggle is needed.** Open the extension to test it in the actual app. The directory remains the source of truth (**it isn't copied into the installed list**); **code changes auto-reload in about 1.5 seconds**, with access to real data and the service backend. Remove the directory to unregister it. Use **Publish** on the right to submit it to the market (developer verification is required).

**Always use this for day-to-day debugging**; it's much faster than repeatedly deploying. Once the extension is running, the capability list returned by `await window.xhub.runtime.info()` is the final authority on whether an API is available.

> **If a service extension's backend doesn't start:** First check `<data-root>\logs\service\<extension-id>.log` (backend stdout/stderr written by the host, including exit code and original errors), then check `service backend not ready: … exit=…` in `logs\x-hub.log`. `serviceReady=false` does not mean “Node is downloading”; don't guess. See the “Troubleshooting” and “Where to store data and logs” sections of `service.md`.

**Restart the backend process after changing service backend code**: The roughly 1.5-second auto-reload only reloads the page (navigates the iframe); the Node backend process is reused by extension ID, so refreshing the page won't load new backend logic. For a persistent instance, **right-click the extension icon in the pinned sidebar area → Restart backend** (stops and starts the Node process; the port is reassigned, but the extension page does not need to be refreshed). If persistent mode is off, restart x-hub to restart all backends. Frontend HTML/JS changes still use auto-reload as usual.

## Pre-Submission Checklist

- [ ] `manifest.json` is valid JSON, all required fields are present, and every HTML file referenced by `entry` exists.
- [ ] All local JS / CSS / image paths referenced by the entry HTML exist (**no external CDN dependencies**).
- [ ] All colors use `var(--xhub-*, fallback)`; check both light and dark themes.
- [ ] Bridge calls use `try/catch`; if rejected (`PERMISSION_DENIED`), show a clear message instead of a blank screen.
- [ ] **Use `var(--xhub-page-bg, transparent)` for the page background** (no wallpaper = host page background; wallpaper = transparent) and `var(--xhub-surface)` for content surfaces. `module` cards should also be transparent.
- [ ] If declaring `module`, make sure the entry lays out correctly in the **smallest cell**: the card iframe viewport equals the content area (subtract about 30px if the host header is enabled), and may be as short as ~86px (see item 22 in `pitfalls.md`).
- [ ] If declaring `module`, **don't duplicate the host header title**: extension cards have no host header by default, so draw the title yourself. The host header is added only with `moduleOptions.defaultHideTitle: false` (then the card name is `manifest.name`; don't draw it again—see item 23).
- [ ] In transparent wallpaper mode (`data-xhub-wallpaper-clear="1"`), remove the white border and add a soft black text glow.

## Platform Checks

When the client publishes an extension, it runs **local preflight checks** that match the server checks. **Errors block publishing; warnings are informational.**

| Check | Level | Details |
|---|---|---|
| `manifest.json` parses | error | Invalid JSON syntax / field type |
| Valid `id` | error | See the format rules in `manifest.md` (lowercase reverse-domain name) |
| `id` uses `com.x-hub.*` | warn | Reserved platform namespace; non-official accounts are rejected by the **server** |
| `version` is `x.y.z` | error | Three numeric components; values such as `1.0.0-beta` are invalid |
| All entry files exist | error | Every path in `entry` must point to a real file (the error lists `surface → path`) |
| No entry declared | warn | At least one surface needs an entry HTML file |
| Service backend entry exists | error | The file referenced by `backend.entry` must exist |
| External listening without `network` | error | When `backend.host` is not loopback |
| Uses a bridge API not implemented by the host | warn | Based on `runtime.info().capabilities` |
| **Code uses a permission not declared in the manifest** | **error** | See below |

**The last item is the easiest to miss**: preflight **statically scans** `.html/.js/.mjs/.cjs` files in the extension directory (skipping dot-prefixed files and `node_modules`), extracts every `xhub.…` call chain, infers the required permission, and compares it with `manifest.permissions`. Rules:

- In `data.*`, methods beginning with `create/update/delete/set/toggle/reorder/import/schedule` require `data:write`; all others require `data:read`.
- `ui.*` → `notify` | `net.*` → `network` | `system.*` → `system` | `clipboard.*` → `clipboard` | `fs.*` → `fs` | `sharedStorage.*` → `shared-storage` | `titlebar.*` → `titlebar` | `events.emit` → `events`
- **Top-level `xhub.openExternal(...)` → `open-url`**; other `runtime.*` methods require no permission.
- `runtime.*` (except `openExternal`) / `storage.*` / `config.*` / `theme.*` / `service.*` / `expose` → no permission required.

⚠️ Scanning is **plain-text matching**, so a call such as `xhub.data.notes.create(...)` in a **comment, string, or code example also counts as “used”**—declare the permission or avoid putting that call in comments.

Conversely, a declared permission with no matching call in the code triggers a warning (`network` cannot be statically detected). Remove permissions you don't use to make review easier.

## Deployment and Packaging

- Restart x-hub (or refresh Extension Center) after deployment to see the extension.
- For the `module` surface, go to Settings → Workbench → Custom Layout and drag the module into the grid.
- Packages are `.xhpack` files (ZIP format; `--zip` can output `.zip`; `manifest.json` must be at the archive root or one directory below it). You can also install locally with **Import Extension Package** in Extension Center.

## Publishing to the Market

**Don't edit the market listing manually**: In Extension Center, click **Publish** on an extension under development → client preflight (table above) → package and upload → platform **automated checks** (archive structure / manifest / entry points / permission declarations / static scan / version increase) → **manual review** → signed market release. Installed users receive the update automatically. If rejected, the client shows itemized reasons under **My Submissions**.

- **You don't need to edit the manifest version manually**: The Publish dialog's “Release version” defaults to the current patch version +1. When you publish, the client writes that version into the extension's `manifest.json` before packaging (it must be higher than the current version; leave it blank to submit the version already in the manifest, which is useful when resubmitting the same version after a platform-check issue). Edit code → click Publish → submit; no manual file edits are needed.
- You can **upload screenshots** when publishing (up to 5, max 2 MB each, PNG/JPG/WebP); they appear on the extension details page. One to three screenshots are recommended (main screen + typical usage). If the extension already has a released version, its screenshots are **automatically included from the previous release** when the dialog opens. Publish as-is if the UI hasn't changed, or remove and replace them.
- Publishing requires **publisher status**: Sign in under Settings → Account → redeem an invitation code → apply to become a developer. After approval, extensions under development will have a **Publish** button.
- **User feedback goes through the host feedback board**: After release, users can submit feedback from market details, the extension settings dialog, or an extension error state (bug / idea / question, optionally with contact details and diagnostics). Developers can open **Feedback Inbox** in Extension Center to review unread feedback by extension, reply inline, and mark items resolved. Publishing isn't the end—check in and don't let user feedback go unanswered.

## Optional: Scaffolding Repository (Internal Development Environment Only)

External developers can skip this section. The following commands show the available conveniences for reference:

Generate a scaffold (optional; this skill already includes `templates/`):

```bash
npm run new                        # Interactive (prompts for an id)
npm run new -- --name Weather Card --surfaces module,view --id com.yourname.weather \
  --desc "Shows current weather on the workbench" --runtime web --dir ./weather
# ⚠️ --id is required and must be your own reverse-domain name. com.x-hub.* is reserved for official first-party extensions.
#    The scaffold command rejects it (platform staff must explicitly pass --official).
# Multiple module variants: --variants compact:Compact:2x2:2x2,month:Month:4x4:5x5
#   Format: id:name[:minWxminH:idealWxidealH]. Defaults are 2x2 / 4x3; generates moduleVariants in the manifest.
```

Validate, preview, deploy, and package:

```bash
npm run preview -- <extension-dir>     # Local preview: mock bridge + injected theme; no host required
npm run validate -- <extension-dir>    # Validate manifest, entries, and local resource references
npm run deploy -- <extension-dir>      # Install to %APPDATA%\x-hub\extensions\<id>\
npm run pack -- <extension-dir>        # Package dist/<id>-<version>.xhpack
```

> ⚠️ The npm script must be named `deploy`, **not** `install`: `install` is an npm lifecycle hook and may be triggered unintentionally by `npm install`.
>
> ⚠️ Before deploying a service extension, quit x-hub if it's running and has the extension's backend files locked; otherwise `deploy` may fail with **EPERM**.

**Preview notes**: Lists all entry URLs; injects a mock bridge (`storage` / `config` / `theme` / `events` / `runtime.info`) and all `--xhub-*` variables; use `?xhub-theme=light|dark` to switch themes and `?xhub-variant=<id>` to switch module variants. Mock data is stored in browser localStorage (not shared with the host), `data.*` returns empty arrays, and `service.request` rejects—**verify these behaviors in the host app before release**.
