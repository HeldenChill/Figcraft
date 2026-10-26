'use strict';

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { resolveInside } = require('./bug-memory');

function normalizePath(p) {
  return p.replace(/\\/g, '/');
}

function sha256(buf) {
  return crypto.createHash('sha256').update(buf).digest('hex');
}

function hashFile(p) {
  if (!fs.existsSync(p)) return null;
  return sha256(fs.readFileSync(p));
}

function atomicWrite(target, content) {
  const dir = path.dirname(target);
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  const tmp = target + '.' + crypto.randomBytes(6).toString('hex') + '.tmp';
  fs.writeFileSync(tmp, content);
  fs.renameSync(tmp, target);
}

function readJson(p) {
  return JSON.parse(fs.readFileSync(p, 'utf8'));
}

function prepareTransaction(root, outDir, manifest) {
  if (!manifest || manifest.schemaVersion !== 2) {
    throw new Error('Manifest must have schemaVersion 2');
  }
  if (!Array.isArray(manifest.writes)) {
    throw new Error('Manifest writes must be an array');
  }

  // Check inventorySha256 if present
  if (manifest.inventorySha256) {
    const invPath = path.join(outDir, 'inventory.json');
    if (fs.existsSync(invPath)) {
      const curInvHash = hashFile(invPath);
      if (curInvHash !== manifest.inventorySha256) {
        throw new Error(`Inventory file hash mismatch: expected ${manifest.inventorySha256}, got ${curInvHash}`);
      }
    }
  }

  // Check mappingSha256 if present
  if (manifest.mappingSha256) {
    const mapPath = path.join(outDir, 'reconciliation.json');
    if (fs.existsSync(mapPath)) {
      const curMapHash = hashFile(mapPath);
      if (curMapHash !== manifest.mappingSha256) {
        throw new Error(`Mapping file hash mismatch: expected ${manifest.mappingSha256}, got ${curMapHash}`);
      }
    }
  }

  // Check source guards if present
  if (Array.isArray(manifest.sources)) {
    for (const src of manifest.sources) {
      const srcPath = resolveInside(root, src.path);
      const currentHash = hashFile(srcPath);
      if (currentHash !== src.sha256) {
        throw new Error(`Source file hash mismatch for ${src.path}: expected ${src.sha256}, got ${currentHash}`);
      }
    }
  }

  // Check staged files exist and match afterHash
  const stagedDirReal = fs.existsSync(path.join(outDir, 'staged')) ? fs.realpathSync(path.join(outDir, 'staged')) : path.resolve(outDir, 'staged');
  for (const w of manifest.writes) {
    const stagedPath = w.stagedPath ? path.resolve(w.stagedPath) : resolveInside(outDir, path.join('staged', w.path));
    if (!fs.existsSync(stagedPath)) {
      throw new Error(`Staged output missing: ${w.path} at ${stagedPath}`);
    }
    const realStaged = fs.realpathSync(stagedPath);
    const relStaged = path.relative(stagedDirReal, realStaged);
    if (relStaged.startsWith('..') || path.isAbsolute(relStaged) || relStaged === '') {
      const err = new Error(`Staged output path escapes staged directory: ${w.path}`);
      err.kind = 'stagedPathEscape';
      throw err;
    }
    const stagedBytes = fs.readFileSync(stagedPath);
    const stagedHash = sha256(stagedBytes);
    if (stagedHash !== w.afterHash) {
      const err = new Error(`Staged output hash mismatch for ${w.path}: expected ${w.afterHash}, got ${stagedHash}`); err.kind = 'stagedHashMismatch'; throw err;
    }
  }

  return {
    prepared: true,
    writeCount: manifest.writes.length,
    manifest
  };
}

