const vscode = require('vscode'),
  { spawn } = require('node:child_process');
function run(args, input, cwd) {
  return new Promise((resolve, reject) => {
    const cli = vscode.workspace.getConfiguration('rustx').get('path', 'rustx');
    const child = spawn(cli, args, { cwd, stdio: ['pipe', 'pipe', 'pipe'], shell: false });
    let text = '',
      error = '';
    const timer = setTimeout(() => {
      child.kill();
      reject(Error('RustX command timed out'));
    }, 120000);
    child.on('error', (e) => {
      clearTimeout(timer);
      reject(e);
    });
    child.stdout.on('data', (c) => {
      text += c;
      if (text.length > 8 * 1024 * 1024) {
        child.kill();
        reject(Error('RustX response too large'));
      }
    });
    child.stderr.on('data', (c) => (error = (error + c).slice(-65536)));
    child.on('close', (code) => {
      clearTimeout(timer);
      resolve({ code, text, error });
    });
    child.stdin.end(input || '');
  });
}
function activate(context) {
  if (!vscode.workspace.isTrusted) return;
  const selectors = [{ language: 'rust', scheme: 'file' }],
    diagnostics = vscode.languages.createDiagnosticCollection('rustx');
  context.subscriptions.push(diagnostics);
  const cwd = (doc) =>
    vscode.workspace.getWorkspaceFolder(doc.uri)?.uri.fsPath ||
    require('node:path').dirname(doc.uri.fsPath);
  const format = {
    provideDocumentFormattingEdits: async (doc, options, token) => {
      const result = await run(['fmt', '--stdin'], doc.getText(), cwd(doc));
      if (result.code) throw Error(result.error);
      return [
        vscode.TextEdit.replace(
          new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length)),
          result.text,
        ),
      ];
    },
  };
  context.subscriptions.push(
    vscode.languages.registerDocumentFormattingEditProvider(selectors, format),
  );
  async function symbols(doc) {
    const result = await run(['symbols'], null, cwd(doc));
    if (result.code) throw Error(result.error);
    return JSON.parse(result.text);
  }
  context.subscriptions.push(
    vscode.languages.registerCompletionItemProvider(
      selectors,
      {
        async provideCompletionItems(doc, position) {
          const line = doc.lineAt(position.line).text.slice(0, position.character);
          const offset = doc.offsetAt(position);
          const before = doc.getText().slice(Math.max(0, offset - 16384), offset);
          const data = await symbols(doc);
          if (/<\/?[A-Za-z]*$/.test(line)) {
            return [
              ...data.catalog.tags,
              ...data.symbols.filter((s) => s.component).map((s) => s.name),
            ].map((tag) => {
              const c = new vscode.CompletionItem(tag, vscode.CompletionItemKind.Class);
              c.detail = 'RustX component';
              return c;
            });
          }
          const tag = before.match(/<([A-Z]\w*)\b[^>]*$/)?.[1];
          if (!tag) return [];
          const component = data.symbols.find((s) => s.name === tag && s.component);
          return (component?.props || data.catalog.props)
            .filter(
              (prop) =>
                !['drawing', 'camera', 'on_canvas', 'src'].includes(prop) ||
                (prop === 'src' ? tag === 'Image' : tag === 'Canvas'),
            )
            .map((prop) => {
              const c = new vscode.CompletionItem(prop, vscode.CompletionItemKind.Property);
              c.insertText = new vscode.SnippetString(prop + '={${1}}');
              c.detail = component ? 'Rust component prop' : 'RustX typed prop';
              return c;
            });
        },
      },
      '<',
      ' ',
    ),
  );
  context.subscriptions.push(
    vscode.languages.registerDefinitionProvider(selectors, {
      async provideDefinition(doc, position) {
        const range = doc.getWordRangeAtPosition(position);
        if (!range) return;
        const name = doc.getText(range);
        const data = await symbols(doc);
        return data.symbols
          .filter((s) => s.name === name)
          .map(
            (s) =>
              new vscode.Location(
                vscode.Uri.file(s.file),
                new vscode.Position(s.line, s.character),
              ),
          );
      },
    }),
  );
  async function check() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;
    await vscode.workspace.saveAll();
    const result = await run(['check', '--json'], null, cwd(editor.document));
    diagnostics.clear();
    const byFile = new Map();
    for (const line of result.text.split('\n')) {
      let event;
      try {
        event = JSON.parse(line);
      } catch {
        continue;
      }
      const message = event.message;
      if (!message || event.reason !== 'compiler-message') continue;
      for (const span of message.spans || []) {
        if (
          !span.is_primary ||
          (!span.file_name.endsWith('.rustx') && !span.file_name.endsWith('.rs'))
        )
          continue;
        const path = require('node:path').resolve(cwd(editor.document), span.file_name);
        const range = new vscode.Range(
          span.line_start - 1,
          span.column_start - 1,
          span.line_end - 1,
          span.column_end - 1,
        );
        const d = new vscode.Diagnostic(
          range,
          message.message,
          message.level === 'error'
            ? vscode.DiagnosticSeverity.Error
            : vscode.DiagnosticSeverity.Warning,
        );
        d.source = 'rustx / rustc';
        d.code = message.code?.code;
        const list = byFile.get(path) || [];
        list.push(d);
        byFile.set(path, list);
      }
    }
    for (const [path, list] of byFile) diagnostics.set(vscode.Uri.file(path), list);
    return result.code;
  }
  context.subscriptions.push(
    vscode.commands.registerCommand('rustx.check', check),
    vscode.commands.registerCommand('rustx.format', () =>
      vscode.commands.executeCommand('editor.action.formatDocument'),
    ),
  );
  return { check, format, symbols };
}
module.exports = { activate };
