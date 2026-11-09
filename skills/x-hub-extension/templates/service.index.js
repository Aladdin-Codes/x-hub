// service/index.js — dependency-free Node backend listening on the host-provided PORT
// Replace TODOs with real logic; the frontend should always use window.xhub.service.request('/api/...').
// Don't fetch the port directly; the host reverse-proxies /svc/<extId>/* and handles CORS.
const http = require('http')

const port = parseInt(process.env.PORT || '0', 10)
const HOST = '127.0.0.1' // Rule: listen only on loopback.

const server = http.createServer((req, res) => {
  res.setHeader('Content-Type', 'application/json; charset=utf-8')

  // Keep the health-check path in sync with manifest.backend.health.
  if (req.url === '/healthz') {
    res.end(JSON.stringify({ ok: true }))
    return
  }

  if (req.url === '/api/hello') {
    res.end(JSON.stringify({ message: 'hi', extId: process.env.XHUB_EXT_ID }))
    return
  }

  // AI tool endpoint (register tools declared in manifest.agent.tools here; a 2xx response body is the tool result.
  // For failures, return a non-2xx response and put the reason in the error field; max response size is 256 KB, so paginate large lists.)
  if (req.url === '/xhub/agent-tool' && req.method === 'POST') {
    let body = ''
    req.on('data', (c) => (body += c))
    req.on('end', () => {
      try {
        const { tool, args } = JSON.parse(body || '{}')
        const handlers = {
          get_info: () => ({ name: 'My Extension', version: '0.1.0' }),
          // TODO: Replace with real data queries/actions.
        }
        const handler = handlers[tool]
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

  // TODO: Add real routes here (when reading the body, append chunks; don't assume it arrives all at once).
  res.statusCode = 404
  res.end(JSON.stringify({ error: 'NOT_FOUND', path: req.url }))
})

server.listen(port, HOST, () => {
  // The host assigns the port (manifest.backend.port = 0); get the actual port from server.address().
  console.log(`[service] listening on http://${HOST}:${server.address().port}`)
})

// The host sends SIGTERM when stopping the extension; shut down gracefully.
process.on('SIGTERM', () => server.close(() => process.exit(0)))
