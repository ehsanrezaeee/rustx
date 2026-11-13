const status = document.querySelector('#status'),
  tree = document.querySelector('#tree'),
  details = document.querySelector('#details');
function inspect() {
  const app = window.opener?.rustx;
  if (!app) {
    status.textContent = 'Open this inspector from an enabled RustX development bundle.';
    return;
  }
  const state = app.inspect();
  tree.replaceChildren();
  for (const node of state.nodes) {
    const row = document.createElement('tr');
    for (const value of [
      node.id || node.component || node.kind || node.node,
      node.owner ?? '',
      node.source ? `${node.source.file}:${node.source.line}` : '',
      node.rect.map((n) => Math.round(n * 10) / 10).join(', '),
    ]) {
      const cell = document.createElement('td');
      cell.textContent = String(value);
      row.append(cell);
    }
    if (node.source && app.sourceRoot) {
      const cell = row.children[2],
        link = document.createElement('a'),
        file = node.source.file.startsWith('/')
          ? node.source.file
          : app.sourceRoot + '/' + node.source.file;
      link.textContent = cell.textContent;
      link.href =
        'vscode://file/' +
        file.split('/').map(encodeURIComponent).join('/') +
        ':' +
        node.source.line +
        ':' +
        node.source.column;
      cell.replaceChildren(link);
    }
    row.tabIndex = 0;
    row.onclick = () => {
      app.highlight(node.node);
      details.textContent = JSON.stringify(node, null, 2);
    };
    row.onkeydown = (e) => {
      if (e.key === 'Enter') row.click();
    };
    tree.append(row);
  }
  status.textContent = `${state.nodes.length} materialized nodes; counts ${state.counts.join(', ')}`;
  details.textContent = JSON.stringify(
    { metrics: state.metrics, owners: state.owners, profile: app.profile() },
    null,
    2,
  );
}
document.querySelector('#refresh').onclick = inspect;
document.querySelector('#profile').onclick = () => {
  const app = window.opener?.rustx;
  details.textContent = JSON.stringify(app?.profile(), null, 2);
};
inspect();
