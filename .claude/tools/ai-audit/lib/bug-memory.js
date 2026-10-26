'use strict';

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');

const VALID_STATUSES = {
  'CONFIRMED': ['st-confirmed'],
  'IN_PROGRESS': ['st-inprogress'],
  'VERIFY_PENDING': ['st-verify'],
  'RESOLVED': ['st-resolved'],
  'WONT_FIX': ['st-wontfix'],
  'DEFERRED': ['st-deferred'],
  'REJECTED': ['st-rejected'],
  'SUSPECTED': ['st-suspected'],
  'DUPLICATE': ['st-duplicate']
};

const VALID_TYPES = new Set(['bug', 'gap', 'planned', 'risk']);
const VALID_SEVERITIES = new Set(['low', 'medium', 'high', 'critical', 'unknown']);
const cellHash = cells => crypto.createHash('sha256').update(JSON.stringify(cells)).digest('hex');

function normalizePath(p) {
  return p.replace(/\\/g, '/');
}

function resolveInside(root, rel) {
  if (typeof rel !== 'string' || !rel || normalizePath(rel).split('/').includes('..')) {
    throw new Error('Path must be relative without traversal: ' + rel);
  }
  if (path.isAbsolute(rel) || rel.startsWith('/') || rel.startsWith('\\')) {
    throw new Error('Path must be relative: ' + rel);
  }
  const rootReal = fs.existsSync(root) ? fs.realpathSync(root) : path.resolve(root);
  const resolved = path.resolve(root, rel);
  const relative = path.relative(rootReal, resolved);
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    throw new Error('Path traverses outside root: ' + rel);
  }
  let cur = resolved;
  while (cur && cur !== path.dirname(cur)) {
    if (fs.existsSync(cur)) {
      const realCur = fs.realpathSync(cur);
      const relToRoot = path.relative(rootReal, realCur);
      if (relToRoot.startsWith('..') || path.isAbsolute(relToRoot)) {
        throw new Error('Path escapes root via symlink: ' + rel);
      }
      break;
    }
    cur = path.dirname(cur);
  }
  return normalizePath(resolved);
}

function splitRow(line) {
  const trimmed = line.trim();
  if (!trimmed.startsWith('|')) {
    return [];
  }
  const cells = [];
  let current = '';
  let inCode = false;
  let codeDelimiter = '';
  let i = 1; // Skip initial '|'

  while (i < trimmed.length) {
    const ch = trimmed[i];

    if (ch === '`') {
      let backtickCount = 1;
      while (i + backtickCount < trimmed.length && trimmed[i + backtickCount] === '`') {
        backtickCount++;
      }
      const delim = '`'.repeat(backtickCount);
      if (!inCode) {
        inCode = true;
        codeDelimiter = delim;
      } else if (codeDelimiter === delim) {
        inCode = false;
        codeDelimiter = '';
      }
      current += delim;
      i += backtickCount;
      continue;
    }

    if (!inCode && ch === '|') {
      let backslashCount = 0;
      let k = current.length - 1;
      while (k >= 0 && current[k] === '\\') {
        backslashCount++;
        k--;
      }
      if (backslashCount % 2 === 1) {
        current += '|';
        i++;
        continue;
      } else {
        cells.push(current.trim());
        current = '';
        i++;
        continue;
      }
    }

    current += ch;
    i++;
  }

  if (current.length > 0) {
    const lastCell = current.trim();
    if (lastCell.length > 0) {
      cells.push(lastCell);
    }
  }

  return cells;
}

function parseStatusSpan(cell) {
  const match = cell.match(/<span\s+class=["']([^"']+)["']>([^<]+)<\/span>/i);
  if (match) {
    return {
      statusClass: match[1].trim(),
      statusText: match[2].trim().toUpperCase()
    };
  }
  return {
    statusClass: null,
    statusText: cell.trim().toUpperCase()
  };
}

