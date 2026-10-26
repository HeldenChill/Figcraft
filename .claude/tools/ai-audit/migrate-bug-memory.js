#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const bugs = require('./lib/bug-memory');

function normalizePath(p) {
  return p.replace(/\\/g, '/');
}

function sha256(buf) {
  return crypto.createHash('sha256').update(buf).digest('hex');
}

function readJson(file) { return JSON.parse(fs.readFileSync(file, 'utf8')); }
function hashFile(file) { return fs.existsSync(file) ? sha256(fs.readFileSync(file)) : null; }
function atomicWrite(file, bytes) {
  fs.mkdirSync(path.dirname(file), {recursive:true});
  const temp = file + '.migration-' + crypto.randomBytes(8).toString('hex');
  try { fs.writeFileSync(temp,bytes); fs.renameSync(temp,file); }
  finally { if (fs.existsSync(temp)) fs.unlinkSync(temp); }
}
function listFiles(root, dir = root) {
  const result = [];
  for (const e of fs.readdirSync(dir,{withFileTypes:true}).sort((a,b)=>a.name.localeCompare(b.name))) {
    const full = path.join(dir,e.name);
    const rel = normalizePath(path.relative(root,full));
    bugs.resolveInside(root,rel);
    if (e.isSymbolicLink()) throw new Error('Migration staging cannot contain symlinks: '+rel);
    if (e.isDirectory()) result.push(...listFiles(root,full));
    else if (e.isFile()) result.push(rel);
    else throw new Error('Unsupported staged entry: '+rel);
  }
  return result;
}
function expectedRecords(inventory, reconciliation) {
  if (inventory.sourceSha256 !== reconciliation.sourceSha256) throw new Error('Reconciliation source hash mismatch');
  const keys = new Set(inventory.occurrences.map(o=>o.key));
  const mappings = new Map();
  for (const m of reconciliation.occurrences || []) {
    if (!keys.has(m.key) || mappings.has(m.key)) throw new Error('Unknown or duplicate reconciliation key: '+m.key);
    // Merges/history reclassification must be explicitly implemented, never silently discarded.
    if (m.classification !== 'record') throw new Error('Unsupported classification requires explicit conservation implementation: '+m.classification);
    if (!/^[a-z0-9][a-z0-9_-]*$/i.test(m.domain || '')) throw new Error('Invalid domain: '+m.domain);
    mappings.set(m.key,m);
  }
  const repairs = new Map();
  for (const r of reconciliation.fieldRepairs || []) {
    if (!r.key || !keys.has(r.key) || repairs.has(r.key)) throw new Error('Field repair needs unique occurrence key');
    repairs.set(r.key,r);
  }
  const result = [];
  for (const o of inventory.occurrences) {
    const m = mappings.get(o.key);
    if (!m) throw new Error('Unmapped occurrence: '+o.key);
    const repair = repairs.get(o.key);
    const cells = [...(repair ? repair.cells : m.cells || m.repairedCells || o.cells)];
    if (cells.length !== 10) throw new Error('Malformed occurrence needs reviewed ten-cell repair: '+o.key);
    if (m.canonicalId) cells[0] = m.canonicalId;
    result.push({key:o.key,id:cells[0],cells,source:`.cursor/memory/bugs/mem-bugs-${m.domain.toLowerCase().trim()}.md`,cellsSha256:sha256(JSON.stringify(cells))});
  }
  if (new Set(result.map(r=>r.id)).size !== result.length) throw new Error('Reconciliation leaves duplicate IDs');
  return result;
}
function conservationCheck(root, outDir, reconciliation, inventory) {
  const expected = expectedRecords(inventory,reconciliation);
  const snapshot = bugs.readProjectLedger(root);
  const validation = bugs.validateLedger(snapshot);
  const errors = [...validation.errors];
  let missing = 0;
  let sourceOccurrencesCount = inventory.occurrences.length;
  const originalFile = path.join(outDir,'original-ledger.txt');
  if (fs.existsSync(originalFile)) {
    const bytes = fs.readFileSync(originalFile);
    if (sha256(bytes)!==inventory.sourceSha256) errors.push({kind:'originalSourceHashMismatch',detail:'Original snapshot differs from inventory'});
    else {
      const rescanned = bugs.scanText(bytes.toString('utf8'),inventory.source);
      sourceOccurrencesCount = rescanned.occurrences.length;
      const oldRows = new Map(inventory.occurrences.map(o=>[o.key,o]));
      for (const o of rescanned.occurrences) {
        if (!oldRows.has(o.key)) {missing++;errors.push({kind:'sourceOccurrenceOmitted',detail:o.key+' '+o.id});}
        else if (JSON.stringify(oldRows.get(o.key).cells)!==JSON.stringify(o.cells)) errors.push({kind:'sourceOccurrenceChanged',detail:o.key+' '+o.id});
      }
    }
  }
  const expectedIds = new Set(expected.map(r=>r.id));
  for (const exp of expected) {
    const matches = snapshot.records.filter(r=>r.id===exp.id && r.file===exp.source);
    if (matches.length !== 1) {missing++;errors.push({kind:'missingRecordMapping',detail:exp.key+' -> '+exp.id});}
    else if (sha256(JSON.stringify(matches[0].cells)) !== exp.cellsSha256) errors.push({kind:'recordEvidenceChanged',detail:exp.key+' -> '+exp.id});
  }
  for (const r of snapshot.records) if (!expectedIds.has(r.id)) errors.push({kind:'unmappedOutputRecord',detail:r.id});
  const oldHistory = (inventory.history || []).map(h=>JSON.stringify(h.cells));
  const newHistory = snapshot.history.map(h=>JSON.stringify(h.cells));
  if (JSON.stringify(oldHistory) !== JSON.stringify(newHistory)) errors.push({kind:'historyChanged',detail:'Historical mapping cells differ from inventory'});
  const historyPreserved = !errors.some(e=>e.kind==='historyChanged');
  return {snapshot,validation,expected,conservation:{schemaVersion:1,sourceOccurrencesCount,inventoryOccurrencesCount:inventory.occurrences.length,authoritativeCount:snapshot.records.length,unmappedCount:missing,errorsCount:errors.length,warningsCount:validation.warnings.length,errors,warnings:validation.warnings,historyPreserved,recordMapping:expected.map(({cells,...r})=>r)}};
}

