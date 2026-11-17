// Actual host-native scaffold/package/extract/startup. This is not clean-machine or interaction certification.
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
const root = process.cwd();
const cli = path.join(root, 'target/debug', process.platform === 'win32' ? 'rustx.exe' : 'rustx');
const output = path.resolve(
  root,
  process.env.RUSTX_NATIVE_EVIDENCE_DIR || 'docs/evidence/react-parity/RP-007',
);
await fs.mkdir(output, { recursive: true });
const base = await fs.mkdtemp(path.join(os.tmpdir(), 'rustx-native-package-'));
const project = path.join(base, 'native-demo');
const env = { ...process.env, CARGO_TARGET_DIR: path.join(root, 'target') };
const report = {
  date: new Date().toISOString(),
  platform: process.platform,
  arch: process.arch,
  osRelease: os.release(),
  osVersion: os.version(),
  source: process.env.GITHUB_SHA || 'local working tree',
  cliSha256: createHash('sha256')
    .update(await fs.readFile(cli))
    .digest('hex'),
  checks: [],
  base,
};
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');
const run = (args, cwd = project) => execFileSync(cli, args, { cwd, env, stdio: 'inherit' });
try {
  run(['new', project], base);
  const blocked = path.join(project, 'keep-me');
  await fs.mkdir(blocked);
  await fs.writeFile(path.join(blocked, 'sentinel'), 'unchanged');
  for (const args of [
    ['package', '--out', 'keep-me'],
    ['package', '--out', '../escape'],
    ['package', '--target', process.platform === 'win32' ? 'macos' : 'windows'],
  ]) {
    const result = spawnSync(cli, args, { cwd: project, env, encoding: 'utf8' });
    assert.equal(result.status, 1, result.stderr);
  }
  assert.equal(await fs.readFile(path.join(blocked, 'sentinel'), 'utf8'), 'unchanged');
  report.checks.push(
    'existing output, traversal and wrong-host target rejected without replacing user files',
  );
  run(['package', '--target', 'native']);
  const bundle = path.join(project, 'dist-native');
  const entries = await fs.readdir(bundle);
  const zip = entries.find((n) => n.endsWith('.zip'));
  assert(zip);
  const sum = await fs.readFile(path.join(bundle, 'SHA256SUMS.txt'), 'utf8');
  assert.equal(sum.trim(), `${sha(await fs.readFile(path.join(bundle, zip)))}  ${zip}`);
  const extracted = path.join(base, 'recipient');
  await fs.mkdir(extracted);
  if (process.platform === 'darwin')
    execFileSync('ditto', ['-x', '-k', path.join(bundle, zip), extracted]);
  else
    execFileSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-NonInteractive',
        '-Command',
        '$ErrorActionPreference="Stop"; Expand-Archive -LiteralPath $env:RUSTX_TEST_ARCHIVE -DestinationPath $env:RUSTX_TEST_EXTRACT',
      ],
      {
        env: {
          ...process.env,
          RUSTX_TEST_ARCHIVE: path.join(bundle, zip),
          RUSTX_TEST_EXTRACT: extracted,
        },
      },
    );
  const payload = path.join(extracted, zip.slice(0, -4));
  const manifest = JSON.parse(await fs.readFile(path.join(payload, 'package.json'), 'utf8'));
  assert.equal(manifest.format, 'rustx-native-package-v1');
  assert.equal(manifest.static_crt, process.platform === 'win32');
  for (const [name, digest] of Object.entries(manifest.files))
    assert.equal(sha(await fs.readFile(path.join(payload, name))), digest, name);
  for (const line of (await fs.readFile(path.join(payload, 'SHA256SUMS.txt'), 'utf8'))
    .trim()
    .split('\n')) {
    const [digest, name] = line.split('  ');
    assert.equal(sha(await fs.readFile(path.join(payload, name))), digest, name);
  }
  const licenseIndex = Object.keys(manifest.files).find(
    (p) => p.endsWith('/licenses/DEPENDENCIES.json') || p === 'licenses/DEPENDENCIES.json',
  );
  assert(licenseIndex, 'dependency license inventory');
  const licenseReport = JSON.parse(await fs.readFile(path.join(payload, licenseIndex), 'utf8'));
  assert(licenseReport.packages.length > 1);
  report.licenseReviewNeeded = licenseReport.packages
    .filter((p) => p.review_needed)
    .map((p) => p.name + '@' + p.version);
  report.checks.push(
    'ZIP extracted; payload, manifest and archive SHA-256 hashes verified; dependency license inventory present',
  );
  if (process.platform === 'darwin')
    execFileSync('plutil', ['-lint', path.join(payload, 'native-demo.app/Contents/Info.plist')], {
      stdio: 'inherit',
    });
  const isolated = path.join(base, 'unrelated-cwd');
  await fs.mkdir(isolated);
  const launchEnv = Object.fromEntries(
    [
      'HOME',
      'TMPDIR',
      'USER',
      'LOGNAME',
      'SystemRoot',
      'WINDIR',
      'TEMP',
      'TMP',
      'USERPROFILE',
      'APPDATA',
      'LOCALAPPDATA',
    ]
      .filter((k) => process.env[k])
      .map((k) => [k, process.env[k]]),
  );
  launchEnv.PATH =
    process.platform === 'win32'
      ? `${process.env.SystemRoot}\\System32;${process.env.SystemRoot}`
      : '/usr/bin:/bin';
  launchEnv.RUSTX_RENDERER = 'cpu';
  launchEnv.RUSTX_SMOKE_FRAMES = '2';
  // Hide the source directory so source-relative runtime access cannot accidentally succeed.
  await fs.rename(project, project + '-hidden');
  let smoke;
  try {
    smoke = spawnSync(path.join(payload, manifest.executable), [], {
      cwd: isolated,
      env: launchEnv,
      encoding: 'utf8',
      timeout: 60000,
    });
  } finally {
    await fs.rename(project + '-hidden', project);
  }
  await fs.writeFile(
    path.join(output, `startup-${process.platform}.log`),
    (smoke.stdout || '') + (smoke.stderr || ''),
  );
  assert.equal(smoke.status, 0, String(smoke.error || smoke.stderr || smoke.signal));
  report.checks.push(
    'actual extracted native CPU window reaches two frames and exits, with source hidden, unrelated cwd and toolchain-free PATH',
  );
  const again = spawnSync(cli, ['package'], { cwd: project, env, encoding: 'utf8' });
  assert.equal(again.status, 1);
  assert.equal(await fs.readFile(path.join(bundle, 'SHA256SUMS.txt'), 'utf8'), sum);
  report.checks.push(
    'repeat packaging preserves existing verified package instead of silently overwriting it',
  );
  const artifacts = path.join(root, 'releases', 'RP-007');
  await fs.mkdir(artifacts, { recursive: true });
  await fs.copyFile(path.join(bundle, zip), path.join(artifacts, zip));
  await fs.writeFile(path.join(artifacts, zip + '.sha256'), sum);
  await fs.copyFile(
    path.join(payload, 'package.json'),
    path.join(output, `package-${process.platform}.json`),
  );
  await fs.copyFile(
    path.join(payload, 'DEPENDENCIES.txt'),
    path.join(output, `dependencies-${process.platform}.txt`),
  );
  report.archive = path.join(artifacts, zip);
  report.sha256 = sha(await fs.readFile(path.join(bundle, zip)));
  report.result = 'pass';
} catch (error) {
  report.result = 'fail';
  report.error = String(error.stack || error);
  throw error;
} finally {
  await fs.writeFile(
    path.join(output, `native-${process.platform}.json`),
    JSON.stringify(report, null, 2) + '\n',
  );
}
console.log(JSON.stringify(report, null, 2));
