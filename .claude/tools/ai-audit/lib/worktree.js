'use strict';
const fs = require('fs');
const path = require('path');
const { execFileSync, spawnSync } = require('child_process');
const { toSlash } = require('./index');

// Sparse patterns: code, config and docs only. Keeps a 20 GB Unity checkout to a few hundred MB.
const SPARSE = [
  '/*', '!/*/', '/.claude/', '/.cursor/', '/.codex/', '/.agents/', '/docs/', '/Packages/', '/ProjectSettings/',
  '/Assets/**/*.cs', '/Assets/**/*.asmdef', '/Assets/**/*.asmref', '/Assets/**/*.md',
];

const git = (cwd, ...args) => execFileSync('git', args, { cwd, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 });

// The worktree always moves to the project's CURRENT commit (Review Focus 8): a worktree left on the
// commit it was created at would make every P2–P5 re-run re-measure the baseline config.
// Only committed config is measured; uncommitted edits in the project are invisible here.
function ensureWorktree(project, wt, { sparse }) {
  const head = git(project, 'rev-parse', 'HEAD').trim();
  if (!fs.existsSync(wt)) {
    git(project, 'worktree', 'add', '--detach', '--no-checkout', wt, head);
    if (sparse) {
      git(wt, 'sparse-checkout', 'init', '--no-cone');
      git(wt, 'sparse-checkout', 'set', '--no-cone', ...SPARSE);
    }
  }
  git(wt, 'checkout', '-q', '--force', '--detach', head);
  for (const f of fs.readdirSync(project)) {
    if (/\.(csproj|sln|slnx)$/.test(f)) fs.copyFileSync(path.join(project, f), path.join(wt, f));
  }
  const lib = path.join(wt, 'Library');
  if (fs.existsSync(path.join(project, 'Library')) && !fs.existsSync(lib)) {
    spawnSync('cmd', ['/c', 'mklink', '/J', lib, path.join(project, 'Library')], { stdio: 'ignore' });
  }
}

function resetWorktree(wt) {
  git(wt, 'reset', '--hard', '-q');
  git(wt, 'clean', '-fdq'); // no -x: ignored Library junction and csproj copies survive
}

function changedFiles(wt) {
  return git(wt, 'status', '--porcelain', '-uall')
    .split(/\r?\n/).filter(Boolean)
    .map((l) => toSlash(l.slice(3).replace(/^"|"$/g, '').split(' -> ').pop()));
}

function addedLines(wt, rel) {
  const tracked = spawnSync('git', ['ls-files', '--error-unmatch', rel], { cwd: wt }).status === 0;
  if (!tracked) {
    try { return fs.readFileSync(path.join(wt, rel), 'utf8'); } catch { return ''; }
  }
  return git(wt, 'diff', '--', rel).split(/\r?\n/)
    .filter((l) => l.startsWith('+') && !l.startsWith('+++')).join('\n');
}

module.exports = { ensureWorktree, resetWorktree, changedFiles, addedLines };
