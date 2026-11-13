// Recover notices omitted from crate archives, pinned to each archive's own VCS commit.
import fs from 'node:fs/promises';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
const inventory = JSON.parse(await fs.readFile('docs/evidence/stage4-licenses.json')),
  metadata = JSON.parse(
    execFileSync('cargo', ['metadata', '--format-version', '1', '--locked'], {
      encoding: 'utf8',
      maxBuffer: 32 * 1024 * 1024,
    }),
  );
if (
  inventory.cargoLockSha256 !==
  createHash('sha256')
    .update(await fs.readFile('Cargo.lock'))
    .digest('hex')
) {
  throw Error('License inventory is stale; run scripts/audit-dependencies.mjs first');
}
const cache = new Map();
async function retrieve(repo, commit) {
  const key = repo + '@' + commit;
  if (cache.has(key)) return cache.get(key);
  const pending = (async () => {
    const files = [];
    await Promise.all(
      [
        'LICENSE',
        'LICENSE-MIT',
        'LICENSE-APACHE',
        'LICENSE.md',
        'LICENSE.txt',
        'LICENSE-MIT.txt',
        'LICENSE-APACHE.txt',
        'LICENSE.BSD-2-Clause',
        'LICENSE.MIT',
        'LICENSE.Apache-2.0',
        'COPYING',
        'COPYING.MIT',
        'COPYING.APACHE',
      ].map(async (name) => {
        const url = `https://raw.githubusercontent.com/${repo}/${commit}/${name}`;
        try {
          const response = await fetch(url, { signal: AbortSignal.timeout(15000) });
          if (response.ok) {
            const bytes = await response.text();
            if (bytes.length > 50 && bytes.length < 200000) files.push({ name, url, bytes });
          }
        } catch {}
      }),
    );
    return files;
  })();
  cache.set(key, pending);
  return pending;
}
const pending = inventory.packages.filter((p) => p.source && !p.license_files.length);
for (let i = 0; i < pending.length; i += 4) {
  await Promise.all(
    pending.slice(i, i + 4).map(async (entry) => {
      const pkg = metadata.packages.find(
          (p) => p.name === entry.name && p.version === entry.version,
        ),
        match = pkg.repository?.match(
          /^https?:\/\/github.com\/([^/]+\/[^/#]+?)(?:\.git)?(?:\/tree\/.*|\/)?$/,
        );
      if (!match) return;
      let vcs;
      try {
        vcs = JSON.parse(
          await fs.readFile(path.join(path.dirname(pkg.manifest_path), '.cargo_vcs_info.json')),
        );
      } catch {
        return;
      }
      if (!vcs.git?.sha1?.match(/^[a-f0-9]{40}$/)) return;
      for (const file of await retrieve(match[1], vcs.git.sha1)) {
        const relative = `${entry.name}-${entry.version}/${file.name}`;
        await fs.mkdir(path.join('releases/licenses', path.dirname(relative)), { recursive: true });
        await fs.writeFile(path.join('releases/licenses', relative), file.bytes);
        entry.license_files.push(relative);
        (entry.upstream_notices ??= []).push({
          file: relative,
          url: file.url,
          vcs_commit: vcs.git.sha1,
        });
      }
    }),
  );
}
inventory.missingLicenseCopies = inventory.packages.filter(
  (p) => p.source && !p.license_files.length,
);
await fs.writeFile('docs/evidence/stage4-licenses.json', JSON.stringify(inventory, null, 2) + '\n');
await fs.writeFile('releases/licenses/inventory.json', JSON.stringify(inventory, null, 2) + '\n');
console.log(
  JSON.stringify(
    {
      recovered: pending.length - inventory.missingLicenseCopies.length,
      remaining: inventory.missingLicenseCopies.map((p) => ({
        name: p.name,
        repository: p.repository,
        license: p.license,
      })),
    },
    null,
    2,
  ),
);
