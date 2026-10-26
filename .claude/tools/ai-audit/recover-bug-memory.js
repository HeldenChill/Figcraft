#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const recovery = require('./lib/bug-memory-recovery');
const transaction = require('./lib/bug-memory-transaction');
const contract = require('./lib/recovery-contract');

function parseArgs(args) {
  const result = { _: [] };
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a.startsWith('--')) {
      const key = a.slice(2);
      if (i + 1 < args.length && !args[i + 1].startsWith('--')) {
        result[key] = args[i + 1];
        i++;
      } else {
        result[key] = true;
      }
    } else {
      result._.push(a);
    }
  }
  return result;
}

function main() {
  const parsed = parseArgs(process.argv.slice(2));
  const command = parsed._[0];

  if (!command) {
    console.error('Usage: recover-bug-memory.js <inventory|prepare|verify|apply|rollback> [options]');
    process.exit(1);
  }

  const root = parsed.project ? path.resolve(parsed.project) : process.cwd();
  const outDir = parsed.out ? path.resolve(parsed.out) : path.join(root, '.cursor/audits/ai-config/2026-10-10-bug-memory-recovery');

  try {
    switch (command) {
      case 'inventory': {
        const legacy = parsed.legacy ? path.resolve(parsed.legacy) : path.join(root, '.cursor/audits/ai-config/2026-10-10-bug-memory-migration/original-ledger.txt');
        const inv = recovery.inventoryRecovery(root, { legacy, out: outDir });
        if (parsed.json) {
          console.log(JSON.stringify({
            schemaVersion: 2,
            contractVersion: 1,
            ok: true,
            phase: 'inventory',
            errors: [],
            warnings: [],
            ...inv
          }, null, 2));
        } else {
          console.log(`Inventory captured: ${inv.legacyCensus.totalRows} legacy rows (${inv.legacyCensus.byKind.record || 0} records, ${inv.legacyCensus.byKind.history || 0} history, ${inv.legacyCensus.byKind.malformed || 0} malformed).`);
          console.log(`Live collection: ${inv.liveFiles.length} files, ${inv.liveOccurrences.length} occurrences.`);
        }
        break;
      }

      case 'prepare': {
        const mapping = parsed.mapping ? path.resolve(parsed.mapping) : path.join(outDir, 'reconciliation.json');

        // Contract preflight check
        const ctx = contract.loadRecoveryContract(root, { out: outDir, mapping, phase: 'prepare' });
        const val = contract.validateRecoveryContract(ctx, { phase: 'prepare' });
        if (!val.ok) {
          const err = new Error('Reconciliation validation failed:\n' + val.errors.map(e => e.detail || e.kind).join('\n'));
          err.kind = val.errors[0]?.kind || 'reconciliationCoverage';
          err.path = val.errors[0]?.path || 'reconciliation.json';
          throw err;
        }

        const manifest = recovery.prepareRecovery(root, { out: outDir, mapping });
        if (parsed.json) {
          console.log(JSON.stringify({
            schemaVersion: 2,
            contractVersion: 1,
            ok: true,
            phase: 'prepare',
            errors: [],
            warnings: [],
            ...manifest
          }, null, 2));
        } else {
          console.log(`Recovery prepared: ${manifest.writes.length} staged file writes ready.`);
        }
        break;
      }

      case 'verify': {
        const staged = Boolean(parsed.staged);
        const reportOut = parsed['report-out'] ? path.resolve(parsed['report-out']) : null;
        const delta = parsed.delta ? path.resolve(parsed.delta) : null;

        // Verify via contract
        const ctx = contract.loadRecoveryContract(root, { out: outDir, phase: staged ? 'staged' : 'live', delta });
        const val = contract.validateRecoveryContract(ctx, { phase: staged ? 'staged' : 'live', delta });
        const proof = contract.createRecoveryProof(ctx, val, { phase: staged ? 'staged' : 'live', delta, reportOut });

        const result = {
          schemaVersion: 2,
          contractVersion: 1,
          ok: val.ok,
          phase: 'verify',
          target: staged ? 'staged' : 'live',
          errors: val.errors,
          warnings: val.warnings,
          dimensions: val.dimensions,
          ledgerValidation: { valid: val.ok, recordCount: ctx.inventory?.liveOccurrences?.length || 0 }
        };

        if (parsed.json) {
          console.log(JSON.stringify(result, null, 2));
          if (!result.ok) {
            process.exit(1);
          }
        } else {
          if (result.ok) {
            console.log(`Verification PASSED (${result.target}). Ledger records: ${result.ledgerValidation?.recordCount || 'N/A'}.`);
          } else {
            console.error(`Verification FAILED with ${result.errors.length} errors:`);
            for (const err of result.errors) console.error('  - ' + (err.detail || err.kind || err));
            process.exit(1);
          }
        }
        break;
      }

      case 'apply': {
        const manifestPath = path.join(outDir, 'prepared-manifest.json');
        if (!fs.existsSync(manifestPath)) {
          const err = new Error('Prepared manifest missing: run prepare first');
          err.kind = 'requiredInputMissing';
          err.path = 'prepared-manifest.json';
          throw err;
        }
        // Mandatory preflight check: validate staged conservation contract before live mutation
        const ctx = contract.loadRecoveryContract(root, { out: outDir, phase: 'staged' });
        const val = contract.validateRecoveryContract(ctx, { phase: 'staged' });
        if (!val.ok) {
          const err = new Error('Apply preflight conservation failed: ' + val.errors.map(e => e.detail || e.kind).join('; '));
          err.kind = val.errors[0]?.kind || 'evidencePreservation';
          err.errors = val.errors;
          throw err;
        }

        const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
        const applied = transaction.applyTransaction(root, outDir, manifest, { validatedContract: true });
        if (parsed.json) {
          console.log(JSON.stringify({
            schemaVersion: 2,
            contractVersion: 1,
            ok: true,
            phase: 'apply',
            errors: [],
            warnings: [],
            ...applied
          }, null, 2));
        } else {
          console.log(`Apply completed: state=${applied.state}, ${applied.filesWritten.length} files written.`);
        }
        break;
      }

      case 'rollback': {
        const rolled = transaction.rollbackTransaction(root, outDir);
        if (parsed.json) {
          console.log(JSON.stringify({
            schemaVersion: 2,
            contractVersion: 1,
            ok: true,
            phase: 'rollback',
            errors: [],
            warnings: [],
            ...rolled
          }, null, 2));
        } else {
          console.log(`Rollback completed: state=${rolled.state}, ${rolled.filesRestored.length} files restored.`);
        }
        break;
      }

      default:
        console.error(`Unknown subcommand: ${command}. Use inventory|prepare|verify|apply|rollback.`);
        process.exit(1);
    }
  } catch (err) {
    if (parsed.json) {
      console.log(JSON.stringify({
        schemaVersion: 2,
        contractVersion: 1,
        ok: false,
        phase: command || 'unknown',
        errors: [{
          kind: err.kind || 'executionError',
          path: err.path || 'unknown',
          detail: err.message
        }],
        warnings: []
      }, null, 2));
    } else {
      console.error(`Error: ${err.message}`);
    }
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}

module.exports = { main };
