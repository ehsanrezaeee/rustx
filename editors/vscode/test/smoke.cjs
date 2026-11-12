const vscode = require('vscode'),
  assert = require('node:assert/strict'),
  fs = require('node:fs/promises'),
  path = require('node:path');
exports.run = async () => {
  const report = {
    date: new Date().toISOString(),
    editor: vscode.version,
    checks: [],
    limitations: [
      'Actual VS Code extension-host automation on macOS; no Windows/editor-human-usability claim',
    ],
  };
  try {
    const root = process.env.RUSTX_EDITOR_PROJECT,
      cli = process.env.RUSTX_CLI;
    assert(root && cli);
    await vscode.workspace
      .getConfiguration('rustx')
      .update('path', cli, vscode.ConfigurationTarget.Workspace);
    await vscode.workspace
      .getConfiguration()
      .update(
        '[rust]',
        { 'editor.defaultFormatter': 'rustx-local.rustx' },
        vscode.ConfigurationTarget.Workspace,
      );
    const ext = vscode.extensions.getExtension('rustx-local.rustx');
    assert(ext);
    await ext.activate();
    report.rustAnalyzer =
      vscode.extensions.getExtension('rust-lang.rust-analyzer')?.packageJSON.version;
    await fs.writeFile(
      path.join(root, 'src/card.rs'),
      'use rustx::prelude::*;\n#[component]\npub fn Welcome(cx:Scope,title:String)->View{rustx!{<Text>{title}</Text>}}\n',
    );
    const lib = path.join(root, 'src/lib.rs');
    let library = await fs.readFile(lib, 'utf8');
    if (!library.includes('pub mod card;')) await fs.writeFile(lib, 'pub mod card;\n' + library);
    const uri = vscode.Uri.file(path.join(root, 'src/app.rustx'));
    const doc = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(doc);
    assert.equal(doc.languageId, 'rust');
    const symbols = await ext.exports.symbols(doc);
    assert(symbols.symbols.some((item) => item.name === 'Welcome' && item.props.includes('title')));
    async function replace(text) {
      const edit = new vscode.WorkspaceEdit();
      edit.replace(
        uri,
        new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)),
        text,
      );
      await vscode.workspace.applyEdit(edit);
    }
    const prefix =
      'use rustx::prelude::*;\nuse crate::card::{Welcome,WelcomeProps};\npub fn app(cx:Scope)->View{rustx!{<Column>';
    await replace(prefix + '<Wel');
    let completions = await vscode.commands.executeCommand(
      'vscode.executeCompletionItemProvider',
      uri,
      doc.positionAt(doc.getText().length),
    );
    assert(completions.items.some((i) => i.label === 'Welcome'));
    report.checks.push('actual .rustx language recognition and imported Rust component completion');
    await replace(prefix + '<Welcome ');
    completions = await vscode.commands.executeCommand(
      'vscode.executeCompletionItemProvider',
      uri,
      doc.positionAt(doc.getText().length),
    );
    assert(completions.items.some((i) => i.label === 'title'));
    report.checks.push('typed custom component property completion');
    const valid = prefix + '<Welcome title={"Editor works"}/></Column>}}\n';
    await replace(valid);
    await doc.save();
    const definitions = await vscode.commands.executeCommand(
      'vscode.executeDefinitionProvider',
      uri,
      doc.positionAt(doc.getText().lastIndexOf('Welcome') + 2),
    );
    assert(definitions.some((d) => (d.uri || d.targetUri).fsPath.endsWith('card.rs')));
    report.checks.push('navigation from original .rustx tag to imported component definition');
    await vscode.commands.executeCommand('editor.action.formatDocument');
    report.formatter = vscode.workspace.getConfiguration('editor', uri).get('defaultFormatter');
    report.formatted = doc.getText();
    assert(doc.getText().includes('<Column>\n'));
    await doc.save();
    report.checks.push('actual formatter provider formats markup with Rust expressions');
    await replace(valid.replace('title={"Editor works"}', 'title={12}'));
    await doc.save();
    await vscode.commands.executeCommand('rustx.check');
    const errors = vscode.languages
      .getDiagnostics(uri)
      .filter(
        (d) => d.severity === vscode.DiagnosticSeverity.Error && d.source === 'rustx / rustc',
      );
    assert(errors.length);
    assert(errors.some((d) => doc.lineAt(d.range.start.line).text.includes('12')));
    report.diagnostics = errors.map((d) => ({
      line: d.range.start.line + 1,
      code: d.code,
      message: d.message,
    }));
    report.checks.push('real cargo/rustc type error displayed on original .rustx source line');
    await replace(valid);
    await doc.save();
    report.result = 'passed';
  } catch (error) {
    report.result = 'failed';
    report.error = String(error);
    throw error;
  } finally {
    await fs.writeFile(process.env.RUSTX_EDITOR_REPORT, JSON.stringify(report, null, 2) + '\n');
    console.log(JSON.stringify(report, null, 2));
  }
};
