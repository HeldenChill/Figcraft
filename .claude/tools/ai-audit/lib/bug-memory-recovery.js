'use strict';

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {
  splitRow,
  resolveInside,
  validateLedger,
  countIds,
  readConfig,
  readProjectLedger,
  scanText
} = require('./bug-memory');

function normalizePath(p) {
  return p.replace(/\\/g, '/');
}
const {
  prepareTransaction,
  applyTransaction,
  rollbackTransaction,
  verifyTransaction,
  sha256,
  hashFile,
  atomicWrite
} = require('./bug-memory-transaction');

const VALID_TYPES = new Set(['bug', 'gap', 'planned', 'risk', 'perf', 'debt', 'flaky']);
const VALID_SEVERITIES = new Set(['low', 'medium', 'high', 'critical', 'unknown']);

function scanRecoverySource(text, source = '') {
  const rows = [];
  const blocks = [];
  const findings = [];
  const lines = text.split(/\r?\n/);

  let currentSection = '';
  let currentHeader = null;
  let inTable = false;
  let tableStartLine = -1;
  let tableLines = [];
  let inHistoryTable = false;
  let historySchema = null;

  function isNextLineSeparator(idx) {
    if (idx + 1 >= lines.length) return false;
    const nLine = lines[idx + 1].trim();
    if (!nLine.startsWith('|')) return false;
    const nCells = splitRow(nLine);
    return nCells.length > 0 && nCells.every(c => /^:?-+:?$/.test(c));
  }

  for (let idx = 0; idx < lines.length; idx++) {
    const lineNum = idx + 1;
    const line = lines[idx];
    const trimmed = line.trim();

    if (trimmed.startsWith('#')) {
      if (inTable && tableLines.length > 0) {
        blocks.push({
          type: 'table',
          startLine: tableStartLine,
          endLine: lineNum - 1,
          section: currentSection,
          header: currentHeader,
          lines: tableLines
        });
        inTable = false;
        tableLines = [];
        currentHeader = null;
        inHistoryTable = false;
        historySchema = null;
      }
      currentSection = trimmed.replace(/^#+\s*/, '').trim();
      blocks.push({
        type: 'heading',
        line: lineNum,
        raw: line,
        section: currentSection
      });
      continue;
    }

    if (!trimmed.startsWith('|')) {
      if (inTable && tableLines.length > 0) {
        blocks.push({
          type: 'table',
          startLine: tableStartLine,
          endLine: lineNum - 1,
          section: currentSection,
          header: currentHeader,
          lines: tableLines
        });
        inTable = false;
        tableLines = [];
        currentHeader = null;
        inHistoryTable = false;
        historySchema = null;
      }
      if (/BUG-/i.test(line)) {
        const match = line.match(/(BUG-[A-Za-z0-9-]+)/i);
        rows.push({
          key: `${source}:${lineNum}`,
          source,
          line: lineNum,
          raw: line,
          cells: [],
          tableHeader: null,
          section: currentSection,
          kind: 'reference',
          candidateId: match ? match[1] : null
        });
      }
      continue;
    }

    // It is a table line
    if (!inTable) {
      inTable = true;
      tableStartLine = lineNum;
      tableLines = [];
    }

    // Check if this row is a header followed by separator
    if (isNextLineSeparator(idx)) {
      tableLines.push({ lineNum, raw: line });
      const headerCells = splitRow(line);
      const lowerCells = headerCells.map(c => c.toLowerCase());
      idx++; // skip separator
      const sepLineNum = idx + 1;
      tableLines.push({ lineNum: sepLineNum, raw: lines[idx] });

      currentHeader = headerCells;
      if (lowerCells.length >= 10 && lowerCells[0] === 'id' && lowerCells[1] === 'type' && lowerCells[2] === 'severity') {
        inHistoryTable = false;
        historySchema = null;
      } else if (lowerCells.length === 3 && (lowerCells[0].includes('old') || lowerCells[1].includes('new'))) {
        inHistoryTable = true;
        historySchema = 'old-new-reason';
      } else if (lowerCells.length === 4 && (lowerCells[0].includes('old') || lowerCells[1].includes('new'))) {
        inHistoryTable = true;
        historySchema = 'old-new-status-record';
      } else if (lowerCells.length === 5 && (lowerCells[0].includes('old') || lowerCells[2].includes('reissued'))) {
        inHistoryTable = true;
        historySchema = 'old-kept-reissued';
      } else {
        inHistoryTable = false;
        historySchema = null;
      }
      continue;
    }

    tableLines.push({ lineNum, raw: line });
    const cells = splitRow(line);
    const firstCell = cells[0] || '';
    const isBugPrefixed = /^BUG-/i.test(firstCell);
    const match = firstCell.match(/^(BUG-[A-Za-z0-9-]+)/i);
    const candidateId = match ? match[1] : (isBugPrefixed ? firstCell : null);

    const looksLikeRecord = cells.length === 10 && VALID_TYPES.has(cells[1].toLowerCase()) && VALID_SEVERITIES.has(cells[2].toLowerCase());

    if (inHistoryTable && looksLikeRecord) {
      inHistoryTable = false;
      historySchema = null;
    }

    let kind = 'table-row';
    let historyMapping = null;

    if (inHistoryTable) {
      const looksLikeDamaged = cells.length >= 3 && VALID_TYPES.has(cells[1].toLowerCase()) && VALID_SEVERITIES.has(cells[2].toLowerCase());
      if (looksLikeDamaged) {
        kind = 'malformed';
        findings.push({
          kind: 'damagedHistoryRow',
          detail: `Damaged history row appears to be truncated record: ${line}`,
          source,
          line: lineNum
        });
      } else if (historySchema === 'old-new-reason' && cells.length === 3) {
        kind = 'history';
        historyMapping = { oldId: cells[0], newId: cells[1], reason: cells[2] };
      } else if (historySchema === 'old-new-status-record' && cells.length === 4) {
        kind = 'history';
        historyMapping = { oldId: cells[0], newId: cells[1], status: cells[2], record: cells[3] };
      } else if (historySchema === 'old-kept-reissued' && cells.length === 5) {
        kind = 'history';
        historyMapping = { oldId: cells[0], keptBy: cells[1], reissuedAs: cells[2], title: cells[3], citationsRepointed: cells[4] };
      } else if (isBugPrefixed) {
        kind = 'malformed';
        findings.push({
          kind: 'malformedHistoryRow',
          detail: `Row does not match history schema (${historySchema}): ${line}`,
          source,
          line: lineNum
        });
      }
    } else {
      if (looksLikeRecord) {
        kind = 'record';
      } else if (isBugPrefixed || (currentHeader && currentHeader.length === 10)) {
        kind = 'malformed';
        findings.push({
          kind: 'malformedCandidate',
          detail: `Malformed candidate row (${cells.length} cols): ${firstCell}`,
          source,
          line: lineNum,
          cellsLength: cells.length
        });
      }
    }

    if (currentSection.toLowerCase().includes('history') && (kind === 'record' || kind === 'malformed')) {
      findings.push({
        kind: 'recordUnderHistory',
        detail: `Record or malformed row found under history section: ${firstCell}`,
        source,
        line: lineNum
      });
    }

    rows.push({
      key: `${source}:${lineNum}`,
      source,
      line: lineNum,
      raw: line,
      cells,
      tableHeader: currentHeader,
      section: currentSection,
      kind,
      candidateId: candidateId || firstCell,
      historyMapping
    });
  }

  if (inTable && tableLines.length > 0) {
    blocks.push({
      type: 'table',
      startLine: tableStartLine,
      endLine: lines.length,
      section: currentSection,
      header: currentHeader,
      lines: tableLines
    });
  }

  return { rows, blocks, findings };
}

function inventoryRecovery(root, options = {}) {
  const config = readConfig(root);
  if (config.mode !== 'collection') {
    throw new Error(`Recovery requires collection mode, got: ${config.mode}`);
  }

  const legacyPath = options.legacy ? path.resolve(options.legacy) : null;
  if (!legacyPath || !fs.existsSync(legacyPath)) {
    throw new Error(`Legacy source file missing: ${options.legacy}`);
  }

  const outDir = options.out ? path.resolve(options.out) : path.join(path.resolve(root), '.cursor/audits/ai-config/2026-10-10-bug-memory-recovery');
  if (!fs.existsSync(outDir)) {
    fs.mkdirSync(outDir, { recursive: true });
  }

  const legacyBytes = fs.readFileSync(legacyPath);
  const legacySha = sha256(legacyBytes);
  const relLegacy = normalizePath(path.relative(path.resolve(root), legacyPath));

  const legacyScan = scanRecoverySource(legacyBytes.toString('utf8'), relLegacy);

  const byKind = {};
  for (const r of legacyScan.rows) {
    byKind[r.kind] = (byKind[r.kind] || 0) + 1;
  }

  // Scan current live collection using readProjectLedger
  const projectLedger = readProjectLedger(root);
  const liveFiles = [];
  const liveOccurrences = [];

  const seenPaths = new Set();
  for (const f of projectLedger.files) {
    const full = resolveInside(root, f);
    if (fs.existsSync(full)) {
      seenPaths.add(f);
      const b = fs.readFileSync(full);
      liveFiles.push({ path: f, sha256: sha256(b), bytes: b.length });
    }
  }

  const bugsRoot = resolveInside(root, '.cursor/memory/bugs');
  function walkBugsDir(dir) {
    if (!fs.existsSync(dir)) return;
    for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, ent.name);
      if (ent.isDirectory()) {
        walkBugsDir(full);
      } else {
        const rel = normalizePath(path.relative(root, full));
        if (!seenPaths.has(rel)) {
          seenPaths.add(rel);
          const b = fs.readFileSync(full);
          liveFiles.push({ path: rel, sha256: sha256(b), bytes: b.length });
        }
      }
    }
  }
  walkBugsDir(bugsRoot);

  for (const occ of projectLedger.occurrences) {
    const occFile = occ.source || occ.file || config.ledger;
    liveOccurrences.push({
      key: `${occFile}:${occ.line}`,
      id: occ.id,
      file: occFile,
      cells: occ.cells,
      status: occ.status,
      kind: occ.kind
    });
  }

  // Include history files if any
  const historyReadmeRel = '.cursor/memory/bugs/history/README.md';
  const historyReadmeFull = resolveInside(root, historyReadmeRel);
  if (fs.existsSync(historyReadmeFull)) {
    const b = fs.readFileSync(historyReadmeFull);
    liveFiles.push({ path: historyReadmeRel, sha256: sha256(b), bytes: b.length });
  }

  const configRel = '.claude/rules/project-values.md';
  const configFull = resolveInside(root, configRel);
  let configSha256 = null;
  if (fs.existsSync(configFull)) {
    configSha256 = sha256(fs.readFileSync(configFull));
  }

  const inventory = {
    contractVersion: 1,
    configSha256,
    schemaVersion: 2,
    projectRoot: normalizePath(path.resolve(root)),
    legacySource: {
      path: relLegacy,
      sha256: legacySha,
      bytes: legacyBytes.length
    },
    legacyCensus: {
      totalRows: legacyScan.rows.length,
      byKind,
      blocksCount: legacyScan.blocks.length,
      findingsCount: legacyScan.findings.length
    },
    legacyRows: legacyScan.rows,
    legacyBlocks: legacyScan.blocks,
    legacyFindings: legacyScan.findings,
    liveFiles,
    liveOccurrences
  };

  const invBytes = Buffer.from(JSON.stringify(inventory, null, 2) + '\n', 'utf8');
  atomicWrite(path.join(outDir, 'inventory.json'), invBytes);

  const censusBytes = Buffer.from(JSON.stringify({
    schemaVersion: 2,
    legacySource: inventory.legacySource,
    census: inventory.legacyCensus
  }, null, 2) + '\n', 'utf8');
  atomicWrite(path.join(outDir, 'source-census.json'), censusBytes);

  const liveBytes = Buffer.from(JSON.stringify({
    schemaVersion: 2,
    liveFiles,
    liveOccurrencesCount: liveOccurrences.length,
    liveOccurrences
  }, null, 2) + '\n', 'utf8');
  atomicWrite(path.join(outDir, 'live-inventory.json'), liveBytes);

  return inventory;
}