function scanText(text, source) {
  const occurrences = [];
  const history = [];
  const findings = [];
  const lines = text.split(/\r?\n/);

  let currentSection = '';
  let inLedgerTable = false;
  let had10ColHeader = false;
  let inHistoryTable = false;

  for (let idx = 0; idx < lines.length; idx++) {
    const lineNum = idx + 1;
    const line = lines[idx];
    const trimmed = line.trim();

    if (trimmed.startsWith('#')) {
      currentSection = trimmed.replace(/^#+\s*/, '').trim();
      inLedgerTable = currentSection.toLowerCase() === 'ledger';
      had10ColHeader = false;
      inHistoryTable = /renumber|mapping/i.test(currentSection);
      continue;
    }

    if (!trimmed.startsWith('|')) {
      inLedgerTable = false;
      inHistoryTable = false;
      continue;
    }

    if (trimmed.startsWith('|') && trimmed.includes('|')) {
      const cells = splitRow(line);
      const lowerCells = cells.map(c => c.toLowerCase());

      if (lowerCells.length >= 10 && lowerCells[0] === 'id' && lowerCells[1] === 'type' && lowerCells[2] === 'severity') {
        inLedgerTable = true;
        had10ColHeader = true;
        inHistoryTable = false;
        continue;
      }

      if (lowerCells.length >= 2 && (lowerCells[0].includes('old id') || lowerCells[0].includes('prior id') || lowerCells[0].includes('legacy id') || lowerCells[0] === 'old' || (lowerCells[0] === 'id' && lowerCells.length < 10))) {
        inHistoryTable = true;
        inLedgerTable = false;
        had10ColHeader = false;
        continue;
      }

      if (cells.every(c => /^:?-+:?$/.test(c))) {
        continue;
      }

      const firstCell = cells[0] || '';
      const isBugId = /^BUG-[A-Za-z0-9-]+$/i.test(firstCell);
      const isAnnotatedBugId = /^BUG-[A-Za-z0-9-]+(\s+\([^)]+\))?$/i.test(firstCell);

      const looksLikeRecord = cells.length >= 3 && VALID_TYPES.has(cells[1].toLowerCase()) && VALID_SEVERITIES.has(cells[2].toLowerCase());
      if (inHistoryTable && (looksLikeRecord || cells.length === 10)) {
        inHistoryTable = false;
        inLedgerTable = true;
        had10ColHeader = true;
      }
      if (inHistoryTable) {
        if (isAnnotatedBugId || isBugId) {
          const entry = {
            oldId: firstCell,
            line: lineNum,
            source,
            raw: line,
            cells
          };
          if (cells.length === 5) {
            entry.keptBy = cells[1] || '';
            entry.reissuedAs = cells[2] || '';
            entry.newId = cells[2] || '';
            entry.title = cells[3] || '';
            entry.citationsRepointed = cells[4] || '';
            entry.schema = 5;
          } else if (cells.length === 4) {
            entry.newId = cells[1] || '';
            entry.status = cells[2] || '';
            entry.record = cells[3] || '';
            entry.schema = 4;
          } else {
            entry.newId = cells[1] || '';
            entry.reason = cells[2] || '';
            entry.schema = cells.length;
          }
          history.push(entry);
          continue;
        }
      }

      if (inLedgerTable) {
        if (had10ColHeader) {
          if (cells.length === 10) {
            occurrences.push({
              key: `${source}:${lineNum}`,
              source,
              line: lineNum,
              section: currentSection,
              raw: line,
              cells,
              id: firstCell,
              kind: 'record'
            });
            if (currentSection.toLowerCase() !== 'ledger') {
              findings.push({
                kind: 'recordOutsideLedger',
                detail: `Valid record in 10-column table outside canonical Ledger section (${currentSection}): ${firstCell}`,
                path: source,
                line: lineNum
              });
            }
          } else {
            occurrences.push({
              key: `${source}:${lineNum}`,
              source,
              line: lineNum,
              section: currentSection,
              raw: line,
              cells,
              id: firstCell,
              kind: 'malformed'
            });
            findings.push({
              kind: 'malformedBugRow',
              detail: `Row under ledger table has ${cells.length} cells instead of 10: ${firstCell}`,
              path: source,
              line: lineNum
            });
          }
        } else {
          // Under ledger section without explicit 10-col header (legacy/minimal test fixtures)
          if (isBugId) {
            occurrences.push({
              key: `${source}:${lineNum}`,
              source,
              line: lineNum,
              section: currentSection,
              raw: line,
              cells,
              id: firstCell,
              kind: 'record'
            });
          }
        }
        continue;
      }

      if (isBugId) {
        if (cells.length === 10 && VALID_TYPES.has(cells[1].toLowerCase()) && VALID_SEVERITIES.has(cells[2].toLowerCase())) {
          occurrences.push({
            key: `${source}:${lineNum}`,
            source,
            line: lineNum,
            section: currentSection,
            raw: line,
            cells,
            id: firstCell,
            kind: 'record'
          });
          findings.push({
            kind: 'recordOutsideLedger',
            detail: `Valid record candidate outside recognized ledger table in section "${currentSection}": ${firstCell}`,
            path: source,
            line: lineNum
          });
        } else if (currentSection === '') {
          // File without sections (raw fixture)
          occurrences.push({
            key: `${source}:${lineNum}`,
            source,
            line: lineNum,
            section: currentSection,
            raw: line,
            cells,
            id: firstCell,
            kind: 'record'
          });
        } else {
          occurrences.push({
            key: `${source}:${lineNum}`,
            source,
            line: lineNum,
            section: currentSection,
            raw: line,
            cells,
            id: firstCell,
            kind: 'malformed'
          });
          findings.push({
            kind: 'ambiguousBugRow',
            detail: `Ambiguous BUG row outside ledger in section "${currentSection}": ${line}`,
            path: source,
            line: lineNum
          });
        }
      }
    }
  }

  return { occurrences, history, findings };
}

