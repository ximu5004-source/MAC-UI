// Create a clean, auditable source snapshot without local credentials, build
// outputs or private screenshots. Does not commit, push or create a repository.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, lstatSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const root = realpathSync(process.cwd());
const version = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8')).version;
const snapshotName = `MAC-UI-${version}-public`;
const output = path.resolve(root, 'target', 'github-publication', snapshotName);
if (!existsSync(path.join(root, 'src', 'tauri.conf.json'))) throw new Error('Run from the MAC UI repository root');
if (existsSync(output)) throw new Error('Refusing to overwrite an existing publication snapshot');
const gitOutput = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '-z'], { cwd: root });
const candidates = [...new Set(gitOutput.toString('utf8').split('\0').filter(Boolean))].sort();
const excluded = [];
const files = [];
const sensitive = /^(\.cert|\.env(?:\..*)?|target|dist|release|gen|src\/runtime)(\/|$)|(^|\/)(\.git|node_modules)(\/|$)|\.(pfx|p12|pem|key|pwd|exe|dll|pdb)$/i;

for (const relative of candidates) {
  if (sensitive.test(relative)) { excluded.push(relative); continue; }
  const source = path.resolve(root, relative);
  if (!source.startsWith(root + path.sep)) throw new Error(`Unsafe path: ${relative}`);
  if (!existsSync(source)) continue; // A deleted tracked file is not part of this version.
  const stats = lstatSync(source);
  if (!stats.isFile() || stats.isSymbolicLink()) throw new Error(`Not a regular source file: ${relative}`);
  if (stats.size > 50 * 1024 * 1024) throw new Error(`Unexpectedly large source file: ${relative}`);
  // Preserve upstream CI recipes as inert references. Their old release and
  // nightly jobs must not trigger with upstream signing/publishing identities.
  const destinationRelative = relative.startsWith('.github/workflows/')
    ? relative.replace('.github/workflows/', '.github/upstream-workflows/') + '.disabled'
    : relative;
  const destination = path.resolve(output, destinationRelative);
  if (!destination.startsWith(output + path.sep)) throw new Error('Unsafe destination');
  mkdirSync(path.dirname(destination), { recursive: true });
  copyFileSync(source, destination);
  const original = readFileSync(source);
  const copied = readFileSync(destination);
  if (!original.equals(copied)) throw new Error(`Copy mismatch: ${relative}`);
  files.push({ source: relative, path: destinationRelative, bytes: copied.length,
    sha256: createHash('sha256').update(copied).digest('hex') });
}

for (const required of ['LICENSE', 'NOTICE.md', 'README.md', 'README.zh-CN.md', 'README.upstream.md', 'Cargo.lock', 'package-lock.json', 'documentation/releases/3.0.1.md']) {
  if (!files.some(file => file.path === required)) throw new Error(`Missing publication source: ${required}`);
}
for (const image of ['desktop-overview', 'desktop-stacks', 'launchpad', 'control-center', 'power-session', 'desktop-settings', 'dock-settings']) {
  if (!files.some(file => file.path === `documentation/images/mac-ui/${image}.png`)) throw new Error(`Missing redacted image: ${image}`);
}

const upstreamBase = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
writeFileSync(path.join(output, 'SOURCE-PROVENANCE.json'), JSON.stringify({
  product: 'MAC UI', version, maintainer: 'JONA',
  upstream: 'https://github.com/eythaann/Seelen-UI', upstreamBase,
  note: 'Source snapshot of MAC UI modifications. Upstream CI recipes are preserved as inert references; certificates, private signing keys, build outputs and local installer copies are excluded.',
}, null, 2) + '\n');
writeFileSync(path.join(output, '..', `MAC-UI-${version}-publication-manifest.json`), JSON.stringify({
  version, upstreamBase, files, excluded,
}, null, 2) + '\n');
console.log(JSON.stringify({ version, output, files: files.length,
  totalBytes: files.reduce((total, file) => total + file.bytes, 0),
  excluded, archivedWorkflows: files.filter(file => file.path.startsWith('.github/upstream-workflows/')).length }, null, 2));
