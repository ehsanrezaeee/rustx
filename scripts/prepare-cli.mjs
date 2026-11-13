// Nested Cargo packages are excluded by Cargo's package walker. Embed the complete
// SDK as one deterministic archive, then unpack only in the build output directory.
import fs from 'node:fs/promises';
import path from 'node:path';
const root = process.cwd(),
  assets = path.join(root, 'crates/rustx-cli/assets');
await fs.rm(assets, { recursive: true, force: true });
await fs.mkdir(assets, { recursive: true });
const paths = [];
async function walk(relative) {
  for (const entry of await fs.readdir(relative, { withFileTypes: true })) {
    if (entry.isSymbolicLink()) throw Error('SDK symlink ' + relative);
    const name = path.posix.join(relative, entry.name);
    if (name === 'docs/evidence') continue;
    if (entry.isDirectory()) await walk(name);
    else if (!/\.(png|pdf)$/.test(name)) paths.push(name);
  }
}
for (const entry of await fs.readdir('crates', { withFileTypes: true })) {
  if (entry.isDirectory() && entry.name !== 'rustx-cli') await walk('crates/' + entry.name);
}
for (const dir of ['web', 'fixtures', 'playground', 'docs']) await walk(dir);
paths.push('LICENSE', 'NOTICE.md', 'rust-toolchain.toml', 'Cargo.lock');
paths.sort();
const chunks = [];
function add(name, bytes) {
  const key = Buffer.from(name),
    header = Buffer.alloc(12);
  header.writeUInt32LE(key.length, 0);
  header.writeBigUInt64LE(BigInt(bytes.length), 4);
  chunks.push(header, key, bytes);
}
for (const file of paths) add(file, await fs.readFile(file));
add('Cargo.toml', await fs.readFile('crates/rustx-cli/templates/sdk-workspace.toml'));
await fs.writeFile(path.join(assets, 'framework.bin'), Buffer.concat(chunks));
console.log('Prepared self-contained CLI SDK archive: ' + paths.length + ' files.');
