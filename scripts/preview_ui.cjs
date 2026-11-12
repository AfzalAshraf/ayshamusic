// Interactive sandbox preview, not a replacement for the production Rust/audio server.
// Run: node scripts/preview_ui.cjs (port 5050 by default)
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const unavailable = { status: 'failed', error: 'UI preview only: Rust/audio backend unavailable here. Heart effects do not need it.' };
const banner = `<aside style="position:fixed;top:0;left:0;right:0;z-index:10000;background:#35232e;color:#fff;text-align:center;padding:7px 12px;font:12px/1.4 system-ui;pointer-events:none">Interactive UI preview · Audio unavailable · <span data-preview-motion>Checking heart animation…</span></aside><style>body .header{padding-top:42px;min-height:106px;flex-shrink:0}</style>`;
const diagnostics = `<script>
(() => {
  const note = document.createElement('small');
  note.setAttribute('data-preview-motion', '');
  note.style.cssText = 'display:block;margin-top:8px;color:#ddd;font-size:10px;letter-spacing:0';
  document.querySelector('.love-loading').appendChild(note);
  const update = () => {
    const message = typeof heartPointerClick !== 'function' ? 'Heart script did not initialize' : 'Hearts always on — tap/click the page to make them fly';
    document.querySelectorAll('[data-preview-motion]').forEach(el => { el.textContent = message; });
  };
  update();
})();
</script>`;
function createPreviewServer() {
  return http.createServer((req, res) => {
    const url = new URL(req.url, 'http://preview.invalid');
    res.setHeader('Cache-Control', 'no-store');
    const respond = (code, type, body) => {
      res.writeHead(code, { 'Content-Type': type });
      res.end(req.method === 'HEAD' ? undefined : body);
    };
    if (url.pathname === '/' && ['GET', 'HEAD'].includes(req.method)) {
      const html = fs.readFileSync(path.join(root, 'src/index.html'), 'utf8').replace('<body>', '<body>' + banner).replace('</body>', diagnostics + '</body>');
      return respond(200, 'text/html; charset=utf-8', html);
    }
    if (url.pathname === '/icons.svg' && ['GET', 'HEAD'].includes(req.method)) return respond(200, 'image/svg+xml', fs.readFileSync(path.join(root, 'src/icons.svg')));
    if (url.pathname === '/favicon.ico') return respond(204, 'image/x-icon', '');
    if (url.pathname === '/api/status' && req.method === 'GET') return respond(200, 'application/json', JSON.stringify({ current_track: null, is_playing: false, queue_len: 0, volume: 0.8, status_msg: unavailable.error, preview_only: true }));
    if (url.pathname === '/api/bt/status' && req.method === 'GET') return respond(200, 'application/json', JSON.stringify({ connected: false, error: unavailable.error, preview_only: true }));
    if (url.pathname.startsWith('/api/')) return respond(503, 'application/json', JSON.stringify(unavailable));
    return respond(404, 'text/plain; charset=utf-8', 'Not found');
  });
}
if (require.main === module) {
  const server = createPreviewServer();
  server.on('error', error => { console.error(error); process.exitCode = 1; });
  const port = Number(process.env.PORT || 5050);
  server.listen(port, '0.0.0.0', () => console.log(`Interactive Aysha UI on 0.0.0.0:${port}; hearts work locally, live audio backend unavailable.`));
}
module.exports = { createPreviewServer };
