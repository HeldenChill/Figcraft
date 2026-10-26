'use strict';
const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

// A worktree session shares the project's auto-memory dir, so a scenario that saves memory
// (F11, F12) would leak into the owner's real memory. Snapshot before a run, restore after.
function guardDir(dir) {
  const existed = fs.existsSync(dir);
  const snap = new Map();
  if (existed) for (const f of fs.readdirSync(dir)) snap.set(f, fs.readFileSync(path.join(dir, f)));
  return {
    restore() {
      if (!existed) { fs.rmSync(dir, { recursive: true, force: true }); return; }
      for (const f of fs.readdirSync(dir)) if (!snap.has(f)) fs.rmSync(path.join(dir, f), { recursive: true, force: true });
      for (const [f, buf] of snap) {
        const p = path.join(dir, f);
        if (!fs.existsSync(p) || !fs.readFileSync(p).equals(buf)) fs.writeFileSync(p, buf);
      }
    },
  };
}

// Resolve explicit guard targets before any audit worktree/output mutation.
// shared-config is portable: CLI --shared-repo overrides SHARED_CONFIG_DIR.
function validateGuardRepo(repo, { clean = true } = {}) {
  if (!fs.existsSync(repo)) throw new Error(`guarded repo does not exist: ${repo}`);
  const actual = fs.realpathSync(repo);
  const git = (...args) => execFileSync('git', args, { cwd: actual, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  const root = fs.realpathSync(git('rev-parse', '--show-toplevel').trim());
  const key = (p) => process.platform === 'win32' ? p.toLowerCase() : p;
  if (key(actual) !== key(root)) throw new Error(`guarded path must be the repository root: ${repo}`);
  if (clean && git('status', '--porcelain').trim()) throw new Error(`guarded repo is dirty: ${actual}`);
  return actual;
}

function resolveGuardRepos(repos, { project = process.cwd(), sharedRepo, env = process.env } = {}) {
  return repos.map((spec) => {
    if (typeof spec !== 'string' || !spec.trim()) throw new Error('guarded repository path must be a non-empty string');
    const value = spec === 'shared-config' ? sharedRepo || env.SHARED_CONFIG_DIR : spec;
    if (!value) throw new Error('shared-config guard requires --shared-repo or SHARED_CONFIG_DIR');
    return validateGuardRepo(path.resolve(project, value));
  });
}

// Models may edit a sibling repo. Validate exact root and cleanliness first;
// recheck root before restore so Git cannot reset an unintended ancestor repo.
function guardRepo(repo) {
  const actual = validateGuardRepo(repo);
  const git = (...args) => execFileSync('git', args, { cwd: actual, encoding: 'utf8' });
  return { restore() {
    validateGuardRepo(actual, { clean: false });
    git('reset', '--hard', '-q'); git('clean', '-fdq');
  } };
}

module.exports = { guardDir, guardRepo, resolveGuardRepos };