function parseArgs(args) {
  const cmd = args[0];
  const options = {
    command: cmd,
    project: '.',
    out: null,
    mapping: null,
    reportOut: null,
    staged: false
  };

  for (let i = 1; i < args.length; i++) {
    const a = args[i];
    if (a === '--project' && i + 1 < args.length) options.project = args[++i];
    else if (a === '--out' && i + 1 < args.length) options.out = args[++i];
    else if (a === '--mapping' && i + 1 < args.length) options.mapping = args[++i];
    else if (a === '--report-out' && i + 1 < args.length) options.reportOut = args[++i];
    else if (a === '--staged') options.staged = true;
  }
  return options;
}

function cmdInventory(root, outDir) {
  fs.mkdirSync(outDir, { recursive: true });
  const config = bugs.readConfig(root);
  const ledgerPath = config.ledger || '.cursor/memory/mem-known-bugs-index.md';
  const fullLedger = path.join(root, ledgerPath);
  if (!fs.existsSync(fullLedger)) {
    throw new Error('Ledger source file does not exist: ' + fullLedger);
  }
  const bytes = fs.readFileSync(fullLedger);
  const sourceHash = sha256(bytes);
  const text = bytes.toString('utf8');

  const scan = bugs.scanText(text, ledgerPath);
  const snapshot = bugs.readProjectLedger(root);
  const validationBefore = bugs.validateLedger(snapshot);

  const inventory = {
    schemaVersion: 1,
    source: normalizePath(ledgerPath),
    sourceSha256: sourceHash,
    bytes: bytes.length,
    occurrencesCount: scan.occurrences.length,
    historyCount: scan.history.length,
    findingsCount: scan.findings.length,
    occurrences: scan.occurrences,
    history: scan.history,
    findings: scan.findings
  };

  fs.writeFileSync(path.join(outDir, 'inventory.json'), JSON.stringify(inventory, null, 2) + '\n');
  const original = path.join(outDir,'original-ledger.txt');
  if (fs.existsSync(original) && hashFile(original)!==sourceHash) throw new Error('Existing source snapshot differs; use a new evidence run');
  if (!fs.existsSync(original)) fs.writeFileSync(original,bytes);
  fs.writeFileSync(path.join(outDir, 'validation-before.json'), JSON.stringify(validationBefore, null, 2) + '\n');
  console.log(`Inventory complete: ${scan.occurrences.length} occurrences, ${scan.history.length} history items recorded.`);
}