function validateReconciliation(inventory, reconciliation) {
  const errors = [];
  const warnings = [];

  if (!reconciliation || reconciliation.schemaVersion !== 2) {
    errors.push('Reconciliation must have schemaVersion: 2');
    return { valid: false, errors, warnings };
  }

  if (!Array.isArray(reconciliation.decisions)) {
    errors.push('Reconciliation decisions must be an array');
    return { valid: false, errors, warnings };
  }

  if (reconciliation.inventorySha256 && inventory) {
    const expectedSha = inventory.inventorySha256;
    if (expectedSha && reconciliation.inventorySha256 !== expectedSha) {
      errors.push(`Inventory file hash mismatch: expected ${expectedSha}, got ${reconciliation.inventorySha256}`);
    } else if (!expectedSha && typeof reconciliation.inventorySha256 === 'string' && reconciliation.inventorySha256.length !== 64) {
      errors.push('Invalid inventorySha256 format');
    }
  }

  const decisionMap = new Map();
  for (const d of reconciliation.decisions) {
    if (!d.key) {
      errors.push('Decision missing key');
      continue;
    }
    if (decisionMap.has(d.key)) {
      errors.push(`Duplicate decision key: ${d.key}`);
    }
    decisionMap.set(d.key, d);
  }

  // Ensure every legacy source row has an exact decision
  if (Array.isArray(inventory.legacyRows)) {
    for (const row of inventory.legacyRows) {
      if (!decisionMap.has(row.key)) {
        errors.push(`Missing reconciliation decision for legacy source row: ${row.key}`);
      }
    }
  }

  // Ensure every live occurrence has an exact decision
  if (Array.isArray(inventory.liveOccurrences)) {
    for (const occ of inventory.liveOccurrences) {
      if (!decisionMap.has(occ.key)) {
        errors.push(`Missing reconciliation decision for live occurrence: ${occ.key}`);
      }
    }
  }

  const allowedClassifications = new Set(['record', 'repeated', 'malformed', 'history', 'reference', 'derived']);
  const outputIds = new Map();

  for (const d of reconciliation.decisions) {
    if (!allowedClassifications.has(d.classification)) {
      errors.push(`Invalid classification '${d.classification}' for key ${d.key}`);
      continue;
    }

    if (d.classification === 'record') {
      if (!d.outputId || !/^BUG-/i.test(d.outputId)) {
        errors.push(`Invalid outputId '${d.outputId}' for record ${d.key}`);
      }
      if (!d.outputFile || !d.outputFile.endsWith('.md')) {
        errors.push(`Invalid outputFile '${d.outputFile}' for record ${d.key}`);
      }
      if (!Array.isArray(d.outputCells) || d.outputCells.length !== 10) {
        errors.push(`Record ${d.key} must have exactly 10 outputCells`);
      } else {
        // Source evidence retention
        const srcRow = (inventory.legacyRows || []).find(r => r.key === d.key) || (inventory.liveOccurrences || []).find(o => o.key === d.key);
        if (srcRow && Array.isArray(srcRow.cells) && srcRow.cells[6]) {
          const srcEv = srcRow.cells[6].trim();
          const outEv = (d.outputCells[6] || '').trim();
          if (srcEv.length > 0 && !outEv.toLowerCase().includes(srcEv.toLowerCase())) {
            const err = new Error(`Output evidence for ${d.outputId || d.key} discarded source evidence`);
            err.kind = 'evidencePreservation';
            errors.push(err);
          }
        }
        const type = (d.outputCells[1] || '').toLowerCase();
        const sev = (d.outputCells[2] || '').toLowerCase();
        if (!VALID_TYPES.has(type)) {
          errors.push(`Record ${d.key} invalid type: ${d.outputCells[1]}`);
        }
        if (!VALID_SEVERITIES.has(sev)) {
          errors.push(`Record ${d.key} invalid severity: ${d.outputCells[2]}`);
        }
      }

      if (d.outputId) {
        if (outputIds.has(d.outputId)) {
          errors.push(`Collision on outputId '${d.outputId}': claimed by both ${outputIds.get(d.outputId)} and ${d.key}`);
        } else {
          outputIds.set(d.outputId, d.key);
        }
      }
    } else if (d.classification === 'malformed') {
      if (!Array.isArray(d.outputCells) || d.outputCells.length !== 10) {
        errors.push(`Malformed row ${d.key} must have a reviewed 10-cell repair in outputCells`);
      }
    } else if (d.classification === 'repeated') {
      if (!d.canonicalKey) {
        errors.push(`Repeated decision ${d.key} must specify canonicalKey`);
      } else if (!decisionMap.has(d.canonicalKey)) {
        errors.push(`Repeated decision ${d.key} references nonexistent canonicalKey: ${d.canonicalKey}`);
      } else if (d.canonicalKey === d.key) {
        errors.push(`Self-referential canonicalKey on ${d.key}`);
      } else {
        // Cycle check
        const visited = new Set([d.key]);
        let cur = d.canonicalKey;
        while (cur) {
          if (visited.has(cur)) {
            errors.push(`Canonical key cycle detected for ${d.key}: contains ${cur}`);
            break;
          }
          visited.add(cur);
          const nextD = decisionMap.get(cur);
          if (!nextD) break;
          if (nextD.classification === 'record') break;
          cur = nextD.canonicalKey;
        }
      }
    }
  }

  if (Array.isArray(reconciliation.unresolved) && reconciliation.unresolved.length > 0) {
    errors.push(`Unresolved decisions present (${reconciliation.unresolved.length}): must be resolved before prepare`);
  }

  return { valid: errors.length === 0, errors, warnings };
}

