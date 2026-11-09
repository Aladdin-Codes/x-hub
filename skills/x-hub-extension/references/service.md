# Writing a Service Backend (runtime: "service" only)

> **When to read this:** When an extension needs a backend, AI, a platform API, or native capabilities.

## Minimal backend

Dependency-free Node server listening on the host-provided `PORT`:

```js
// service/index.js
const http = require('http')
const port = parseInt(process.env.PORT || '0', 10)
const server = http.createServer((req, res) => {
  res.setHeader('Content-Type', 'application/json; charset=utf-8')
  if (req.url === '/healthz') { res.end(JSON.stringify({ ok: true })); return }
  res.end(JSON.stringify({ message: 'hi', extId: process.env.XHUB_EXT_ID }))
})
server.listen(port, '127.0.0.1', () => console.log('listening', server.address().port))
```

## Non-negotiable rules

1. **Listen only on `127.0.0.1`** and read the port from `process.env.PORT` (dynamically assigned by the host; set `manifest.backend.port` to `0`).
   - `manifest.backend.host` defaults to loopback; **leave it unset**.
   - If you truly need to expose a service (`0.0.0.0` / LAN address), you **must** declare the `network` permission or the host will refuse to start the backend. Users will be asked to authorize such extensions, so avoid this unless necessary. The host also passes the actual listen host to the backend in the `XHUB_LISTEN_HOST` environment variable.
2. **Call the backend from the frontend with `window.xhub.service.request('/api/x')`**; do **not** fetch the port directly—the host reverse-proxies `/svc/<extId>/*` and handles CORS.
3. **Ship backend code only; don't rely on local `node_modules`** (the host manages the runtime). A heavyweight app that must bundle its own complete runtime is an exception and needs an explanation.
4. Provide a `health` path (such as `/healthz`) so the host can check backend health.

> The backend does **not** need any permission to make external HTTP requests (scraping or calling third-party APIs)—that is handled by the Node process. The permission system governs bridge calls between the extension and host, and listening on an externally accessible address.

## AI tool endpoint (required when `agent.tools` is declared in the manifest)

The host forwards AI assistant calls to the extension's fixed endpoint, `POST /xhub/agent-tool`:
the body is `{"tool": "<tool-name>", "args": {...}}`, and the `X-XHub-Ext-Id` request header contains the extension ID (for log tracing).
The **body of a 2xx response (any JSON) is the tool result**, passed verbatim to the model. For failures, return a non-2xx response and put the reason in the body's `error` or `message` field. Response bodies are limited to 256 KB; paginate or truncate large lists inside the tool.

```js
// Add alongside your application routes in the createServer callback
if (req.url === '/xhub/agent-tool' && req.method === 'POST') {
  let body = ''
  req.on('data', (c) => (body += c))
  req.on('end', () => {
    try {
      const { tool, args } = JSON.parse(body || '{}')
      const handler = {
        list_orders: (a) => ({ orders: queryOrders(a.status) }),
        // TODO: Register every tool declared in manifest.agent.tools here
      }[tool]
      if (!handler) {
        res.statusCode = 404
        res.end(JSON.stringify({ error: `Undeclared tool: ${tool}` }))
        return
      }
      res.end(JSON.stringify(handler(args ?? {})))
    } catch (e) {
      res.statusCode = 400
      res.end(JSON.stringify({ error: String((e && e.message) || e) }))
    }
  })
  return
}
```

## Troubleshooting: Where to look when the backend won't start

The host **writes** backend stdout / stderr to `<data-root>\logs\service\<extension-id>.log` (max 1 MB per file, with a `----- service started <time> -----` separator on each launch so you can identify which lines belong to each run). On the frontend, `runtime.info().serviceReady === false` only means “the port isn't reachable”—the backend may not have started, may have crashed, or the port may be occupied. **Open this file first.** The host log `logs\x-hub.log` also includes `service backend not ready: … exit=<code>` and the last 20 backend log lines. Don't tell users speculative things like “Node is downloading.”

**Restart the backend process to apply backend code changes**: The frontend's roughly 1.5-second auto-reload only reloads the page; the backend process is reused by extension ID, so refreshing the page alone won't load new backend logic. For a persistent instance, right-click its pinned sidebar icon and choose **Restart backend** (stops and starts the Node process). If persistent mode is off, restart x-hub. The port changes on restart, but the extension page does not need to be refreshed—`service.request` looks up the current port mapping by extension ID each time.

## Where to store data and logs

**Don't infer the host data root from `__dirname`**—when mounted from a development directory, the extension's parent is a version directory or an arbitrary source directory, not `extensions`, so inference fails. There are only two suitable storage choices:

| What to store | Where to store it |
|---|---|
| Backend data / cache / logs | **`.data/`** inside the extension directory (dot-prefixed; skipped by host packaging and hot reload) |
| Key/value data shared across surfaces | Bridge API `config.*` / `storage.*` (the host persists it; don't write your own files) |

⚠️ Writing directly into the extension directory (outside `.data/`) in mounted mode means **writing into your source tree**. If that's also your release source directory, user data will be packaged and uploaded with the extension (a real incident put shared todo data into a review package). Put backup files left over by old-data migrations in `.data/` too.

⚠️ Strip the Windows `\\?\` prefix before passing a path to an external program (`spawn`, command-line argument)—`__dirname` may be a verbatim path in a mounted directory (`\\?\C:\…`), which external programs cannot read: `\\?\C:\x` → `C:\x`, `\\?\UNC\srv\share` → `\\srv\share`.

## Frontend calls

```js
const res = await window.xhub.service.request('/api/x', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ q: 'hello' }),
})
const data = await res.json()   // You can also use res.status / res.headers / res.text()
```

Use `serviceReady` / `proxyPrefix` from `runtime.info()` to check backend readiness. If the backend is down, `service.request` fails; show a clear, human-readable message instead of a blank screen.

## Authorization (`service:execute`)

A service backend runs only after the user **trusts the current version** (the “Run local backend” permission toggle). There are two points at which this can happen:

- **打开扩展时未授权**：宿主直接拦截入口加载——扩展页面**根本不会运行**，宿主自己的错误态带「去授权」按钮。你不需要（也无法）在页面里处理这种情况。
- **运行中被撤销授权**：页面还活着，但 `service.request` 会 reject `PERMISSION_DENIED: 本地后端未获信任或网络权限已关闭…`。此时界面应展示「本地后端未授权」提示 + **「去授权」按钮**，点击调 `xhub.openPermissions()`（无需权限）让宿主直接打开本扩展的设置弹窗：

```js
try {
  await window.xhub.service.request('/api/summary')
} catch (e) {
  if (/PERMISSION_DENIED|未获信任/.test(e.message) && typeof window.xhub.openPermissions === 'function') {
    showAuthHint() // 渲染「去授权」按钮，点击 → window.xhub.openPermissions()
  }
}
```

`typeof … === 'function'` 守卫是给旧宿主的兼容：没有这个桥 API 时按钮不渲染，只留错误文案。

## 部署注意

部署 service 扩展前，若 x-hub 正在运行并锁定了该扩展的后端文件，`deploy` 会报 **EPERM**——先退出 x-hub 再部署。
