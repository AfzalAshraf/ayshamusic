const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const vm = require('node:vm');
const {build, renderPreview, MESSAGE} = require('../scripts/build_vercel_preview.cjs');
const source = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf8');

test('static preview preserves original source and emits only allowlisted static files', () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'aysha-static-'));
  try {
    build(temp);
    assert.deepEqual(fs.readdirSync(temp).sort(), ['config.json','static']);
    assert.deepEqual(fs.readdirSync(path.join(temp,'static')).sort(), ['404.html','icons.svg','index.html','unavailable.json']);
    assert.equal(fs.readFileSync(path.join(__dirname,'../src/index.html'),'utf8'), source);
    assert.equal(fs.existsSync(path.join(temp,'functions')), false);
    assert.equal(fs.readFileSync(path.join(temp,'static/icons.svg'),'utf8'), fs.readFileSync(path.join(__dirname,'../src/icons.svg'),'utf8'));
  } finally { fs.rmSync(temp,{recursive:true,force:true}); }
});
test('preview labels both opening and home, retains effects and disables polling', () => {
  const html = renderPreview(source);
  assert.equal(html.split('Frontend preview · Audio unavailable').length - 1, 2);
  assert.ok(html.includes('heartPointerClick'));
  assert.ok(html.includes('Delius'));
  assert.ok(html.includes('Made with'));
  assert.ok(!html.includes('setInterval(pollStatus, 400);'));
  assert.ok(!html.includes('speed-insights'));
  assert.ok(!html.includes('Reveal a tiny heart'));
});
test('preview API fails closed without invoking fetch or audio', async () => {
  const html = renderPreview(source);
  const start = html.indexOf('async function api(');
  const end = html.indexOf('\nfunction icon(', start);
  const context = vm.createContext({fetch:()=>{throw new Error('Network must not run');}});
  vm.runInContext(html.slice(start,end), context);
  for (const endpoint of ['/api/search','/api/play','/api/bt/connect','/api/stream']) {
    await assert.rejects(vm.runInContext(`api('POST', ${JSON.stringify(endpoint)})`, context), {message:MESSAGE});
  }
});
test('API routing returns 503 rather than HTML or a working backend; unknown paths 404', () => {
  const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'aysha-static-'));
  try {
    const {config} = build(temp);
    assert.equal(config.version,3);
    const api = config.routes[0];
    assert.equal(api.status,503);
    for (const url of ['/api','/api/status','/api/stream','/api/bt/connect']) assert.ok(new RegExp('^'+api.src+'$').test(url));
    assert.equal(config.routes.at(-1).status,404);
    assert.equal(JSON.parse(fs.readFileSync(path.join(temp,'static/unavailable.json'))).preview_only,true);
    assert.ok(!JSON.stringify(config).includes('https://'));
  } finally { fs.rmSync(temp,{recursive:true,force:true}); }
});
test('build fails loudly if source injection contract changes', () => {
  assert.throws(()=>renderPreview(source.replace('async function api(', 'async function changedApi(')), /source contract changed/);
});
test('Vercel explicitly overrides Rust auto-detection with dependency-free static build', () => {
  const config = JSON.parse(fs.readFileSync(path.join(__dirname,'../vercel.json')));
  assert.equal(config.framework,null);
  assert.equal(config.buildCommand,'node scripts/build_vercel_preview.cjs');
  assert.ok(!config.buildCommand.includes('cargo'));
});
