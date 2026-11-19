import path from 'node:path';
import { execFileSync } from 'node:child_process';

export function createZip(archive, entries, cwd) {
  if (process.platform === 'win32') {
    execFileSync(
      path.join(process.env.SystemRoot, 'System32', 'tar.exe'),
      ['--format', 'zip', '-cf', archive, '--', ...entries],
      { cwd },
    );
  } else {
    execFileSync('zip', ['-q', '-r', archive, ...entries], { cwd });
  }
}