function prepareRecovery(root, options = {}) {
  const outDir = options.out ? path.resolve(options.out) : path.join(path.resolve(root), '.cursor/audits/ai-config/2026-10-10-bug-memory-recovery');
  const invPath = path.join(outDir, 'inventory.json');
  if (!fs.existsSync(invPath)) {
    throw new Error('Inventory file missing: run inventory first');
  }
  const inventory = JSON.parse(fs.readFileSync(invPath, 'utf8'));

  const mappingPath = options.mapping ? path.resolve(options.mapping) : path.join(outDir, 'reconciliation.json');
  if (!fs.existsSync(mappingPath)) {
    throw new Error(`Mapping file missing: ${mappingPath}`);
  }
  const mappingBytes = fs.readFileSync(mappingPath);
  const mapping = JSON.parse(mappingBytes.toString('utf8'));

  // Reject stale live baseline if modified since inventory
  if (Array.isArray(inventory.liveFiles)) {
    for (const lf of inventory.liveFiles) {
      const full = resolveInside(root, lf.path);
      if (!fs.existsSync(full)) {
        throw new Error(`Live file missing since inventory: ${lf.path}`);
      }
      const currentHash = sha256(fs.readFileSync(full));
      if (lf.sha256 && currentHash !== lf.sha256) {
        throw new Error(`Live file modified since inventory: ${lf.path} (stale inventory hash)`);
      }
    }
  }

  const val = validateReconciliation(inventory, mapping);
  if (!val.valid) {
    throw new Error('Reconciliation validation failed:\n' + val.errors.join('\n'));
  }

  const stagedDir = path.join(outDir, 'staged');
  if (fs.existsSync(stagedDir)) {
    fs.rmSync(stagedDir, { recursive: true, force: true });
  }
  fs.mkdirSync(stagedDir, { recursive: true });

  // Copy config/hub/exceptions overlay into stagedDir for staged validation without adding to writes
  const overlayFiles = [
    '.claude/rules/project-values.md',
    '.cursor/memory/bugs/README.md',
    '.cursor/memory/bugs/validation-exceptions.json'
  ];
  for (const rel of overlayFiles) {
    const liveFull = resolveInside(root, rel);
    if (fs.existsSync(liveFull)) {
      const stagedTarget = resolveInside(stagedDir, rel);
      fs.mkdirSync(path.dirname(stagedTarget), { recursive: true });
      fs.copyFileSync(liveFull, stagedTarget);
    }
  }

  // Map decisions by outputFile
  const recordsByFile = new Map();
  const historyBlocks = mapping.historyBlocks || [];

  for (const d of mapping.decisions) {
    if (d.classification === 'record' || (d.classification === 'malformed' && d.outputCells && d.outputCells.length === 10)) {
      const file = d.outputFile;
      if (!recordsByFile.has(file)) recordsByFile.set(file, []);
      recordsByFile.get(file).push(d);
    }
  }

  // Live occurrences map to check existing live records
  const liveMap = new Map();
  for (const occ of inventory.liveOccurrences) {
    liveMap.set(occ.id, occ);
  }

  const writes = [];
  const domainHeader = '| ID | Type | Severity | Status | Title | Source | Evidence | Found | Updated | Resolution |\n|---|---|---|---|---|---|---|---|---|---|';

  // For each domain file that receives recovered records or exists in live
  const allDomainFiles = new Set([...inventory.liveFiles.map(f => f.path).filter(p => p.startsWith('.cursor/memory/bugs/mem-bugs-')), ...recordsByFile.keys()]);

  for (const relFile of allDomainFiles) {
    const stagedFile = resolveInside(stagedDir, relFile);
    fs.mkdirSync(path.dirname(stagedFile), { recursive: true });

    const liveFull = resolveInside(root, relFile);
    let beforeHash = null;
    let existingText = '';

    if (fs.existsSync(liveFull)) {
      const liveBytes = fs.readFileSync(liveFull);
      beforeHash = sha256(liveBytes);
      existingText = liveBytes.toString('utf8');
    }

    // Parse existing records in this file if any
    const existingScan = existingText ? scanText(existingText, relFile) : { occurrences: [] };
    const existingIds = new Set(existingScan.occurrences.filter(o => o.kind === 'record').map(o => o.id));

    // Combine existing records and newly added records
    const fileDecisions = recordsByFile.get(relFile) || [];
    const decisionById = new Map();
    for (const d of fileDecisions) {
      decisionById.set(d.outputId, d);
    }

    let newContent = existingText;
    if (!newContent) {
      const titleName = path.basename(relFile, '.md').replace(/^mem-bugs-/, '');
      const capitalized = titleName.charAt(0).toUpperCase() + titleName.slice(1);
      newContent = `# Memory: ${capitalized} Bugs\n\n${domainHeader}\n`;
    } else {
      // If file has existing records, check if any existing record's cells changed in decisions
      const lines = newContent.split(/\r?\n/);
      let modified = false;
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (line.trim().startsWith('|') && line.includes('|')) {
          const cells = splitRow(line);
          if (cells.length === 10 && cells[0].startsWith('BUG-')) {
            const id = cells[0];
            const d = decisionById.get(id);
            if (d && d.outputCells && JSON.stringify(d.outputCells) !== JSON.stringify(cells)) {
              lines[i] = '| ' + d.outputCells.join(' | ') + ' |';
              modified = true;
            }
          }
        }
      }
      if (modified) {
        newContent = lines.join('\n');
      }
    }

    // Append new records that did not exist in existingIds
    const newRecords = fileDecisions.filter(d => !existingIds.has(d.outputId));
    if (newRecords.length > 0) {
      const rowsToAdd = newRecords.map(d => '| ' + d.outputCells.join(' | ') + ' |').join('\n');
      if (newContent.trim().endsWith('|')) {
        newContent = newContent.trimEnd() + '\n' + rowsToAdd + '\n';
      } else {
        newContent = newContent + '\n' + rowsToAdd + '\n';
      }
    }

    const stagedBytes = Buffer.from(newContent, 'utf8');
    fs.writeFileSync(stagedFile, stagedBytes);
    const afterHash = sha256(stagedBytes);

    writes.push({
      path: relFile,
      beforeHash,
      afterHash,
      stagedPath: stagedFile
    });
  }

  // History file replacement if history blocks present
  const historyFileRel = '.cursor/memory/bugs/history/README.md';
  const liveHistoryFull = resolveInside(root, historyFileRel);
  if (fs.existsSync(liveHistoryFull)) {
    const liveHistoryBytes = fs.readFileSync(liveHistoryFull);
    const stagedHistoryFile = resolveInside(stagedDir, historyFileRel);
    fs.mkdirSync(path.dirname(stagedHistoryFile), { recursive: true });

    let historyContent = liveHistoryBytes.toString('utf8');
    if (historyBlocks.length > 0) {
      const blocksText = historyBlocks.map(b => b.raw || b.content || '').join('\n\n');
      historyContent = '# Bug Identity History\n\n' + blocksText + '\n';
    }

    const stagedHistoryBytes = Buffer.from(historyContent, 'utf8');
    fs.writeFileSync(stagedHistoryFile, stagedHistoryBytes);

    writes.push({
      path: historyFileRel,
      beforeHash: sha256(liveHistoryBytes),
      afterHash: sha256(stagedHistoryBytes),
      stagedPath: stagedHistoryFile
    });
  }

  const manifest = {
    schemaVersion: 2,
    createdAt: new Date().toISOString(),
    projectRoot: normalizePath(path.resolve(root)),
    inventorySha256: sha256(fs.readFileSync(invPath)),
    mappingSha256: sha256(mappingBytes),
    sources: [
      { path: inventory.legacySource.path, sha256: inventory.legacySource.sha256, role: 'legacy' },
      { path: normalizePath(path.relative(path.resolve(root), invPath)), sha256: sha256(fs.readFileSync(invPath)), role: 'inventory' },
      { path: normalizePath(path.relative(path.resolve(root), mappingPath)), sha256: sha256(mappingBytes), role: 'mapping' }
    ],
    writes
  };

  const additionalGuards = [
    { path: '.cursor/memory/bugs/README.md', role: 'hub' },
    { path: '.claude/rules/project-values.md', role: 'config' },
    { path: '.cursor/memory/bugs/validation-exceptions.json', role: 'exceptions' }
  ];
  for (const g of additionalGuards) {
    const full = resolveInside(root, g.path);
    if (fs.existsSync(full)) {
      if (!manifest.sources.some(s => s.path === g.path)) {
        manifest.sources.push({
          path: g.path,
          sha256: sha256(fs.readFileSync(full)),
          role: g.role
        });
      }
    }
  }

  if (Array.isArray(inventory.liveFiles)) {
    for (const lf of inventory.liveFiles) {
      if (!manifest.sources.some(s => s.path === lf.path)) {
        manifest.sources.push({
          path: lf.path,
          sha256: lf.sha256,
          role: 'liveFile'
        });
      }
    }
  }

  const manifestBytes = Buffer.from(JSON.stringify(manifest, null, 2) + '\n', 'utf8');
  atomicWrite(path.join(outDir, 'prepared-manifest.json'), manifestBytes);

  return manifest;
}

