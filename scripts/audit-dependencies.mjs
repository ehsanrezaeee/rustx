// Complete locked-package OSV query and redistributable license inventory.
import fs from 'node:fs/promises';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
const metadata = JSON.parse(
  execFileSync('cargo', ['metadata', '--format-version', '1', '--locked'], {
    encoding: 'utf8',
    maxBuffer: 32 * 1024 * 1024,
  }),
);
const dir = 'releases/licenses';
await fs.mkdir(dir, { recursive: true });
const inventory = [];
for (const p of metadata.packages) {
  const folder = path.dirname(p.manifest_path),
    files = [];
  for (const name of await fs.readdir(folder)) {
    if (/^(LICENSE|LICENCE|COPYING|NOTICE)([._-]|$)/i.test(name)) {
      const source = path.join(folder, name);
      if ((await fs.stat(source)).isFile()) {
        const relative = `${p.name}-${p.version}/${name}`;
        await fs.mkdir(path.dirname(path.join(dir, relative)), { recursive: true });
        await fs.copyFile(source, path.join(dir, relative));
        files.push(relative);
      }
    }
  }
  if (p.license_file) {
    const source = path.resolve(folder, p.license_file);
    try {
      const relative = `${p.name}-${p.version}/${path.basename(source)}`;
      await fs.mkdir(path.dirname(path.join(dir, relative)), { recursive: true });
      await fs.copyFile(source, path.join(dir, relative));
      if (!files.includes(relative)) files.push(relative);
    } catch {}
  }
  inventory.push({
    name: p.name,
    version: p.version,
    license: p.license,
    source: p.source,
    repository: p.repository,
    license_files: files,
  });
}
const report = {
  date: new Date().toISOString(),
  cargoLockSha256: createHash('sha256')
    .update(await fs.readFile('Cargo.lock'))
    .digest('hex'),
  packages: inventory,
  missingLicenseMetadata: inventory.filter((p) => !p.license),
  missingLicenseCopies: inventory.filter((p) => p.source && !p.license_files.length),
  limitations: [
    'SPDX metadata and supplied license texts are an inventory, not legal advice',
    'All locked target packages queried; dependency findings require reachability/maintainer review',
    'OSV availability and new advisories can change; rerun before publication',
  ],
};
await fs.writeFile(dir + '/inventory.json', JSON.stringify(report, null, 2) + '\n');
await fs.writeFile('docs/evidence/stage4-licenses.json', JSON.stringify(report, null, 2) + '\n');
const blocks = (await fs.readFile('Cargo.lock', 'utf8')).split('[[package]]').slice(1);
const packages = blocks
  .filter((s) => /^source = "registry\+/m.test(s))
  .map((s) => ({
    name: s.match(/^name = "([^"]+)"/m)[1],
    version: s.match(/^version = "([^"]+)"/m)[1],
  }));
const audit = {
  date: new Date().toISOString(),
  cargoLockSha256: report.cargoLockSha256,
  provider: 'https://api.osv.dev/v1/querybatch',
  queried: packages.length,
  completedQueries: 0,
  findings: [],
  details: [],
  limitations: report.limitations,
};
try {
  for (let start = 0; start < packages.length; start += 25) {
    const batch = packages.slice(start, start + 25);
    const response = await fetch(audit.provider, {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'user-agent': 'RustX-local-audit/0.1.0' },
      body: JSON.stringify({
        queries: batch.map((p) => ({
          package: { name: p.name, ecosystem: 'crates.io' },
          version: p.version,
        })),
      }),
      signal: AbortSignal.timeout(30000),
    });
    if (!response.ok)
      throw Error('OSV HTTP ' + response.status + ' ' + (await response.text()).slice(0, 500));
    audit.completedQueries += batch.length;
    const data = await response.json();
    for (let i = 0; i < batch.length; i++) {
      const result = data.results[i];
      if (result?.next_page_token)
        throw Error('OSV pagination requires follow-up for ' + batch[i].name);
      if (result?.vulns?.length)
        audit.findings.push({ ...batch[i], vulnerabilities: result.vulns });
    }
  }
  for (const id of [
    ...new Set(audit.findings.flatMap((f) => f.vulnerabilities.map((v) => v.id))),
  ]) {
    const r = await fetch('https://api.osv.dev/v1/vulns/' + encodeURIComponent(id), {
      signal: AbortSignal.timeout(30000),
    });
    if (!r.ok) throw Error('OSV detail ' + r.status);
    audit.details.push(await r.json());
  }
  audit.result = audit.findings.length ? 'findings' : 'no-known-advisories';
} catch (error) {
  audit.result = 'incomplete';
  audit.error = String(error);
  process.exitCode = 1;
} finally {
  await fs.writeFile(
    'docs/evidence/stage4-cargo-audit.json',
    JSON.stringify(audit, null, 2) + '\n',
  );
  console.log(
    JSON.stringify(
      {
        packages: inventory.length,
        missingCopies: report.missingLicenseCopies,
        queried: audit.queried,
        findings: audit.findings,
        result: audit.result,
        error: audit.error,
      },
      null,
      2,
    ),
  );
}