function cmdPrepare(root, outDir, mappingPath) {
  if (!mappingPath || !fs.existsSync(mappingPath)) {
    throw new Error('Mapping reconciliation file is required: ' + mappingPath);
  }
  const invPath = path.join(outDir, 'inventory.json');
  if (!fs.existsSync(invPath)) {
    throw new Error('Inventory must be run before prepare: missing ' + invPath);
  }
  const inventory = JSON.parse(fs.readFileSync(invPath, 'utf8'));
  const reconciliation = JSON.parse(fs.readFileSync(mappingPath, 'utf8'));
  if ((reconciliation.linkRepairs || []).length || (reconciliation.merges || []).length || (reconciliation.renumberings || []).length) {
    throw new Error('Link repairs, merges and renumberings require explicit conservation implementation');
  }

  const fullLedger = path.join(root, inventory.source);
  const currentBytes = fs.readFileSync(fullLedger);
  const currentHash = sha256(currentBytes);
  if (currentHash !== inventory.sourceSha256 || currentHash !== reconciliation.sourceSha256) {
    throw new Error(`Source hash mismatch: file was modified since inventory (${currentHash} != ${inventory.sourceSha256})`);
  }

  const expected = expectedRecords(inventory,reconciliation);
  const stagedRoot = path.join(outDir, 'staged-project');
  if (fs.existsSync(stagedRoot)) {
    // A previous preparation is immutable; use a fresh evidence run instead of retaining stale files.
    throw new Error('Staged project already exists; use a new evidence directory');
  }
  fs.mkdirSync(stagedRoot, { recursive: true });

  // Map occurrences
  const mappingByKey = new Map();
  for (const m of reconciliation.occurrences) {
    mappingByKey.set(m.key, m);
  }

  const fieldRepairByKey = new Map();
  for (const fr of (reconciliation.fieldRepairs || [])) {
    if (fr.key) fieldRepairByKey.set(fr.key, fr);
    if (fr.id) fieldRepairByKey.set(fr.id, fr);
  }

  const domainRecords = new Map();
  let unmappedCount = 0;

  for (const occ of inventory.occurrences) {
    const mapEntry = mappingByKey.get(occ.key);
    if (!mapEntry) {
      unmappedCount++;
      continue;
    }
    if (mapEntry.classification === 'record') {
      const d = mapEntry.domain.toLowerCase().trim();
      if (!domainRecords.has(d)) domainRecords.set(d, []);

      // If canonical ID or cell repairs exist
      let cells = [...occ.cells];
      const repair = fieldRepairByKey.get(occ.key) || fieldRepairByKey.get(occ.id);
      if (repair && Array.isArray(repair.cells) && repair.cells.length === 10) {
        cells = [...repair.cells];
      } else if (Array.isArray(mapEntry.cells) && mapEntry.cells.length === 10) {
        cells = [...mapEntry.cells];
      } else if (Array.isArray(mapEntry.repairedCells) && mapEntry.repairedCells.length === 10) {
        cells = [...mapEntry.repairedCells];
      }
      if (mapEntry.canonicalId) cells[0] = mapEntry.canonicalId;

      domainRecords.get(d).push({
        ...occ,
        id: cells[0],
        cells,
        raw: JSON.stringify(cells) === JSON.stringify(occ.cells) ? occ.raw : '| ' + cells.join(' | ') + ' |'
      });
    }
  }

  if (unmappedCount > 0) {
    throw new Error(`Cannot prepare: ${unmappedCount} occurrences have no reviewed reconciliation mapping`);
  }

  const header = '| ID | Type | Severity | Status | Title | Source | Evidence | Found | Updated | Resolution |';
  const separator = '|---|---|---|---|---|---|---|---|---|---|';

  // 1. Write domain files
  const domainList = Array.from(domainRecords.keys()).sort();
  for (const domain of domainList) {
    const records = domainRecords.get(domain);
    const domainTitle = domain.charAt(0).toUpperCase() + domain.slice(1);
    const domainFileRel = `.cursor/memory/bugs/mem-bugs-${domain}.md`;
    const fullDomainFile = path.join(stagedRoot, domainFileRel);
    fs.mkdirSync(path.dirname(fullDomainFile), { recursive: true });

    const content = [
      `# Memory: Bugs — ${domainTitle}`,
      '',
      `**ID:** mem-bugs-${domain}`,
      `**Parent:** bugs/README.md`,
      `**Status:** Living authoritative records for this domain.`,
      '',
      '## Ledger',
      '',
      header,
      separator,
      ...records.map(r => r.raw),
      ''
    ].join('\n');
    fs.writeFileSync(fullDomainFile, content);
  }

  // 2. Write bugs/README.md hub
  const hubRel = '.cursor/memory/bugs/README.md';
  const fullHub = path.join(stagedRoot, hubRel);
  fs.mkdirSync(path.dirname(fullHub), { recursive: true });
  const hubRows = domainList.map(d => {
    const dTitle = d.charAt(0).toUpperCase() + d.slice(1);
    return `| ${dTitle} | [${dTitle}](mem-bugs-${d}.md) | ${dTitle} domain records |`;
  });
  const hubContent = [
    '# Memory: Bugs Hub',
    '',
    '**ID:** bugs-hub',
    '**Parent:** .cursor/memory/mem-arch-assemblies-canonical.md',
    '**Status:** Authoritative domain index for project bugs.',
    '',
    '## Domains',
    '',
    '| Domain | File | Scope |',
    '|---|---|---|',
    ...hubRows,
    '',
    '## History and Mappings',
    '',
    'See [History](history/README.md) for historical ID renumberings and retired tables.',
    ''
  ].join('\n');
  fs.writeFileSync(fullHub, hubContent);

  // 3. Write bugs/history/README.md
  const historyRel = '.cursor/memory/bugs/history/README.md';
  const fullHistory = path.join(stagedRoot, historyRel);
  fs.mkdirSync(path.dirname(fullHistory), { recursive: true });
  const historyContent = [
    '# Memory: Bugs — Historical ID Renumberings and Mappings',
    '',
    'This directory preserves historical ID mappings, renumbering audits, and maintenance notes.',
    'It contains no authoritative living bug records.',
    '',
    '## ID renumbering',
    '',
    '| Old ID | New ID | Reason |',
    '|---|---|---|',
    ...inventory.history.map(h => h.raw),
    ''
  ].join('\n');
  fs.writeFileSync(fullHistory, historyContent);

  // 4. Write compatibility pointer at old ledger location
  const fullPointer = path.join(stagedRoot, inventory.source);
  fs.mkdirSync(path.dirname(fullPointer), { recursive: true });
  const pointerContent = [
    '# Memory: Known Bugs — Collection Entry',
    '',
    'The canonical bug collection is [Bugs](bugs/README.md).',
    'Search its domain files before filing or changing a record.',
    'Historical identity mappings are preserved in [History](bugs/history/README.md).',
    'This file is a compatibility entry point and contains no authoritative bug records.',
    ''
  ].join('\n');
  fs.writeFileSync(fullPointer, pointerContent);

  // 5. Update .claude/rules/project-values.md
  const valuesRel = '.claude/rules/project-values.md';
  let valuesText = '';
  if (fs.existsSync(path.join(root, valuesRel))) {
    valuesText = fs.readFileSync(path.join(root, valuesRel), 'utf8');
  }
  const configBlock = [
    '## bug-lifecycle-tracking',
    '```text',
    'BUG_LEDGER = .cursor/memory/bugs/README.md',
    'BUG_RECORDS_DIR = .cursor/memory/bugs',
    'AUDIT_ROOT = .cursor/audits',
    'BUG_ID_PREFIX = BUG-',
    '```'
  ].join('\n');

  let updatedValues;
  if (/##\s*bug-lifecycle-tracking[\s\S]*?(?=\n##\s|$)/i.test(valuesText)) {
    updatedValues = valuesText.replace(/##\s*bug-lifecycle-tracking[\s\S]*?(?=\n##\s|$)/i, configBlock);
  } else {
    updatedValues = (valuesText ? valuesText.trimEnd() + '\n\n' : '') + configBlock + '\n';
  }
  const fullStagedValues = path.join(stagedRoot, valuesRel);
  fs.mkdirSync(path.dirname(fullStagedValues), { recursive: true });
  fs.writeFileSync(fullStagedValues, updatedValues);

  // 6. Update master registry row in mem-arch-assemblies-canonical.md
  const indexRel = '.cursor/memory/mem-arch-assemblies-canonical.md';
  if (fs.existsSync(path.join(root, indexRel))) {
    let indexText = fs.readFileSync(path.join(root, indexRel), 'utf8');
    const hubRow = '| HUB: Bugs - domain records, lifecycle and audit evidence | `bugs/README.md` |';
    if (!indexText.includes('`bugs/README.md`')) {
      if (indexText.includes('<!-- LOCAL-MEMORY-INDEX:BEGIN -->')) {
        indexText = indexText.replace('<!-- LOCAL-MEMORY-INDEX:BEGIN -->', '<!-- LOCAL-MEMORY-INDEX:BEGIN -->\n' + hubRow);
      }
    }
    const fullStagedIndex = path.join(stagedRoot, indexRel);
    fs.mkdirSync(path.dirname(fullStagedIndex), { recursive: true });
    fs.writeFileSync(fullStagedIndex, indexText);
  }

  const manifest = {
    schemaVersion: 1,
    sourceSha256: inventory.sourceSha256,
    mappingSha256: sha256(fs.readFileSync(mappingPath)),
    inventorySha256: sha256(fs.readFileSync(invPath)),
    domains: domainList,
    recordCount: expected.length,
    source: inventory.source,
    mapping: normalizePath(path.relative(outDir,mappingPath)),
    recordMapping: expected.map(({cells,...r})=>r),
    writes: listFiles(stagedRoot).map(rel=>({path:rel,beforeHash:hashFile(bugs.resolveInside(root,rel)),afterHash:hashFile(bugs.resolveInside(stagedRoot,rel)),stagedPath:normalizePath(path.join('staged-project',rel)),reason:'Reviewed bug collection migration'}))
  };
  bugs.resolveInside(outDir,manifest.mapping);
  const stagedCheck = conservationCheck(stagedRoot,outDir,reconciliation,inventory);
  if (stagedCheck.conservation.errors.length) throw new Error('Prepared output failed validation/conservation: '+JSON.stringify(stagedCheck.conservation.errors));
  fs.writeFileSync(path.join(outDir, 'prepared-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
  console.log(`Prepare complete: ${manifest.recordCount} records organized across ${domainList.length} domains in staged-project.`);
}

function cmdVerify(root, outDir, staged, reportOut) {
  const targetRoot = staged ? bugs.resolveInside(outDir,'staged-project') : root;
  const inventory = readJson(path.join(outDir,'inventory.json'));
  const manifest = readJson(path.join(outDir,'prepared-manifest.json'));
  const mappingFile = bugs.resolveInside(outDir,manifest.mapping || 'reconciliation.json');
  if (hashFile(mappingFile) !== manifest.mappingSha256) throw new Error('Mapping hash mismatch');
  if (manifest.inventorySha256 && hashFile(path.join(outDir,'inventory.json'))!==manifest.inventorySha256) throw new Error('Inventory hash mismatch');
  const reconciliation = readJson(mappingFile);
  const result = conservationCheck(targetRoot,outDir,reconciliation,inventory);
  result.conservation.staged = Boolean(staged);
  const reportDir = reportOut ? bugs.resolveInside(root,normalizePath(path.relative(root,path.resolve(reportOut)))) : outDir;
  atomicWrite(path.join(reportDir,'conservation.json'),JSON.stringify(result.conservation,null,2)+'\n');
  atomicWrite(path.join(reportDir,staged?'validation-staged.json':'validation-after.json'),JSON.stringify(result.validation,null,2)+'\n');
  if (result.conservation.errors.length) throw new Error('Conservation failed: '+JSON.stringify(result.conservation.errors));
  console.log('Verification passed: '+result.snapshot.records.length+' records, '+result.validation.warnings.length+' warnings.');
}

function cmdApply(root, outDir, mappingPath) {
  const manifest = readJson(path.join(outDir,'prepared-manifest.json'));
  if (!manifest.source || !Array.isArray(manifest.writes) || !manifest.writes.length) throw new Error('Prepared manifest lacks guarded writes; prepare a new run');
  const stagedRoot = bugs.resolveInside(outDir,'staged-project');
  const mappingFile = mappingPath || bugs.resolveInside(outDir,manifest.mapping);
  if (hashFile(mappingFile) !== manifest.mappingSha256) throw new Error('Mapping hash mismatch');
  const inventory = readJson(path.join(outDir,'inventory.json'));
  if (manifest.sourceSha256 !== inventory.sourceSha256) throw new Error('Inventory source hash mismatch');
  const reconciliation = readJson(mappingFile);
  if (hashFile(path.join(outDir,'inventory.json'))!==manifest.inventorySha256) throw new Error('Inventory hash mismatch');
  const checked = conservationCheck(stagedRoot,outDir,reconciliation,inventory);
  if (checked.conservation.errors.length) throw new Error('Staged conservation failed: '+JSON.stringify(checked.conservation.errors));
  const paths = manifest.writes.map(w=>w.path);
  if (new Set(paths).size !== paths.length || JSON.stringify([...paths].sort()) !== JSON.stringify(listFiles(stagedRoot).sort())) throw new Error('Staged file set differs from prepared manifest');
  const preparedBytes = new Map();
  for (const w of manifest.writes) {
    bugs.resolveInside(root,w.path);
    const stagedFile = bugs.resolveInside(outDir,w.stagedPath);
    if (normalizePath(path.relative(stagedRoot,stagedFile)) !== w.path) throw new Error('Invalid staged path: '+w.path);
    const bytes = fs.readFileSync(stagedFile);
    if (sha256(bytes) !== w.afterHash) throw new Error('Staged output hash mismatch: '+w.path);
    preparedBytes.set(w.path,bytes);
  }
  if (manifest.writes.every(w=>hashFile(bugs.resolveInside(root,w.path))===w.afterHash)) {
    console.log('Project is already migrated. All prepared outputs match. Apply skipped.');return;
  }
  if (hashFile(bugs.resolveInside(root,manifest.source)) !== manifest.sourceSha256) throw new Error('Source hash mismatch after prepare');
  for (const w of manifest.writes) if (hashFile(bugs.resolveInside(root,w.path)) !== w.beforeHash) throw new Error('Destination changed since prepare: '+w.path);
  const journalFile = path.join(outDir,'rollback-manifest.json');
  if (fs.existsSync(journalFile)) throw new Error('Existing rollback journal; finish or rollback that run first');
  const journal = {schemaVersion:2,state:'prepared',filesWritten:[],filesBackedUp:[],writes:manifest.writes};
  // All backups and their hashes exist before the first live write.
  for (const w of manifest.writes) if (w.beforeHash !== null) {
    const target = bugs.resolveInside(root,w.path);
    const backup = bugs.resolveInside(outDir,normalizePath(path.join('backups',w.path)));
    if (fs.existsSync(backup) && hashFile(backup)!==w.beforeHash) throw new Error('Unowned backup differs: '+w.path);
    atomicWrite(backup,fs.readFileSync(target));
    journal.filesBackedUp.push({rel:w.path,hash:w.beforeHash});
  }
  atomicWrite(journalFile,JSON.stringify(journal,null,2)+'\n');
  try {
    for (const w of manifest.writes) {
      if (hashFile(bugs.resolveInside(root,w.path))!==w.beforeHash) throw new Error('Destination changed during apply: '+w.path);
      journal.pending = w.path;
      atomicWrite(journalFile,JSON.stringify(journal,null,2)+'\n');
      atomicWrite(bugs.resolveInside(root,w.path),preparedBytes.get(w.path));
      journal.filesWritten.push({rel:w.path,hash:w.afterHash});
      delete journal.pending;
      atomicWrite(journalFile,JSON.stringify(journal,null,2)+'\n');
      if (Number(process.env.BUG_MEMORY_FAIL_AFTER_WRITES)===journal.filesWritten.length) throw new Error('Injected apply interruption');
    }
    journal.state='applied';atomicWrite(journalFile,JSON.stringify(journal,null,2)+'\n');
  } catch (err) {
    journal.state='interrupted';atomicWrite(journalFile,JSON.stringify(journal,null,2)+'\n');
    try {cmdRollback(root,outDir);} catch (rollbackError) {throw new Error(err.message+'; rollback blocked: '+rollbackError.message);}
    throw new Error(err.message+'; completed writes rolled back');
  }
  console.log('Apply complete: wrote '+journal.filesWritten.length+' files.');
}

function cmdRollback(root, outDir) {
  const journalFile = path.join(outDir,'rollback-manifest.json');
  const journal = readJson(journalFile);
  if (journal.state==='rolledBack') {console.log('Rollback already complete.');return;}
  journal.filesRestored = journal.filesRestored || [];
  const backed = new Map(journal.filesBackedUp.map(b=>[b.rel,b]));
  const restored = new Map(journal.filesRestored.map(r=>[r.rel,r.hash]));
  const written = [...journal.filesWritten];
  if (journal.pending) {
    const w = journal.writes.find(w=>w.path===journal.pending);
    const hash = hashFile(bugs.resolveInside(root,w.path));
    if (hash===w.afterHash && !written.some(i=>i.rel===w.path)) written.push({rel:w.path,hash:w.afterHash});
    else if (hash!==w.beforeHash) throw new Error('Pending output modified: '+w.path);
  }
  // Validate every destination and backup before restoring any file.
  for (const item of written) {
    const current = hashFile(bugs.resolveInside(root,item.rel));
    const before = backed.has(item.rel) ? backed.get(item.rel).hash : null;
    if (journal.pendingRestore===item.rel && current===before && !restored.has(item.rel)) {
      restored.set(item.rel,before);journal.filesRestored.push({rel:item.rel,hash:before});
    }
    const expected = restored.has(item.rel) ? restored.get(item.rel) : item.hash;
    if (current!==expected) throw new Error('Cannot rollback modified or missing destination: '+item.rel);
  }
  // Restored files from a pending apply write are also checked on every retry.
  for (const item of journal.filesRestored) if (hashFile(bugs.resolveInside(root,item.rel))!==item.hash) throw new Error('Restored destination changed: '+item.rel);
  const backupBytes = new Map();
  for (const bak of journal.filesBackedUp) {
    const file = bugs.resolveInside(outDir,normalizePath(path.join('backups',bak.rel)));
    const bytes = fs.readFileSync(file);
    if (sha256(bytes)!==bak.hash) throw new Error('Backup hash mismatch: '+bak.rel);
    backupBytes.set(bak.rel,bytes);
  }
  journal.state='rollingBack';delete journal.pendingRestore;
  atomicWrite(journalFile,JSON.stringify(journal,null,2)+'\n');
  for (const item of written.reverse()) {
    if (restored.has(item.rel)) continue;
    const target = bugs.resolveInside(root,item.rel);
    if (hashFile(target)!==item.hash) throw new Error('Destination changed during rollback: '+item.rel);
    journal.pendingRestore=item.rel;
    atomicWrite(journalFile,JSON.stringify(journal,null,2)+'\n');
    if (backed.has(item.rel)) atomicWrite(target,backupBytes.get(item.rel));
    else fs.unlinkSync(target);
    const before = backed.has(item.rel) ? backed.get(item.rel).hash : null;
    restored.set(item.rel,before);journal.filesRestored.push({rel:item.rel,hash:before});
    delete journal.pendingRestore;
    atomicWrite(journalFile,JSON.stringify(journal,null,2)+'\n');
    if (Number(process.env.BUG_MEMORY_FAIL_AFTER_ROLLBACK_WRITES)===journal.filesRestored.length) throw new Error('Injected rollback interruption');
  }
  // Leave directories and every untracked file untouched.
  journal.state='rolledBack';delete journal.pending;
  atomicWrite(journalFile,JSON.stringify(journal,null,2)+'\n');
  console.log('Rollback complete: migration-owned files restored.');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const root = path.resolve(args.project);
  const outDir = args.out ? path.resolve(args.out) : path.join(root, '.cursor/audits/migration');

  try {
    switch (args.command) {
      case 'inventory':
        cmdInventory(root, outDir);
        break;
      case 'prepare':
        cmdPrepare(root, outDir, args.mapping ? path.resolve(args.mapping) : null);
        break;
      case 'verify':
        cmdVerify(root, outDir, args.staged, args.reportOut);
        break;
      case 'apply':
        cmdApply(root, outDir, args.mapping ? path.resolve(args.mapping) : null);
        break;
      case 'rollback':
        cmdRollback(root, outDir);
        break;
      default:
        console.error(`Unknown subcommand: ${args.command}. Use inventory|prepare|verify|apply|rollback.`);
        process.exit(1);
    }
  } catch (err) {
    console.error(`Error: ${err.message}`);
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}
