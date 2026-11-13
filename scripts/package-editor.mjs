import fs from 'node:fs/promises';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
const root = fileURLToPath(new URL('../', import.meta.url)),
  folder = path.join(root, '.rustx/editor-vsix');
await fs.rm(folder, { recursive: true, force: true });
await fs.mkdir(path.join(folder, 'extension'), { recursive: true });
for (const name of ['package.json', 'extension.cjs', 'README.md'])
  await fs.copyFile(path.join(root, 'editors/vscode', name), path.join(folder, 'extension', name));
await fs.writeFile(
  path.join(folder, '[Content_Types].xml'),
  '<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="json" ContentType="application/json"/><Default Extension="cjs" ContentType="application/javascript"/><Default Extension="md" ContentType="text/markdown"/><Default Extension="vsixmanifest" ContentType="text/xml"/></Types>',
);
await fs.writeFile(
  path.join(folder, 'extension.vsixmanifest'),
  '<?xml version="1.0"?><PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011"><Metadata><Identity Language="en-US" Id="rustx" Version="0.1.0" Publisher="rustx-local"/><DisplayName>RustX</DisplayName><Description xml:space="preserve">RustX authoring tools</Description><Tags>Rust</Tags><Categories>Programming Languages,Formatters</Categories><Properties><Property Id="Microsoft.VisualStudio.Code.Engine" Value="^1.90.0"/><Property Id="Microsoft.VisualStudio.Code.ExtensionDependencies" Value="rust-lang.rust-analyzer"/></Properties></Metadata><Installation><InstallationTarget Id="Microsoft.VisualStudio.Code"/></Installation><Dependencies/><Assets><Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true"/><Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true"/></Assets></PackageManifest>',
);
await fs.mkdir(path.join(root, 'releases'), { recursive: true });
const output = path.join(root, 'releases/rustx-editor-0.1.0.vsix');
await fs.rm(output, { force: true });
execFileSync('zip', ['-q', '-r', output, '.'], { cwd: folder });
const digest = createHash('sha256')
  .update(await fs.readFile(output))
  .digest('hex');
await fs.writeFile(output + '.sha256', `${digest}  ${path.basename(output)}\n`);
console.log(output + '\nSHA-256 ' + digest);