function applyTransaction(root, outDir, manifest, options = {}) {
  if (!manifest) {
    const manifestPath = path.join(outDir, 'prepared-manifest.json');
    if (!fs.existsSync(manifestPath)) {
      throw new Error('Prepared manifest missing: ' + manifestPath);
    }
    manifest = readJson(manifestPath);
  }
  prepareTransaction(root, outDir, manifest);

  // Guard directory membership: verify no unregistered domain or history files were added since prepare
  const bugsDir = resolveInside(root, '.cursor/memory/bugs');
  if (fs.existsSync(bugsDir)) {
    function walkDir(dir) {
      const results = [];
      for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, ent.name);
        if (ent.isDirectory()) {
          results.push(...walkDir(full));
        } else {
          results.push(full);
        }
      }
      return results;
    }
    const allFilesOnDisk = walkDir(bugsDir);
    const knownPaths = new Set();
    if (Array.isArray(manifest.sources)) {
      for (const s of manifest.sources) {
        knownPaths.add(normalizePath(path.resolve(root, s.path)));
      }
    }
    if (Array.isArray(manifest.writes)) {
      for (const w of manifest.writes) {
        knownPaths.add(normalizePath(path.resolve(root, w.path)));
      }
    }
    for (const f of allFilesOnDisk) {
      const normF = normalizePath(f);
      if (!knownPaths.has(normF)) {
        const rel = normalizePath(path.relative(root, f));
        const err = new Error(`Unregistered file added since prepare: ${rel}`);
        err.kind = 'sourceMembershipChanged';
        err.path = rel;
        throw err;
      }
    }
  }

  // Mandatory preflight check for recovery manifests
  if ((manifest.migrationType === 'recovery' || manifest.inventorySha256) && !options.validatedContract) {
    const recoveryContract = require('./recovery-contract.js');
    const ctx = recoveryContract.loadRecoveryContract(root, { out: outDir, phase: 'staged' });
    const val = recoveryContract.validateRecoveryContract(ctx, { phase: 'staged' });
    if (!val.ok) {
      const err = new Error(`Apply preflight conservation failed: ${val.errors.map(e => e.detail || e.kind).join('; ')}`);
      err.kind = val.errors[0]?.kind || 'evidencePreservation';
      err.errors = val.errors;
      throw err;
    }
  }

  const preparedBytes = new Map();
  for (const w of manifest.writes) {
    const stagedPath = w.stagedPath ? path.resolve(w.stagedPath) : resolveInside(outDir, path.join('staged', w.path));
    preparedBytes.set(w.path, fs.readFileSync(stagedPath));
  }

  // Check if project is already migrated/recovered
  if (manifest.writes.every(w => hashFile(resolveInside(root, w.path)) === w.afterHash)) {
    return { state: 'alreadyApplied', filesWritten: [] };
  }

  // Destination guard: ensure destination matches beforeHash
  for (const w of manifest.writes) {
    const currentHash = hashFile(resolveInside(root, w.path));
    if (currentHash !== w.beforeHash) {
      throw new Error(`Destination changed since prepare for ${w.path}: expected ${w.beforeHash}, got ${currentHash}`);
    }
  }

  const journalFile = path.join(outDir, 'rollback-manifest.json');
  if (fs.existsSync(journalFile)) {
    const existing = readJson(journalFile);
    if (existing.state !== 'rolledBack') {
      throw new Error('Existing rollback journal; finish or rollback that run first');
    }
  }

  const journal = {
    schemaVersion: 2,
    state: 'prepared',
    filesWritten: [],
    filesBackedUp: [],
    writes: manifest.writes
  };

  // Back up every existing destination before the first live write
  for (const w of manifest.writes) {
    if (w.beforeHash !== null) {
      const target = resolveInside(root, w.path);
      const backup = resolveInside(outDir, normalizePath(path.join('backups', w.path)));
      if (fs.existsSync(backup) && hashFile(backup) !== w.beforeHash) {
        throw new Error('Unowned backup differs: ' + w.path);
      }
      atomicWrite(backup, fs.readFileSync(target));
      journal.filesBackedUp.push({ rel: w.path, hash: w.beforeHash });
    }
  }
  atomicWrite(journalFile, JSON.stringify(journal, null, 2) + '\n');

  try {
    for (const w of manifest.writes) {
      const targetPath = resolveInside(root, w.path);
      if (hashFile(targetPath) !== w.beforeHash) {
        throw new Error('Destination changed during apply: ' + w.path);
      }
      journal.pending = w.path;
      atomicWrite(journalFile, JSON.stringify(journal, null, 2) + '\n');

      atomicWrite(targetPath, preparedBytes.get(w.path));
      journal.filesWritten.push({ rel: w.path, hash: w.afterHash });
      delete journal.pending;
      atomicWrite(journalFile, JSON.stringify(journal, null, 2) + '\n');

      if (Number(process.env.BUG_MEMORY_FAIL_AFTER_WRITES) === journal.filesWritten.length) {
        throw new Error('Injected apply interruption');
      }
    }
    journal.state = 'applied';
    atomicWrite(journalFile, JSON.stringify(journal, null, 2) + '\n');
  } catch (err) {
    journal.state = 'interrupted';
    atomicWrite(journalFile, JSON.stringify(journal, null, 2) + '\n');
    try {
      rollbackTransaction(root, outDir);
    } catch (rollbackError) {
      throw new Error(err.message + '; rollback blocked: ' + rollbackError.message);
    }
    throw new Error(err.message + '; completed writes rolled back');
  }

  return { state: 'applied', filesWritten: journal.filesWritten };
}

