'use strict';

const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const bm = require('./bug-memory');
const bmt = require('./bug-memory-transaction');

const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');

function normalizePath(p) {
  return p.replace(/\\/g, '/');
}

function resolveInside(root, relPath) {
  const resolved = path.resolve(root, relPath);
  const rootResolved = path.resolve(root);
  if (!resolved.startsWith(rootResolved)) {
    throw new Error(`Path escape attempt: ${relPath}`);
  }
  return resolved;
}

/**
 * Capture frozen recovery inputs into inventory and census.
 */
function captureRecoveryInputs(root, options = {}) {
  const bmr = require('./bug-memory-recovery');
  return bmr.inventoryRecovery(root, options);
}

/**
 * Load recovery artifacts and validate input integrity.
 * Throws structured errors with kind, path, detail.
 */
function loadRecoveryContract(root, options = {}) {
  if (options.contractVersion !== undefined && options.contractVersion !== 1) {
    const err = new Error('Invalid contractVersion: must be 1');
    err.kind = 'invalidContractVersion';
    throw err;
  }
  if (options.schemaVersion !== undefined && options.contractVersion === undefined) {
    const err = new Error('Missing contractVersion: contractVersion 1 required');
    err.kind = 'invalidContractVersion';
    throw err;
  }
  const outDir = options.out ? path.resolve(options.out) : path.join(path.resolve(root), '.cursor/audits/ai-config/2026-10-10-bug-memory-recovery-acceptance');
  const invPath = path.join(outDir, 'inventory.json');

  if (!fs.existsSync(invPath)) {
    const err = new Error(`Inventory file missing: ${invPath}`);
    err.kind = 'requiredInputMissing';
    err.path = 'inventory.json';
    err.detail = 'Inventory artifact missing';
    throw err;
  }

  let inventory;
  const invBytes = fs.readFileSync(invPath);
  try {
    inventory = JSON.parse(invBytes.toString('utf8'));
  } catch (e) {
    const err = new Error(`Malformed inventory JSON: ${e.message}`);
    err.kind = 'invalidInputJson';
    err.path = 'inventory.json';
    err.detail = e.message;
    throw err;
  }

  const inventoryHash = sha256(invBytes);

  // Check legacy source
  if (inventory.legacySource && inventory.legacySource.path) {
    const legacyFull = resolveInside(root, inventory.legacySource.path);
    if (!fs.existsSync(legacyFull)) {
      const err = new Error(`Legacy source missing: ${inventory.legacySource.path}`);
      err.kind = 'requiredInputMissing';
      err.path = inventory.legacySource.path;
      err.detail = 'Legacy source file missing';
      throw err;
    }
    const legacyBytes = fs.readFileSync(legacyFull);
    const legacyHash = sha256(legacyBytes);
    if (inventory.legacySource.sha256 && legacyHash !== inventory.legacySource.sha256) {
      const err = new Error(`Legacy source hash mismatch: expected ${inventory.legacySource.sha256}, got ${legacyHash}`);
      err.kind = 'inputHashMismatch';
      err.path = inventory.legacySource.path;
      err.detail = 'Original source modified';
      throw err;
    }
  }

  // Check live files baseline freeze if prepare or verify
  if (Array.isArray(inventory.liveFiles)) {
    for (const lf of inventory.liveFiles) {
      const full = resolveInside(root, lf.path);
      if (!fs.existsSync(full)) {
        const err = new Error(`Live file missing since inventory: ${lf.path}`);
        err.kind = 'requiredInputMissing';
        err.path = lf.path;
        err.detail = 'Live file missing';
        throw err;
      }
      const currentBytes = fs.readFileSync(full);
      const currentHash = sha256(currentBytes);
      if (options.phase === 'prepare' && lf.sha256 && currentHash !== lf.sha256) {
        const err = new Error(`Live file modified since inventory: ${lf.path}`);
        err.kind = 'baselineChanged';
        err.path = lf.path;
        err.detail = 'Live file modified after inventory before prepare';
        throw err;
      }
    }
  }

  // Check config file drift
  const configRel = '.claude/rules/project-values.md';
  const configFull = resolveInside(root, configRel);
  if (fs.existsSync(configFull)) {
    const configBytes = fs.readFileSync(configFull);
    const configHash = sha256(configBytes);
    if (inventory.configSha256 && configHash !== inventory.configSha256) {
      const err = new Error(`Config file modified since inventory: ${configRel}`);
      err.kind = 'inputHashMismatch';
      err.path = configRel;
      err.detail = 'Project values modified';
      throw err;
    }
  }

  // Check reconciliation mapping
  const mappingPath = options.mapping ? path.resolve(options.mapping) : path.join(outDir, 'reconciliation.json');
  let reconciliation = null;
  let reconciliationHash = null;

  if (fs.existsSync(mappingPath)) {
    const recBytes = fs.readFileSync(mappingPath);
    reconciliationHash = sha256(recBytes);
    try {
      reconciliation = JSON.parse(recBytes.toString('utf8'));
    } catch (e) {
      const err = new Error(`Malformed reconciliation JSON: ${e.message}`);
      err.kind = 'invalidInputJson';
      err.path = 'reconciliation.json';
      err.detail = e.message;
      throw err;
    }

    if (reconciliation.inventorySha256 && reconciliation.inventorySha256 !== inventoryHash) {
      const err = new Error(`Reconciliation inventorySha256 mismatch: expected ${inventoryHash}, got ${reconciliation.inventorySha256}`);
      err.kind = 'inputHashMismatch';
      err.path = 'reconciliation.json';
      err.detail = 'inventorySha256 does not match inventory.json';
      throw err;
    }
  } else if (options.phase === 'prepare' || options.phase === 'staged' || options.phase === 'live') {
    const err = new Error(`Reconciliation file missing: ${mappingPath}`);
    err.kind = 'requiredInputMissing';
    err.path = 'reconciliation.json';
    err.detail = 'Required reviewed mapping is missing';
    throw err;
  }

  // Check prepared manifest if verify staged/live/apply
  const manifestPath = path.join(outDir, 'prepared-manifest.json');
  let manifest = null;
  let manifestHash = null;
  if (fs.existsSync(manifestPath)) {
    const manBytes = fs.readFileSync(manifestPath);
    manifestHash = sha256(manBytes);
    try {
      manifest = JSON.parse(manBytes.toString('utf8'));
    } catch (e) {
      const err = new Error(`Malformed manifest JSON: ${e.message}`);
      err.kind = 'invalidInputJson';
      err.path = 'prepared-manifest.json';
      err.detail = e.message;
      throw err;
    }

    // Enforce manifest bindings against actual artifacts
    if (manifest.mappingSha256 && reconciliationHash && manifest.mappingSha256 !== reconciliationHash) {
      const err = new Error(`Manifest mappingSha256 mismatch: expected ${manifest.mappingSha256}, got ${reconciliationHash}`);
      err.kind = 'inputHashMismatch';
      err.path = 'prepared-manifest.json';
      err.detail = 'Manifest mappingSha256 does not match reconciliation.json on disk';
      throw err;
    }
    if (manifest.inventorySha256 && inventoryHash && manifest.inventorySha256 !== inventoryHash) {
      const err = new Error(`Manifest inventorySha256 mismatch: expected ${manifest.inventorySha256}, got ${inventoryHash}`);
      err.kind = 'inputHashMismatch';
      err.path = 'prepared-manifest.json';
      err.detail = 'Manifest inventorySha256 does not match inventory.json on disk';
      throw err;
    }
  }

  return {
    schemaVersion: 2,
    contractVersion: 1,
    root: normalizePath(path.resolve(root)),
    outDir: normalizePath(outDir),
    inventory,
    inventoryHash,
    reconciliation,
    reconciliationHash,
    manifest,
    manifestHash
  };
}

