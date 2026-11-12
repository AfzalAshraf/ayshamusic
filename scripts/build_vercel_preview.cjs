// Static Build Output API artifact only. Never build/run the local Rust service here.
const fs = require('node:fs');
const path = require('node:path');
const ROOT = path.resolve(__dirname, '..');
const MESSAGE = 'Frontend preview only — audio unavailable until the secure music backend is connected.';
function replaceOnce(text, marker, replacement) {
  if (text.split(marker).length !== 2) throw new Error('Preview source contract changed: ' + marker);
  return text.replace(marker, replacement);
}
function renderPreview(html) {
  html = replaceOnce(html, '<body>', `<body><aside class="vercel-preview-note" role="note">Frontend preview · Audio unavailable</aside>`);
  html = replaceOnce(html, '</head>', `<meta name="robots" content="noindex,nofollow"><style>
.vercel-preview-note{position:fixed;top:0;left:0;right:0;z-index:10000;padding:6px 10px;text-align:center;background:#35232e;color:#fff;font:12px/1.4 system-ui;pointer-events:none}
body .header{padding-top:42px;min-height:106px;flex-shrink:0}
</style></head>`);
  html = replaceOnce(html, '<small id="love-intro-hint">', '<small style="display:block;color:#ddd">Frontend preview · Audio unavailable</small><small id="love-intro-hint">');
  // Fail closed before any API call, even with a selection restored from tab storage.
  // Local production src/index.html is never written by this builder.
  html = replaceOnce(html, 'async function api(method, path, body) {',
    `async function api(method, path, body) {\n  throw new Error(${JSON.stringify(MESSAGE)});`);
  html = replaceOnce(html, 'setInterval(pollStatus, 400);', '// No backend polling in the static Vercel preview.');
  return html;
}
function build(output = path.join(ROOT, '.vercel/output')) {
  // Output contains generated files only; never retain a stale functions bundle.
  fs.rmSync(output, {recursive:true, force:true});
  fs.mkdirSync(path.join(output, 'static'), {recursive:true});
  const html = renderPreview(fs.readFileSync(path.join(ROOT, 'src/index.html'), 'utf8'));
  fs.writeFileSync(path.join(output, 'static/index.html'), html);
  fs.copyFileSync(path.join(ROOT, 'src/icons.svg'), path.join(output, 'static/icons.svg'));
  fs.writeFileSync(path.join(output, 'static/unavailable.json'), JSON.stringify({status:'failed', preview_only:true, error:MESSAGE}));
  fs.writeFileSync(path.join(output, 'static/404.html'), '<!doctype html><title>Not found</title><h1>Not found</h1><a href="/">Return to Aysha preview</a>');
  const config = {version:3, routes:[
    {src:'/api(?:/.*)?', dest:'/unavailable.json', status:503,
      headers:{'Content-Type':'application/json; charset=utf-8','Cache-Control':'no-store'}},
    {src:'/.*', headers:{'Cache-Control':'no-store','X-Content-Type-Options':'nosniff','Referrer-Policy':'same-origin'}, continue:true},
    {handle:'filesystem'},
    {src:'/.*', dest:'/404.html', status:404},
  ]};
  fs.writeFileSync(path.join(output, 'config.json'), JSON.stringify(config, null, 2) + '\n');
  return {html, config};
}
if (require.main === module) {
  build();
  console.log('Built static Aysha preview in .vercel/output; no Rust/functions/audio/backend routes.');
}
module.exports = {build, renderPreview, MESSAGE};
