import fs from 'node:fs/promises';
import { spawn, execFileSync } from 'node:child_process';
import path from 'node:path';
const root = process.cwd(),
  base = await fs.mkdtemp('/private/tmp/rustx-editor-smoke-'),
  project = path.join(base, 'project');
execFileSync(path.join(root, 'target/debug/rustx'), ['new', project], { stdio: 'inherit' });
await fs.mkdir(path.join(base, 'profile/User'), { recursive: true });
await fs.writeFile(
  path.join(base, 'profile/User/settings.json'),
  JSON.stringify({
    'telemetry.telemetryLevel': 'off',
    'workbench.startupEditor': 'none',
    'rust-analyzer.checkOnSave': false,
    'security.workspace.trust.enabled': false,
  }),
);
const code =
  process.env.VSCODE_PATH || '/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code';
const extensions =
  process.env.RUSTX_EDITOR_EXTENSIONS || '/private/tmp/rustx-stage4-editor-extensions';
const report = path.join(root, 'docs/evidence/stage4-editor.json');
await fs.rm(report, { force: true });
const log = await fs.open(path.join(root, 'docs/evidence/stage4-editor-smoke.log'), 'w');
const child = spawn(
  code,
  [
    '--user-data-dir',
    path.join(base, 'profile'),
    '--extensions-dir',
    extensions,
    '--extensionDevelopmentPath=' + root + '/editors/vscode',
    '--extensionTestsPath=' + root + '/editors/vscode/test/smoke.cjs',
    '--disable-workspace-trust',
    '--skip-welcome',
    '--skip-release-notes',
    '--new-window',
    project,
  ],
  {
    env: {
      ...process.env,
      RUSTX_EDITOR_PROJECT: project,
      RUSTX_EDITOR_REPORT: report,
      RUSTX_CLI: path.join(root, 'target/debug/rustx'),
      CARGO_TARGET_DIR: path.join(root, 'target'),
    },
    stdio: ['ignore', log.fd, log.fd],
  },
);
child.on('error', (error) => {
  throw error;
});
const deadline = Date.now() + 180000;
let result;
while (Date.now() < deadline) {
  try {
    result = JSON.parse(await fs.readFile(report, 'utf8'));
    break;
  } catch {}
  await new Promise((r) => setTimeout(r, 500));
}
await log.close();
if (!result) throw Error('Actual editor smoke timed out; inspect stage4-editor-smoke.log');
console.log(JSON.stringify(result, null, 2));
if (result.result !== 'passed') process.exitCode = 1;
