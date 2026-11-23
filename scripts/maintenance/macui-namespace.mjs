// Mechanical namespace migration. Legal attribution and external dependencies stay intact.
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '../..');
const ignored = new Set(['node_modules', 'target', 'dist', 'vendor', 'npm', 'gen', '.git']);
const extensions = new Set(['.rs', '.ts', '.tsx', '.js', '.mjs', '.svelte', '.scss', '.css', '.yml', '.yaml', '.json', '.toml', '.html', '.xml', '.nsh', '.nsi', '.ps1']);
function rename(text) {
  return text
    .replaceAll('sluhk', 'macui-hook')
    .replaceAll('slu-service', 'macui-service')
    .replaceAll('slu_service', 'macui_service')
    .replaceAll('slu.exe', 'macui-cli.exe')
    .replaceAll('src/slu/', 'src/macui-cli/')
    .replaceAll('"slu/main.rs"', '"macui-cli/main.rs"')
    .replaceAll('slu-ipc', 'macui-ipc').replaceAll('slu_ipc', 'macui_ipc')
    .replaceAll('slu-macros', 'macui-macros').replaceAll('slu_macros', 'macui_macros')
    .replaceAll('slu-utils', 'macui-utils').replaceAll('slu_utils', 'macui_utils')
    .replaceAll('seelen-core', 'macui-core').replaceAll('seelen_core', 'macui_core')
    .replaceAll('@seelen-ui/', '@macui/').replaceAll('@seelen/', '@macui/')
    .replaceAll('seelen-ui', 'macui').replaceAll('seelen_ui', 'macui')
    .replaceAll('SeelenUI', 'MacUi').replaceAll('Seelen', 'MacUi')
    .replaceAll('seelen', 'macui')
    .replaceAll('__SLU', '__MACUI').replaceAll('SLU_', 'MACUI_')
    .replaceAll('slu-', 'macui-').replaceAll('slu_', 'macui_')
    .replaceAll('SEELEN', 'MACUI')
    .replaceAll('Slu', 'MacUi');
}
const changed = [];
function walk(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (ignored.has(entry.name)) continue;
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) { walk(file); continue; }
    if (!extensions.has(path.extname(file))) continue;
    if (file.endsWith('macui-namespace.mjs') || file.endsWith('local_migration.rs')) continue;
    const source = fs.readFileSync(file, 'utf8');
    const result = source.split(/(?<=\n)/).map(line => {
      if (/copyright|©|Based on Seelen UI|upstream Seelen UI|保留 Seelen UI|原作版权|original copyright|AGPL.*Seelen|Seelen.*AGPL/i.test(line)) return line;
      if (/https?:\/\/(?:github\.com\/(?:Seelen-Inc|eythaann)|discord\.seelen|[^\s"']*seelen\.io)/i.test(line)) return line;
      if (line.includes('@seelen/translation-toolkit')) return line;
      return rename(line);
    }).join('');
    if (result !== source) { fs.writeFileSync(file, result); changed.push(path.relative(root, file)); }
  }
}
for (const name of ['src', 'libs', 'scripts', '.cargo']) walk(path.join(root, name));
for (const name of ['Cargo.toml', 'package.json', 'tsconfig.json']) {
  const file = path.join(root, name);
  const source = fs.readFileSync(file, 'utf8');
  const result = source.split(/(?<=\n)/).map(line => line.includes('@seelen/translation-toolkit') || line.includes('https://github.com/Seelen-Inc/') ? line : rename(line)).join('');
  if (result !== source) { fs.writeFileSync(file, result); changed.push(name); }
}
// Directory and component filenames follow the references rewritten above.
function renameEntries(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (ignored.has(entry.name)) continue;
    const source = path.join(directory, entry.name);
    if (entry.isDirectory()) renameEntries(source);
    const newName = entry.name === 'slu' ? 'macui-cli' : rename(entry.name);
    if (newName !== entry.name) {
      const target = path.join(directory, newName);
      if (fs.existsSync(target)) throw new Error(`Rename conflict: ${target}`);
      fs.renameSync(source, target);
    }
  }
}
for (const name of ['src', 'libs']) renameEntries(path.join(root, name));
console.log(`Rewrote ${changed.length} source files; generated output is rebuilt separately.`);