function rollbackTransaction(root, outDir, options = {}) {
  const journalFile = path.join(outDir, 'rollback-manifest.json');
  if (!fs.existsSync(journalFile)) {
    throw new Error('No rollback journal found at: ' + journalFile);
  }
  const journal = readJson(journalFile);
  if (journal.state === 'rolledBack') {
    return { state: 'rolledBack', filesRestored: journal.filesRestored || [] };
  }

  journal.filesRestored = journal.filesRestored || [];
  const backed = new Map(journal.filesBackedUp.map(b => [b.rel, b]));
  const restored = new Map(journal.filesRestored.map(r => [r.rel, r.hash]));
  const written = [...journal.filesWritten];

  if (journal.pending) {
    const w = journal.writes.find(x => x.path === journal.pending);
    if (w) {
      const hash = hashFile(resolveInside(root, w.path));
      if (hash === w.afterHash && !written.some(i => i.rel === w.path)) {
        written.push({ rel: w.path, hash: w.afterHash });
      } else if (hash !== w.beforeHash) {
        throw new Error('Pending output modified: ' + w.path);
      }
    }
  }

  // Validate every destination and backup before restoring any file
  for (const item of written) {
    const current = hashFile(resolveInside(root, item.rel));
    const before = backed.has(item.rel) ? backed.get(item.rel).hash : null;
    if (journal.pendingRestore === item.rel && current === before && !restored.has(item.rel)) {
      restored.set(item.rel, before);
      journal.filesRestored.push({ rel: item.rel, hash: before });
    }
    const expected = restored.has(item.rel) ? restored.get(item.rel) : item.hash;
    if (current !== expected) {
      throw new Error('Cannot rollback modified or missing destination: ' + item.rel);
    }
  }

  for (const item of journal.filesRestored) {
    if (hashFile(resolveInside(root, item.rel)) !== item.hash) {
      throw new Error('Restored destination changed: ' + item.rel);
    }
  }

  const backupBytes = new Map();
  for (const bak of journal.filesBackedUp) {
    const file = resolveInside(outDir, normalizePath(path.join('backups', bak.rel)));
    const bytes = fs.readFileSync(file);
    if (sha256(bytes) !== bak.hash) {
      throw new Error('Backup hash mismatch: ' + bak.rel);
    }
    backupBytes.set(bak.rel, bytes);
  }

  journal.state = 'rollingBack';
  delete journal.pendingRestore;
  atomicWrite(journalFile, JSON.stringify(journal, null, 2) + '\n');

  for (const item of [...written].reverse()) {
    if (restored.has(item.rel)) continue;
    const target = resolveInside(root, item.rel);
    if (hashFile(target) !== item.hash) {
      throw new Error('Destination changed during rollback: ' + item.rel);
    }
    journal.pendingRestore = item.rel;
    atomicWrite(journalFile, JSON.stringify(journal, null, 2) + '\n');

    if (backed.has(item.rel)) {
      atomicWrite(target, backupBytes.get(item.rel));
    } else {
      if (fs.existsSync(target)) fs.unlinkSync(target);
    }
    const before = backed.has(item.rel) ? backed.get(item.rel).hash : null;
    restored.set(item.rel, before);
    journal.filesRestored.push({ rel: item.rel, hash: before });
    delete journal.pendingRestore;
    atomicWrite(journalFile, JSON.stringify(journal, null, 2) + '\n');

    if (Number(process.env.BUG_MEMORY_FAIL_AFTER_ROLLBACK_WRITES) === journal.filesRestored.length) {
      throw new Error('Injected rollback interruption');
    }
  }

  journal.state = 'rolledBack';
  delete journal.pending;
  atomicWrite(journalFile, JSON.stringify(journal, null, 2) + '\n');

  return { state: 'rolledBack', filesRestored: journal.filesRestored };
}

function verifyTransaction(root, outDir, manifest) {
  const results = [];
  for (const w of manifest.writes) {
    const target = resolveInside(root, w.path);
    const currentHash = hashFile(target);
    results.push({
      path: w.path,
      expected: w.afterHash,
      actual: currentHash,
      matches: currentHash === w.afterHash
    });
  }
  const allMatch = results.every(r => r.matches);
  return { ok: allMatch, results };
}

module.exports = {
  prepareTransaction,
  applyTransaction,
  rollbackTransaction,
  verifyTransaction,
  sha256,
  hashFile,
  atomicWrite
};
