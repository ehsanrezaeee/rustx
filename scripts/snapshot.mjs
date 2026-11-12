// Content identity without recursive hashes, caches, or build/evidence output.
import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
const root = new URL('../', import.meta.url);
const excluded = new Set([
  '.git',
  'target',
  'node_modules',
  'dist',
  'dist-release',
  'dist-bench',
  'dist-native',
  '.rustx',
  'releases',
  '.DS_Store',
  'SOURCE_MANIFEST.sha256',
  'SOURCE-REVISION.txt',
  'BUILD-INFO.json',
]);
const files = [];
async function walk(relative = '') {
  for (const entry of await fs.readdir(new URL(relative, root), { withFileTypes: true })) {
    if (excluded.has(entry.name)) continue;
    const file = path.posix.join(relative, entry.name);
    if (file === 'docs/evidence') continue;
    if (entry.isDirectory()) await walk(file + '/');
    else if (entry.isFile()) files.push(file);
  }
}
const hash = (data) => createHash('sha256').update(data).digest('hex');
await walk();
files.sort();
const rows = [];
for (const file of files) rows.push(`${hash(await fs.readFile(new URL(file, root)))}  ${file}\n`);
const manifest = rows.join('');
await fs.writeFile(new URL('SOURCE_MANIFEST.sha256', root), manifest);
await fs.writeFile(
  new URL('SOURCE-REVISION.txt', root),
  `${hash(manifest)}  SOURCE_MANIFEST.sha256\n`,
);
console.log(`Source manifest: ${files.length} files; ${hash(manifest)}`);