function verifyRecovery(root, options = {}) {
  const outDir = options.out ? path.resolve(options.out) : path.join(path.resolve(root), '.cursor/audits/ai-config/2026-10-10-bug-memory-recovery');
  const invPath = path.join(outDir, 'inventory.json');
  if (!fs.existsSync(invPath)) {
    throw new Error('Inventory file missing: run inventory first');
  }
  const inventory = JSON.parse(fs.readFileSync(invPath, 'utf8'));

  const isStaged = Boolean(options.staged);
  const verifyTarget = isStaged ? path.join(outDir, 'staged') : path.resolve(root);

  const errors = [];
  const warnings = [];

  // Run ledger validation with parsed snapshot
  let snapshot;
  let valResult;
  try {
    snapshot = readProjectLedger(verifyTarget);
    valResult = validateLedger(snapshot);
    if (valResult.errors && valResult.errors.length > 0) {
      errors.push(...valResult.errors.map(e => `[Ledger] ${e.kind}: ${e.detail} (${e.path || e.location})`));
    }
    if (valResult.warnings) {
      warnings.push(...valResult.warnings);
    }
  } catch (err) {
    errors.push('Ledger read/validation error: ' + err.message);
    snapshot = { records: [], mode: 'none' };
    valResult = { errors: [err.message], warnings: [] };
  }

  const ledgerValidation = {
    mode: snapshot ? snapshot.mode : 'none',
    recordCount: snapshot && snapshot.records ? snapshot.records.length : 0,
    valid: valResult ? valResult.errors.length === 0 : false,
    errors: valResult ? valResult.errors : [],
    warnings: valResult ? valResult.warnings : []
  };

  if (isStaged && ledgerValidation.recordCount === 0) {
    errors.push('Staged ledger validation reported 0 records: overlay or domain files missing in staged directory');
  }
  if (snapshot && snapshot.mode === 'collection' && ledgerValidation.recordCount === 0) {
    errors.push('Collection mode requires at least 1 record; 0 records inspected');
  }

  // Conservation validation against reconciliation decisions
  const mappingPath = options.mapping ? path.resolve(options.mapping) : path.join(outDir, 'reconciliation.json');
  let mapping = null;
  if (!fs.existsSync(mappingPath)) {
    errors.push(`Reconciliation mapping file missing: ${mappingPath}`);
  } else {
    try {
      mapping = JSON.parse(fs.readFileSync(mappingPath, 'utf8'));
    } catch (err) {
      errors.push(`Failed to read reconciliation mapping: ${err.message}`);
    }
  }

  if (mapping) {
    const invBytes = fs.readFileSync(invPath);
    const actualInvSha = sha256(invBytes);
    if (mapping.inventorySha256 && mapping.inventorySha256 !== actualInvSha) {
      errors.push(`Reconciliation inventorySha256 mismatch: mapping=${mapping.inventorySha256}, actual=${actualInvSha}`);
    }
  }

  // Delta handling
  const deltaEntriesById = new Map();
  if (options.delta) {
    const deltaPath = path.resolve(options.delta);
    if (!fs.existsSync(deltaPath)) {
      errors.push(`Delta file missing: ${deltaPath}`);
    } else {
      let delta;
      try {
        delta = JSON.parse(fs.readFileSync(deltaPath, 'utf8'));
      } catch (err) {
        errors.push(`Delta JSON parse error: ${err.message}`);
      }

      if (delta) {
        if (delta.schemaVersion !== 2) {
          errors.push('Delta schemaVersion must be 2');
        }

        if (!delta.predecessorProofSha256) {
          errors.push('Delta missing predecessorProofSha256');
        } else {
          const predProofPath = path.join(outDir, 'conservation-live.json');
          if (fs.existsSync(predProofPath)) {
            const predBytes = fs.readFileSync(predProofPath);
            const actualPredHash = sha256(predBytes);
            if (delta.predecessorProofSha256 !== actualPredHash) {
              errors.push(`Delta predecessorProofSha256 mismatch: delta=${delta.predecessorProofSha256}, actual=${actualPredHash}`);
            }
          }
        }

        const mappingRecordMap = new Map();
        if (mapping && Array.isArray(mapping.decisions)) {
          for (const d of mapping.decisions) {
            if (d.classification === 'record') {
              mappingRecordMap.set(d.outputId, d);
            }
          }
        }

        const allowedFields = new Set(['Status', 'Evidence', 'Updated', 'Resolution']);
        const FIELD_INDICES = {
          'Status': 3,
          'Evidence': 6,
          'Updated': 8,
          'Resolution': 9
        };

        for (const entry of delta.entries || []) {
          if (!entry.id) {
            errors.push('Delta entry missing id');
            continue;
          }
          deltaEntriesById.set(entry.id, entry);

          if (Array.isArray(entry.changedFields)) {
            for (const f of entry.changedFields) {
              if (!allowedFields.has(f)) {
                errors.push(`Disallowed field '${f}' in delta for ${entry.id}`);
              }
            }
          }

          if (Array.isArray(entry.allowedFields)) {
            for (const f of entry.allowedFields) {
              if (!allowedFields.has(f)) {
                errors.push(`Disallowed field '${f}' in delta for ${entry.id}`);
              }
            }
          }

          if (entry.afterCells || entry.beforeCells) {
            if (!Array.isArray(entry.beforeCells) || !Array.isArray(entry.afterCells) || entry.beforeCells.length !== 10 || entry.afterCells.length !== 10) {
              errors.push(`Delta entry ${entry.id} must have exactly 10 beforeCells and 10 afterCells`);
              continue;
            }

            // Validate beforeCells against approved predecessor record in mapping
            const approvedDecision = mappingRecordMap.get(entry.id);
            if (approvedDecision && approvedDecision.outputCells) {
              if (JSON.stringify(entry.beforeCells) !== JSON.stringify(approvedDecision.outputCells)) {
                errors.push(`Delta beforeCells for ${entry.id} does not match approved predecessor record in reconciliation mapping`);
              }
            }

            if (entry.beforeCellsSha256 && sha256(Buffer.from(JSON.stringify(entry.beforeCells))) !== entry.beforeCellsSha256) {
              errors.push(`Delta entry ${entry.id} beforeCellsSha256 mismatch`);
            }
            if (entry.afterCellsSha256 && sha256(Buffer.from(JSON.stringify(entry.afterCells))) !== entry.afterCellsSha256) {
              errors.push(`Delta entry ${entry.id} afterCellsSha256 mismatch`);
            }

            const IMMUTABLE_INDICES = [0, 1, 2, 4, 5, 7];
            for (const idx of IMMUTABLE_INDICES) {
              if (entry.beforeCells[idx] !== entry.afterCells[idx]) {
                errors.push(`Disallowed immutable field index ${idx} changed in delta for ${entry.id}: "${entry.beforeCells[idx]}" -> "${entry.afterCells[idx]}"`);
              }
            }

            let anyChange = false;
            for (const [fieldName, idx] of Object.entries(FIELD_INDICES)) {
              if (entry.beforeCells[idx] !== entry.afterCells[idx]) {
                anyChange = true;
              }
            }
            if (!anyChange) {
              warnings.push(`Delta entry ${entry.id} specifies no cell changes`);
            }
          }
        }
      }
    }
  }

  // Compare snapshot records against mapping decisions
  if (mapping && Array.isArray(mapping.decisions) && snapshot && snapshot.records) {
    const liveRecordById = new Map();
    for (const r of snapshot.records) {
      liveRecordById.set(r.id, r);
    }

    for (const d of mapping.decisions) {
      if (d.classification === 'record') {
        const liveRecord = liveRecordById.get(d.outputId);
        if (!liveRecord) {
          errors.push(`Conservation missing record: ${d.outputId} (${d.key}) was not found in ${d.outputFile}`);
          continue;
        }

        const deltaEntry = deltaEntriesById.get(d.outputId);
        const expectedCells = deltaEntry ? deltaEntry.afterCells : d.outputCells;

        if (expectedCells && JSON.stringify(liveRecord.cells) !== JSON.stringify(expectedCells)) {
          errors.push(`Conservation cell mismatch for ${d.outputId}: live cells do not match expected cells`);
        }
      }
    }
  }

  const result = {
    ok: errors.length === 0,
    target: isStaged ? 'staged' : 'live',
    errors,
    warnings,
    ledgerValidation
  };

  let reportOut;
  if (options.reportOut) {
    const p = path.resolve(options.reportOut);
    if (!fs.existsSync(p) && !p.endsWith('.json')) {
      fs.mkdirSync(p, { recursive: true });
      reportOut = path.join(p, isStaged ? 'conservation-staged.json' : 'conservation-live.json');
    } else if (fs.existsSync(p) && fs.statSync(p).isDirectory()) {
      reportOut = path.join(p, isStaged ? 'conservation-staged.json' : 'conservation-live.json');
    } else {
      reportOut = p;
    }
  } else {
    reportOut = path.join(outDir, isStaged ? 'conservation-staged.json' : 'conservation-live.json');
  }

  atomicWrite(reportOut, Buffer.from(JSON.stringify(result, null, 2) + '\n', 'utf8'));

  return result;
}

module.exports = {
  scanRecoverySource,
  inventoryRecovery,
  validateReconciliation,
  prepareRecovery,
  verifyRecovery
};