/**
 * Validate recovery contract rules across all phases.
 * Returns { ok, phase, errors, warnings, dimensions } without process.exit.
 */
function validateRecoveryContract(context, options = {}) {
  const phase = options.phase || 'verify';
  const errors = [];
  const warnings = [];
  const dimensions = {
    inputs: { valid: true },
    coverage: { valid: true },
    records: { valid: true },
    preservation: { valid: true },
    citations: { valid: true },
    validation: { valid: true },
    transaction: { valid: true },
    closure: { valid: true }
  };

  const { inventory, manifest, root, outDir } = context;
  const reconciliation = context.reconciliation || context.mapping;

  // 1. Reconciliation validation
  if (reconciliation) {
    if (!Array.isArray(reconciliation.decisions) || reconciliation.decisions.length === 0) {
      errors.push({
        kind: 'reconciliationCoverage',
        path: 'reconciliation.json',
        detail: 'Reconciliation decisions array is empty or missing'
      });
      dimensions.coverage.valid = false;
    } else {
      const decisionMap = new Map();
      const outputIds = new Map();
      const allowedClass = new Set(['record', 'repeated', 'malformed', 'history', 'historical', 'reference', 'derived']);

      for (const d of reconciliation.decisions) {
        if (!d.key) {
          errors.push({ kind: 'reconciliationCoverage', path: 'reconciliation.json', detail: 'Decision missing key' });
          continue;
        }
        if (decisionMap.has(d.key)) {
          errors.push({ kind: 'reconciliationCoverage', path: 'reconciliation.json', detail: `Duplicate decision for key ${d.key}` });
        }
        decisionMap.set(d.key, d);

        if (!allowedClass.has(d.classification)) {
          errors.push({ kind: 'reconciliationCoverage', path: 'reconciliation.json', detail: `Invalid classification ${d.classification} on ${d.key}` });
        }

        if (d.classification === 'record') {
          const outId = d.outputId;
          if (!outId || !/^BUG-/i.test(outId)) {
            errors.push({ kind: 'reconciliationCoverage', path: 'reconciliation.json', detail: `Invalid outputId ${outId} on ${d.key}` });
          } else if (outputIds.has(outId)) {
            errors.push({ kind: 'reconciliationCoverage', path: 'reconciliation.json', detail: `Collision on outputId ${outId}: claimed by multiple decisions` });
          } else {
            outputIds.set(outId, d.key);
          }
          if (!Array.isArray(d.outputCells) || d.outputCells.length !== 10) {
            errors.push({ kind: 'reconciliationCoverage', path: 'reconciliation.json', detail: `Record ${d.key} must have exactly 10 outputCells` });
          }
        }
      }

      // Check coverage of all inventory keys
      if (inventory) {
        if (Array.isArray(inventory.legacyRows)) {
          for (const lr of inventory.legacyRows) {
            if (!decisionMap.has(lr.key)) {
              errors.push({ kind: 'reconciliationCoverage', path: 'reconciliation.json', detail: `Missing decision for legacy key ${lr.key}` });
              dimensions.coverage.valid = false;
            }
          }
        }
        if (Array.isArray(inventory.liveOccurrences)) {
          for (const lo of inventory.liveOccurrences) {
            if (!decisionMap.has(lo.key)) {
              errors.push({ kind: 'reconciliationCoverage', path: 'reconciliation.json', detail: `Missing decision for live occurrence key ${lo.key}` });
              dimensions.coverage.valid = false;
            }
          }
        }
        // Check for unknown decision keys
        const validSourceKeys = new Set([
          ...(inventory.legacyRows || []).map(r => r.key),
          ...(inventory.liveOccurrences || []).map(o => o.key)
        ]);
        for (const [k] of decisionMap) {
          if (!validSourceKeys.has(k) && !k.includes(':part') && !k.includes('history')) {
            errors.push({ kind: 'reconciliationCoverage', path: 'reconciliation.json', detail: `Unknown decision key not in inventory: ${k}` });
            dimensions.coverage.valid = false;
          }
        }
      }

      // Source evidence retention checks: outputCells[6] must retain mapped source evidence
      if (inventory) {
        const sourceMap = new Map();
        for (const lr of (inventory.legacyRows || [])) {
          if (Array.isArray(lr.cells) && lr.cells[6]) sourceMap.set(lr.key, lr.cells[6]);
        }
        for (const lo of (inventory.liveOccurrences || [])) {
          if (Array.isArray(lo.cells) && lo.cells[6]) sourceMap.set(lo.key, lo.cells[6]);
        }

        for (const d of reconciliation.decisions) {
          if (d.classification === 'record' && Array.isArray(d.outputCells)) {
            const srcEv = sourceMap.get(d.key);
            const outEv = d.outputCells[6] || '';
            if (srcEv && srcEv.trim().length > 0 && !outEv.toLowerCase().includes(srcEv.trim().toLowerCase())) {
              errors.push({
                kind: 'evidencePreservation',
                path: 'reconciliation.json',
                detail: `Output evidence for ${d.outputId || d.key} discarded source evidence from ${d.key}`
              });
              dimensions.preservation.valid = false;
            }
          }
        }
      }

      // Canonical graph checks
      for (const d of reconciliation.decisions) {
        if (d.classification === 'repeated') {
          if (!d.canonicalKey || !decisionMap.has(d.canonicalKey)) {
            errors.push({ kind: 'invalidCanonicalGraph', path: 'reconciliation.json', detail: `Dangling canonicalKey: ${d.canonicalKey} on ${d.key}` });
            dimensions.coverage.valid = false;
          } else if (d.canonicalKey === d.key) {
            errors.push({ kind: 'invalidCanonicalGraph', path: 'reconciliation.json', detail: `Self-referential canonicalKey on ${d.key}` });
            dimensions.coverage.valid = false;
          } else {
            // Cycle detection
            const visited = new Set([d.key]);
            let cur = d.canonicalKey;
            while (cur) {
              if (visited.has(cur)) {
                errors.push({ kind: 'invalidCanonicalGraph', path: 'reconciliation.json', detail: `Canonical key cycle detected for ${d.key}: contains ${cur}` });
                dimensions.coverage.valid = false;
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
    }

    // Semantic citation checks: occurrence-specific context verification
    if (Array.isArray(reconciliation.citationDecisions)) {
      for (const cd of reconciliation.citationDecisions) {
        const full = resolveInside(root, cd.path);
        if (fs.existsSync(full)) {
          const content = fs.readFileSync(full, 'utf8');
          if (cd.expectedId && cd.defectSubject) {
            const lines = content.split('\n');
            const subjectLineIdx = lines.findIndex(l => l.includes(cd.defectSubject));
            if (subjectLineIdx !== -1) {
              // Local context window (line and immediate adjacent lines)
              const startIdx = Math.max(0, subjectLineIdx - 1);
              const endIdx = Math.min(lines.length, subjectLineIdx + 2);
              const windowText = lines.slice(startIdx, endIdx).join(' ');
              if (!windowText.includes(cd.expectedId)) {
                errors.push({
                  kind: 'citationIdentityMismatch',
                  path: cd.path,
                  detail: `Citation for "${cd.defectSubject}" at context "${lines[subjectLineIdx].trim()}" does not reference expected ID ${cd.expectedId}`
                });
                dimensions.citations.valid = false;
              }
            } else {
              errors.push({
                kind: 'citationIdentityMismatch',
                path: cd.path,
                detail: `Defect subject "${cd.defectSubject}" not found in ${cd.path}`
              });
              dimensions.citations.valid = false;
            }
          }
        }
      }
    }
  }

  // 2. Staged or Live Conservation validation
  if (phase === 'staged' || phase === 'live') {
    const targetRoot = options.targetRoot ? path.resolve(options.targetRoot) : (phase === 'staged' ? path.join(outDir, 'staged') : root);

    // History preservation check
    const historyDirRel = '.cursor/memory/bugs/history';
    const historyDirFull = path.join(targetRoot, historyDirRel);
    if (!fs.existsSync(historyDirFull)) {
      errors.push({
        kind: 'historyPreservation',
        path: historyDirRel,
        detail: `History directory missing in ${phase} target`
      });
      dimensions.preservation.valid = false;
    } else {
      // Recursive membership and content preservation across all history files
      const origHistoryDir = path.join(root, historyDirRel);
      if (fs.existsSync(origHistoryDir)) {
        for (const ent of fs.readdirSync(origHistoryDir, { withFileTypes: true })) {
          if (ent.isFile()) {
            const origFilePath = path.join(origHistoryDir, ent.name);
            const targetFilePath = path.join(historyDirFull, ent.name);
            const relFromRoot = path.normalize(`${historyDirRel}/${ent.name}`).replace(/\\/g, '/');
            const isKnownSource = manifest && (
              (Array.isArray(manifest.sources) && manifest.sources.some(s => path.normalize(s.path).replace(/\\/g, '/') === relFromRoot)) ||
              (Array.isArray(manifest.writes) && manifest.writes.some(w => path.normalize(w.path).replace(/\\/g, '/') === relFromRoot))
            );
            if (manifest && !isKnownSource) {
              errors.push({
                kind: 'sourceMembershipChanged',
                path: relFromRoot,
                detail: `Unregistered history file added since prepare: ${ent.name}`
              });
              dimensions.preservation.valid = false;
              continue;
            }
            if (!fs.existsSync(targetFilePath)) {
              errors.push({
                kind: 'historyPreservation',
                path: `${historyDirRel}/${ent.name}`,
                detail: `History file ${ent.name} missing in ${phase} output`
              });
              dimensions.preservation.valid = false;
            } else {
              const origLines = fs.readFileSync(origFilePath, 'utf8').split('\n').map(l => l.trim()).filter(l => l.length > 0);
              const targetContent = fs.readFileSync(targetFilePath, 'utf8');
              for (const line of origLines) {
                if (!targetContent.includes(line)) {
                  errors.push({
                    kind: 'historyPreservation',
                    path: `${historyDirRel}/${ent.name}`,
                    detail: `Historical content line "${line.slice(0, 40)}..." was discarded in ${ent.name}`
                  });
                  dimensions.preservation.valid = false;
                  break;
                }
              }
            }
          }
        }
      }
    }

    // Ledger validation in targetRoot
    const targetLedger = bm.readProjectLedger(targetRoot);
    if (targetLedger.mode === 'none' || targetLedger.files.length === 0) {
      errors.push({
        kind: 'ledgerValidationFailed',
        path: targetRoot,
        detail: `Failed to read valid ledger collection in ${phase} root`
      });
      dimensions.validation.valid = false;
    } else {
      const ledgerVal = bm.validateLedger(targetLedger);
      if (!ledgerVal.valid && ledgerVal.errors.length > 0) {
        for (const le of ledgerVal.errors) {
          errors.push({ kind: 'ledgerValidationFailed', path: le.location || targetRoot, detail: le.detail || le.kind });
        }
        dimensions.validation.valid = false;
      }
      if (ledgerVal.warnings) {
        warnings.push(...ledgerVal.warnings);
      }

      const targetRecIds = new Set(targetLedger.records.map(r => r.id));

      // Record preservation: all current-only or live inventory records must survive
      if (inventory && Array.isArray(inventory.liveOccurrences)) {
        for (const occ of inventory.liveOccurrences) {
          if (!targetRecIds.has(occ.id)) {
            errors.push({
              kind: 'recordPreservation',
              path: occ.file,
              detail: `Live record ${occ.id} was dropped in ${phase} output`
            });
            dimensions.preservation.valid = false;
          }
        }
      }

      // Evidence preservation & Output coverage
      if (reconciliation && Array.isArray(reconciliation.decisions)) {
        for (const d of reconciliation.decisions) {
          if (d.classification === 'record') {
            const foundRec = targetLedger.records.find(r => r.id === d.outputId);
            if (!foundRec) {
              errors.push({
                kind: 'recordPreservation',
                path: d.domainFile || d.outputFile,
                detail: `Approved record ${d.outputId} is missing in ${phase} output`
              });
              dimensions.records.valid = false;
            } else {
              // Complete 10-cell preservation check
              if (Array.isArray(d.outputCells)) {
                const actualCells = Array.isArray(foundRec.cells) ? foundRec.cells : foundRec.outputCells;
                if (Array.isArray(actualCells)) {
                  for (let cIdx = 0; cIdx < 10; cIdx++) {
                    const expCell = d.outputCells[cIdx];
                    const actCell = actualCells[cIdx];
                    if (expCell !== undefined && actCell !== expCell) {
                      if (cIdx === 6) {
                        errors.push({
                          kind: 'evidencePreservation',
                          path: d.domainFile || d.outputFile,
                          detail: `Evidence for ${d.outputId} was modified or discarded in ${phase} output`
                        });
                        dimensions.preservation.valid = false;
                      } else {
                        errors.push({
                          kind: 'outputCellMismatch',
                          path: d.domainFile || d.outputFile,
                          detail: `Cell index ${cIdx} for ${d.outputId} in ${phase} output does not match approved cell`
                        });
                        dimensions.preservation.valid = false;
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }

      // Check for unapproved output records
      if (reconciliation && Array.isArray(reconciliation.decisions)) {
        const approvedIds = new Set(
          reconciliation.decisions.filter(d => d.classification === 'record').map(d => d.outputId)
        );
        for (const rec of targetLedger.records) {
          if (!approvedIds.has(rec.id)) {
            errors.push({
              kind: 'outputCoverage',
              path: targetRoot,
              detail: `Unapproved record ${rec.id} found in ${phase} output`
            });
            dimensions.coverage.valid = false;
          }
        }
      }
    }
  }

  // 3. Closure Delta validation
  if (options.delta || phase === 'closure') {
    const deltaPath = options.delta ? path.resolve(options.delta) : path.join(outDir, 'post-verification-delta.json');
    if (!fs.existsSync(deltaPath)) {
      errors.push({
        kind: 'requiredInputMissing',
        path: 'post-verification-delta.json',
        detail: 'Closure delta artifact is missing'
      });
      dimensions.closure.valid = false;
    } else {
      let delta;
      try {
        delta = JSON.parse(fs.readFileSync(deltaPath, 'utf8'));
      } catch (e) {
        errors.push({ kind: 'invalidInputJson', path: 'post-verification-delta.json', detail: e.message });
        dimensions.closure.valid = false;
      }

      if (delta) {
        // Predecessor proof check
        const proofLivePath = path.join(outDir, "conservation-live.json");
        const preclosurePath = path.join(outDir, "preclosure-records.json");
        let proofJson = null;
        let preclosureRecords = [];

        if (!fs.existsSync(proofLivePath) || !fs.existsSync(preclosurePath)) {
          errors.push({
            kind: "invalidPredecessorProof",
            path: "conservation-live.json",
            detail: "Predecessor live verification proof is missing"
          });
          dimensions.closure.valid = false;
        } else {
          try {
            proofJson = JSON.parse(fs.readFileSync(proofLivePath, "utf8"));
          } catch {}
          if (!proofJson || typeof proofJson !== "object" || Object.keys(proofJson).length === 0 || !proofJson.ok) {
            errors.push({
              kind: "invalidPredecessorProof",
              path: "conservation-live.json",
              detail: "Predecessor live verification proof is empty or invalid"
            });
            dimensions.closure.valid = false;
          }

          try {
            preclosureRecords = JSON.parse(fs.readFileSync(preclosurePath, "utf8"));
          } catch {}
          if (!Array.isArray(preclosureRecords) || preclosureRecords.length === 0) {
            errors.push({
              kind: "invalidPredecessorProof",
              path: "preclosure-records.json",
              detail: "Preclosure snapshot records is empty or invalid"
            });
            dimensions.closure.valid = false;
          }
        }

        if (!Array.isArray(delta.entries) || delta.entries.length === 0) {
          errors.push({
            kind: "invalidClosureChange",
            path: "post-verification-delta.json",
            detail: "Closure delta entries array is empty"
          });
          dimensions.closure.valid = false;
        } else {
          if (!reconciliation || !Array.isArray(reconciliation.allowedClosureScope) || reconciliation.allowedClosureScope.length === 0) {
            errors.push({
              kind: "unauthorizedClosureId",
              path: "post-verification-delta.json",
              detail: "Reconciliation does not define allowedClosureScope"
            });
            dimensions.closure.valid = false;
          }
          const allowedScope = new Set(reconciliation?.allowedClosureScope || []);
          const seenDeltaIds = new Set();

          for (const entry of delta.entries) {
            if (!entry.id) {
              errors.push({ kind: "invalidClosureChange", path: "post-verification-delta.json", detail: "Entry missing ID" });
              dimensions.closure.valid = false;
              continue;
            }

            if (seenDeltaIds.has(entry.id)) {
              errors.push({ kind: "invalidClosureChange", path: "post-verification-delta.json", detail: "Duplicate closure entry for " + entry.id });
              dimensions.closure.valid = false;
            }
            seenDeltaIds.add(entry.id);

            // 1. Scope authorization check
            if (!allowedScope.has(entry.id)) {
              errors.push({
                kind: "unauthorizedClosureId",
                path: "post-verification-delta.json",
                detail: "Entry " + entry.id + " is outside allowed closure scope: " + Array.from(allowedScope).join(", ")
              });
              dimensions.closure.valid = false;
            }

            // 2. Cell hash checks
            if (entry.beforeCellsHash) {
              const computedBeforeHash = Array.isArray(entry.beforeCells) ? sha256(Buffer.from(JSON.stringify(entry.beforeCells))) : null;
              if (entry.beforeCellsHash.startsWith("FORGED") || (computedBeforeHash && entry.beforeCellsHash !== computedBeforeHash) || !entry.beforeCells) {
                errors.push({ kind: "invalidClosureHash", path: "post-verification-delta.json", detail: "Forged or mismatched cell hash on " + entry.id });
                dimensions.closure.valid = false;
              }
            }

            // 3. Predecessor matching against snapshot
            if (Array.isArray(entry.beforeCells)) {
              const matchedPre = preclosureRecords.find(r => r.id === entry.id || (Array.isArray(r.cells) && r.cells[0] === entry.id));
              if (!matchedPre) {
                errors.push({
                  kind: "invalidClosurePredecessor",
                  path: "post-verification-delta.json",
                  detail: "Entry " + entry.id + " does not exist in proven predecessor snapshot"
                });
                dimensions.closure.valid = false;
              } else {
                const provenCells = Array.isArray(matchedPre.cells) ? matchedPre.cells : matchedPre.outputCells;
                if (provenCells && (provenCells[4] !== entry.beforeCells[4] || provenCells[6] !== entry.beforeCells[6])) {
                  errors.push({
                    kind: "invalidClosurePredecessor",
                    path: "post-verification-delta.json",
                    detail: "beforeCells for " + entry.id + " does not match proven predecessor record"
                  });
                  dimensions.closure.valid = false;
                }
              }
            }

            // 4. Missing cells or incomplete entry check (RG04)
            if (!entry.domain && !Array.isArray(entry.beforeCells) && !Array.isArray(entry.afterCells) && !entry.beforeCellsHash) {
              errors.push({
                kind: "invalidClosureChange",
                path: "post-verification-delta.json",
                detail: "Incomplete closure entry for " + entry.id
              });
              dimensions.closure.valid = false;
            }

            // 5. Immutable fields check: [0] ID, [1] Type, [4] Title, [5] Source, [7] Found
            if (Array.isArray(entry.beforeCells) && Array.isArray(entry.afterCells)) {
              const immutableIndices = [0, 1, 4, 5, 7];
              for (const idx of immutableIndices) {
                if (entry.beforeCells[idx] !== entry.afterCells[idx]) {
                  errors.push({
                    kind: "invalidClosureChange",
                    path: "post-verification-delta.json",
                    detail: "Immutable field index " + idx + " changed on " + entry.id
                  });
                  dimensions.closure.valid = false;
                  break;
                }
              }
            }
          }
        }
      }
    }
  }

  const ok = errors.length === 0;
  return { ok, phase, errors, warnings, dimensions };
}

/**
 * Create immutable recovery proof artifact.
 */
function createRecoveryProof(context, result, options = {}) {
  const phase = options.phase || 'verify';
  const outDir = context.outDir || options.outDir;
  if (!outDir) return null;

  // Dedicated report destination
  if (options.reportOut) {
    const reportPath = options.reportOut;
    fs.mkdirSync(path.dirname(reportPath), { recursive: true });
    fs.writeFileSync(reportPath, JSON.stringify(result, null, 2) + '\n', 'utf8');
  }

  // Predecessor proof is created ONCE only upon successful verification.
  // Never overwrite predecessor on failed verify, retry, or delta verify.
  if (!result.ok || context.delta || options.delta) {
    return null;
  }

  const targetName = phase === 'staged' ? 'conservation-staged.json' : 'conservation-live.json';
  const proofPath = path.join(outDir, targetName);
  if (phase === 'live' && fs.existsSync(proofPath)) {
    // Immutable predecessor: do not overwrite
    return null;
  }

  const proof = {
    schemaVersion: 2,
    contractVersion: 1,
    createdAt: new Date().toISOString(),
    phase,
    ok: result.ok,
    errors: result.errors,
    warnings: result.warnings,
    dimensions: result.dimensions,
    hashes: {
      inventorySha256: context.inventoryHash,
      reconciliationSha256: context.reconciliationHash,
      manifestSha256: context.manifestHash
    }
  };

  fs.writeFileSync(proofPath, JSON.stringify(proof, null, 2) + '\n', 'utf8');

  if (phase === 'live' && result.ok) {
    const liveLedger = bm.readProjectLedger(context.root);
    const prePath = path.join(outDir, 'preclosure-records.json');
    if (!fs.existsSync(prePath)) {
      fs.writeFileSync(prePath, JSON.stringify(liveLedger.records, null, 2) + '\n', 'utf8');
    }
  }

  return proof;
}

module.exports = {
  captureRecoveryInputs,
  loadRecoveryContract,
  validateRecoveryContract,
  createRecoveryProof
};
