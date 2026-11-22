// Development bundler only. Output is used exclusively by the isolated test APK.
import { build } from 'esbuild';
import { mkdir, writeFile, copyFile, readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const root = path.dirname(fileURLToPath(import.meta.url));
const out = path.join(root, 'build');
await mkdir(out, { recursive: true });
const result = await build({ absWorkingDir: root, entryPoints: ['android/driver.mjs'],
  outfile: path.join(out, 'fixture.js'), bundle: true, format: 'iife', platform: 'browser',
  target: 'chrome70', keepNames: true, legalComments: 'eof', metafile: true });
await writeFile(path.join(out, 'metafile.json'), JSON.stringify(result.metafile, null, 2));
for (const [name, source] of Object.entries({
  'YOUTUBEJS-LICENSE.txt': 'node_modules/youtubei.js/LICENSE',
  'PROTOBUF-LICENSE.txt': 'notices/PROTOBUF-LICENSE.txt',
  'FFLATE-LICENSE.txt': 'node_modules/fflate/LICENSE',
  'MERIYAH-LICENSE.txt': 'node_modules/youtubei.js/node_modules/meriyah/LICENSE.md'
})) await copyFile(path.join(root, source), path.join(out, name));
console.log('Built isolated Android fixture from pinned source modules; no provider access.');
// The published protobuf package omits its root Apache license. Preserve the
// reviewed v2.17.0 license above plus the embedded Google BSD copyright/terms.
const varint = await readFile(path.join(root, 'node_modules/@bufbuild/protobuf/dist/esm/wire/varint.js'), 'utf8');
await writeFile(path.join(out, 'PROTOBUF-BSD-NOTICE.txt'), varint.split('/**')[0]);