function readConfig(root) {
  const findings = [];
  const valuesPath = path.join(root || '.', '.claude/rules/project-values.md');

  let mode = 'none';
  let ledger = null;
  let recordsDir = null;
  let auditRoot = '.cursor/audits';
  let idPrefix = 'BUG-';

  if (fs.existsSync(valuesPath)) {
    const content = fs.readFileSync(valuesPath, 'utf8');
    const sectionMatch = content.match(/##\s*bug-lifecycle-tracking[\s\S]*?(?=\n##\s|$)/i);
    if (sectionMatch) {
      const sectionText = sectionMatch[0];
      const fenceMatch = sectionText.match(/```[a-zA-Z0-9_-]*\r?\n([\s\S]*?)\r?\n```/);
      if (fenceMatch) {
        const lines = fenceMatch[1].split(/\r?\n/);
        const seenKeys = new Set();
        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed || trimmed.startsWith('#')) continue;
          const eq = trimmed.indexOf('=');
          if (eq === -1) continue;
          const k = trimmed.slice(0, eq).trim();
          const v = trimmed.slice(eq + 1).trim();

          if (seenKeys.has(k)) {
            findings.push({
              kind: 'duplicateConfigKey',
              detail: `Duplicate config key: ${k}`,
              path: '.claude/rules/project-values.md'
            });
          }
          seenKeys.add(k);

          if (k === 'BUG_LEDGER') ledger = v;
          else if (k === 'BUG_RECORDS_DIR') recordsDir = v;
          else if (k === 'AUDIT_ROOT') auditRoot = v;
          else if (k === 'BUG_ID_PREFIX') idPrefix = v;
        }
      }
    }
  }

  if (ledger) {
    try {
      resolveInside(root || '.', ledger);
    } catch (err) {
      findings.push({ kind: 'invalidPathEscape', detail: err.message, path: ledger });
    }
    if (recordsDir) {
      try {
        resolveInside(root || '.', recordsDir);
      } catch (err) {
        findings.push({ kind: 'invalidPathEscape', detail: err.message, path: recordsDir });
      }
      mode = 'collection';
      const normLedger = normalizePath(ledger);
      const normDir = normalizePath(recordsDir).replace(/\/$/, '') + '/';
      if (!normLedger.startsWith(normDir)) {
        findings.push({
          kind: 'ledgerNotInRecordsDir',
          detail: `Configured BUG_LEDGER (${ledger}) must reside inside BUG_RECORDS_DIR (${recordsDir})`,
          path: ledger
        });
      }
    } else {
      mode = 'legacy';
    }

    const fullLedger = path.join(root || '.', ledger);
    if (!fs.existsSync(fullLedger)) {
      findings.push({
        kind: 'configuredLedgerMissing',
        detail: `Configured BUG_LEDGER does not exist: ${ledger}`,
        path: ledger
      });
    }
  } else {
    const collectionHub = '.cursor/memory/bugs/README.md';
    const legacyPath = '.cursor/memory/mem-known-bugs-index.md';
    if (fs.existsSync(path.join(root || '.', collectionHub))) {
      mode = 'collection';
      ledger = collectionHub;
      recordsDir = '.cursor/memory/bugs';
    } else if (fs.existsSync(path.join(root || '.', legacyPath))) {
      mode = 'legacy';
      ledger = legacyPath;
    } else {
      mode = 'none';
      ledger = null;
    }
  }

  return {
    mode,
    ledger: ledger ? normalizePath(ledger) : null,
    recordsDir: recordsDir ? normalizePath(recordsDir) : null,
    auditRoot: normalizePath(auditRoot),
    idPrefix,
    findings
  };
}

function readProjectLedger(root, options = {}) {
  const readText = (rel) => {
    const safe = resolveInside(root || '.', rel);
    return options.readText ? options.readText(rel) : fs.readFileSync(safe, 'utf8');
  };
  const config = readConfig(root);
  const findings = [...config.findings];
  const files = [];
  const records = [];
  const occurrences = [];
  const history = [];
  const baselineWarnings = [];
  if (config.ledger && !config.findings.length) {
    const exceptionsRel = normalizePath(path.join(path.dirname(config.ledger), 'validation-exceptions.json'));
    try {
      const full = resolveInside(root || '.', exceptionsRel);
      if (fs.existsSync(full)) {
        const data = JSON.parse(fs.readFileSync(full, 'utf8'));
        if (data.schemaVersion !== 1 || !Array.isArray(data.warnings) ||
            data.warnings.some(w => !w.id || !w.source || !w.kind || !/^[a-f0-9]{64}$/.test(w.cellsSha256 || '') || !/^[a-f0-9]{64}$/.test(w.sourceSha256 || '') || !w.occurrenceKey)) {
          throw new Error('Invalid content-bound validation exceptions');
        }
        baselineWarnings.push(...data.warnings);
      }
    } catch (err) {
      findings.push({kind:'invalidValidationExceptions', detail:err.message, path:exceptionsRel});
    }
  }
  if (config.findings.length) return {mode:config.mode,files,records,occurrences,history,findings,baselineWarnings};

  // Check readText override on legacy path if root is empty or has readText
  if (options.readText && config.mode === 'none') {
    const testText = readText('.cursor/memory/mem-known-bugs-index.md');
    if (testText != null) {
      config.mode = 'legacy';
      config.ledger = '.cursor/memory/mem-known-bugs-index.md';
    }
  }

  if (config.mode === 'none') {
    return { mode: 'none', files: [], records: [], occurrences: [], history: [], findings };
  }

  if (config.mode === 'legacy') {
    if (config.ledger) {
      files.push(config.ledger);
      try {
        const text = readText(config.ledger);
        if (text != null) {
          const scan = scanText(text, config.ledger);
          occurrences.push(...scan.occurrences);
          history.push(...scan.history);
          findings.push(...scan.findings);
          for (const occ of scan.occurrences) {
            if (occ.kind === 'record') {
              const r = recordFromOccurrence(occ);
              if (r) records.push(r);
            }
          }
        }
      } catch (err) {
        findings.push({ kind: 'readError', detail: err.message, path: config.ledger });
      }
    }
    return { mode: 'legacy', files, records, occurrences, history, findings, baselineWarnings };
  }

  if (config.mode === 'collection') {
    const legacyPath = '.cursor/memory/mem-known-bugs-index.md';
    if (fs.existsSync(path.join(root || '.', legacyPath))) {
      const legacyText = readText(legacyPath);
      const scanLegacy = scanText(legacyText, legacyPath);
      const hasLiveRecords = scanLegacy.occurrences.some(o => o.kind === 'record');
      if (hasLiveRecords) {
        findings.push({
          kind: 'conflictingLedgerAuthority',
          detail: 'Both collection and legacy authority contain records',
          path: legacyPath
        });
      }
    }

    if (!config.ledger || !fs.existsSync(path.join(root || '.', config.ledger))) {
      return { mode: 'collection', files, records, occurrences, history, findings };
    }

    files.push(config.ledger);
    const hubDir = path.dirname(config.ledger);
    const hubText = readText(config.ledger);
    const hubScan = scanText(hubText, config.ledger);
    occurrences.push(...hubScan.occurrences);
    history.push(...hubScan.history);
    findings.push(...hubScan.findings);

    const declaredFiles = new Set();
    const hubLines = hubText.split(/\r?\n/);
    let inHubTable = false;

    for (let i = 0; i < hubLines.length; i++) {
      const line = hubLines[i].trim();
      if (line.startsWith('|') && line.includes('|')) {
        const cells = splitRow(line);
        const lower = cells.map(c => c.toLowerCase());
        if (lower.length >= 3 && lower[0] === 'domain' && lower[1] === 'file' && lower[2] === 'scope') {
          inHubTable = true;
          continue;
        }
        if (cells.every(c => /^:?-+:?$/.test(c))) continue;

        if (inHubTable && cells.length >= 3) {
          const fileCell = cells[1];
          const linkMatch = fileCell.match(/^\[([^\]]+)\]\((mem-bugs-[A-Za-z0-9_-]+\.md)\)$/);
          if (linkMatch) {
            const relFile = linkMatch[2].trim();
            const fullTarget = normalizePath(path.join(hubDir, relFile));
            try {
              resolveInside(root || '.', fullTarget);
            } catch (err) {
              findings.push({kind:'invalidDomainLink',detail:err.message,path:config.ledger,line:i+1});
              continue;
            }

            if (declaredFiles.has(fullTarget)) {
              findings.push({
                kind: 'duplicateDomainLink',
                detail: `Duplicate domain link in hub: ${relFile}`,
                path: config.ledger,
                line: i + 1
              });
            }
            declaredFiles.add(fullTarget);

            if (!fs.existsSync(path.join(root || '.', fullTarget))) {
              findings.push({
                kind: 'missingDomainFile',
                detail: `Declared domain file missing: ${fullTarget}`,
                path: fullTarget,
                line: i + 1
              });
            } else {
              files.push(fullTarget);
            }
          } else {
            findings.push({kind:'invalidDomainLink',detail:'Domain file must be exactly one link to direct child mem-bugs-*.md',path:config.ledger,line:i+1});
          }
        }
      } else if (line.startsWith('#')) {
        inHubTable = false;
      }
    }

    const fullRecordsDir = resolveInside(root || '.', config.recordsDir);
    if (fs.existsSync(fullRecordsDir)) {
      const dirEntries = fs.readdirSync(fullRecordsDir, { withFileTypes: true });
      for (const entry of dirEntries) {
        if (!entry.isDirectory() && entry.name.startsWith('mem-bugs-') && entry.name.endsWith('.md')) {
          const relChild = normalizePath(path.join(config.recordsDir, entry.name));
          if (!declaredFiles.has(relChild)) {
            findings.push({
              kind: 'unregisteredBugFile',
              detail: `Direct child domain file not declared in hub: ${relChild}`,
              path: relChild
            });
          }
        }
      }

      const historyDir = path.join(fullRecordsDir, 'history');
      if (fs.existsSync(historyDir)) {
        const walkHistory = (dir) => {
          try { resolveInside(root || '.', normalizePath(path.relative(root || '.', dir))); }
          catch (err) {findings.push({kind:'invalidPathEscape',detail:err.message,path:normalizePath(path.relative(root || '.', dir))});return;}
          for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
            const fullPath = path.join(dir, ent.name);
            if (ent.isDirectory()) {
              walkHistory(fullPath);
            } else if (ent.name.endsWith('.md')) {
              const rel = normalizePath(path.relative(root || '.', fullPath));
              let histText;
              try { histText = readText(rel); }
              catch (err) {findings.push({kind:'invalidPathEscape',detail:err.message,path:rel});continue;}
              const histScan = scanText(histText, rel);
              history.push(...histScan.history);
              if (histScan.occurrences.some(o => o.kind === 'record')) {
                findings.push({
                  kind: 'recordInHistory',
                  detail: `Live record rows found in history subtree: ${rel}`,
                  path: rel
                });
              }
            }
          }
        };
        walkHistory(historyDir);
      }
    }

    for (const df of declaredFiles) {
      if (fs.existsSync(path.join(root || '.', df))) {
        try {
          const text = readText(df);
          const scan = scanText(text, df);
          occurrences.push(...scan.occurrences);
          history.push(...scan.history);
          findings.push(...scan.findings);
          for (const occ of scan.occurrences) {
            if (occ.kind === 'record') {
              const r = recordFromOccurrence(occ);
              if (r) records.push(r);
            }
          }
        } catch (err) {
          findings.push({ kind: 'readError', detail: err.message, path: df });
        }
      }
    }

    return { mode: 'collection', files, records, occurrences, history, findings, baselineWarnings };
  }

  return { mode: config.mode, files, records, occurrences, history, findings };
}

function recordFromOccurrence(occ) {
  const cells = occ.cells;
  if (!cells || cells.length < 2) return null;
  const parsedStatus = cells.length >= 4 ? parseStatusSpan(cells[3]) : { statusClass: null, statusText: 'CONFIRMED' };
  return {
    id: cells[0],
    type: cells[1] || '',
    severity: cells[2] || '',
    status: parsedStatus.statusText,
    statusClass: parsedStatus.statusClass,
    title: cells[4] || '',
    source: cells[5] || '',
    evidence: cells[6] || '',
    found: cells[7] || '',
    updated: cells[8] || '',
    resolution: cells[9] || '',
    file: occ.source,
    location: `${occ.source}:${occ.line}`,
    cells
  };
}

function countIds(records) {
  const map = new Map();
  for (const r of records) {
    map.set(r.id, (map.get(r.id) || 0) + 1);
  }
  return map;
}

function validateLedger(snapshot, options = {}) {
  const errors = [];
  const warnings = [];
  const records = snapshot.records || [];

  for (const f of snapshot.findings || []) {
    errors.push(f);
  }

  const idCounts = countIds(records);
  for (const [id, count] of idCounts) {
    if (count > 1) {
      errors.push({
        kind: 'dupBugId',
        detail: `${id} x${count}`,
        id
      });
    }
  }

  const allRecordsMap = new Map();
  for (const r of records) {
    allRecordsMap.set(r.id, r);
  }

  for (const r of records) {
    if (!r.cells || r.cells.length !== 10) {
      errors.push({kind:'malformedBugRow',detail:`Record ${r.id} must contain ten cells`,location:r.location});
      continue;
    }
    const add = (kind, detail) => {
      const inherited = (options.baselineWarnings || snapshot.baselineWarnings || []).some(w =>
        w.id === r.id && w.source === r.file && w.kind === kind && w.cellsSha256 === cellHash(r.cells));
      (inherited ? warnings : errors).push({kind,detail,id:r.id,location:r.location});
    };
    if (!/^BUG-(?:\d{4}|\d{6}-[a-f0-9]{4})$/.test(r.id)) add('invalidBugId',`Invalid bug ID: ${r.id}`);
    if (!VALID_TYPES.has(r.type)) add('unknownType',`Unknown type ${r.type} on ${r.id}`);
    if (!VALID_SEVERITIES.has(r.severity)) add('unknownSeverity',`Unknown severity ${r.severity} on ${r.id}`);
    for (const field of ['title','source','evidence','found','updated','resolution']) {
      if (!r[field].trim()) add('missingLifecycleField',`Missing ${field} on ${r.id}`);
    }
    if (!/^<span\s+class=["'][^"']+["']>[^<]+<\/span>$/i.test(r.cells[3])) add('missingStatusSpan',`Status must be one marker span on ${r.id}`);

    const expectedClasses = VALID_STATUSES[r.status];
    if (expectedClasses) {
      if (r.statusClass && !expectedClasses.includes(r.statusClass)) {
        add('statusClassMismatch',`Status class ${r.statusClass} does not match status ${r.status} (expected ${expectedClasses.join(' or ')}) on ${r.id}`);
      }
    } else if (r.status) {
      add('unknownStatus',`Unknown status ${r.status} on ${r.id}`);
    }

    const validDate = value => value === 'unknown' || (/^\d{4}-\d{2}-\d{2}$/.test(value) && !Number.isNaN(Date.parse(value)) && new Date(value).toISOString().slice(0,10) === value);
    if (!validDate(r.found) || !validDate(r.updated)) {
      const detail = `Invalid date cell on ${r.id} (found: "${r.found}", updated: "${r.updated}")`;
      add('invalidDateCell',detail);
    }
    if (['RESOLVED','REJECTED','WONT_FIX','DUPLICATE','DEFERRED'].includes(r.status) && /^(?:open|pending|unknown|—|-|n\/a)?$/i.test(r.resolution.trim())) {
      add('missingTerminalReason',`Status ${r.status} requires reason on ${r.id}`);
    }

    if (r.status === 'DUPLICATE') {
      const match = r.resolution.match(/Duplicate\s+(?:of\s+)?(?:canonical\s+)?(?:\*\*)?(BUG-[A-Za-z0-9-]+)(?:\*\*)?/i);
      if (match) {
        const targetId = match[1];
        if (!allRecordsMap.has(targetId)) {
          errors.push({
            kind: 'brokenDuplicateTarget',
            detail: `Duplicate record ${r.id} references nonexistent target ${targetId}`,
            location: r.location
          });
        }
      } else {
        errors.push({
          kind: 'brokenDuplicateTarget',
          detail: `Duplicate record ${r.id} resolution must specify "Duplicate of BUG-xxxx": "${r.resolution}"`,
          location: r.location
        });
      }
    }
  }

  return { errors, warnings, records };
}

module.exports = {
  splitRow,
  scanText,
  readConfig,
  readProjectLedger,
  validateLedger,
  countIds,
  resolveInside
};
