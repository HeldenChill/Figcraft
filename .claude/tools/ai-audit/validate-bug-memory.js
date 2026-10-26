#!/usr/bin/env node
'use strict';

const path = require('node:path');
const bugs = require('./lib/bug-memory');

function parseArgs(args) {
  const options = {
    project: '.',
    ledger: null,
    recordsDir: null,
    knownMalformedIds: new Set(),
    json: false
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--project' && i + 1 < args.length) {
      options.project = args[++i];
    } else if (arg === '--ledger' && i + 1 < args.length) {
      options.ledger = args[++i];
    } else if (arg === '--records-dir' && i + 1 < args.length) {
      options.recordsDir = args[++i];
    } else if (arg === '--known-malformed-ids' && i + 1 < args.length) {
      const raw = args[++i];
      for (const id of raw.split(',')) {
        const trimmed = id.trim();
        if (trimmed) options.knownMalformedIds.add(trimmed);
      }
    } else if (arg === '--json') {
      options.json = true;
    }
  }

  return options;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const root = path.resolve(options.project);

  let snapshot;
  try {
    snapshot = bugs.readProjectLedger(root);
  } catch (err) {
    if (options.json) {
      console.log(JSON.stringify({
        mode: 'error',
        files: [],
        recordCount: 0,
        errors: [{ kind: 'readError', detail: err.message }],
        warnings: []
      }, null, 2));
    } else {
      console.error(`Error reading ledger: ${err.message}`);
    }
    process.exit(1);
  }

  const validation = bugs.validateLedger(snapshot, {
    knownMalformedIds: options.knownMalformedIds
  });

  // If ledger was explicitly configured but produced 0 records
  if (snapshot.mode !== 'none' && snapshot.records.length === 0) {
    validation.errors.push({
      kind: 'noRecordsFound',
      detail: 'Configured ledger contains no records'
    });
  }

  if (options.json) {
    console.log(JSON.stringify({
      mode: snapshot.mode,
      files: snapshot.files,
      recordCount: snapshot.records.length,
      errors: validation.errors,
      warnings: validation.warnings
    }, null, 2));
  } else {
    for (const w of validation.warnings) {
      console.warn(`WARNING: ${w.kind} - ${w.detail}${w.location ? ' at ' + w.location : ''}`);
    }
    for (const e of validation.errors) {
      console.error(`ERROR: ${e.kind} - ${e.detail}${e.location ? ' at ' + e.location : ''}`);
    }
    if (validation.errors.length === 0) {
      console.log(`PASS: ${snapshot.records.length} unique ledger records (${validation.warnings.length} warnings) [mode: ${snapshot.mode}]`);
    } else {
      console.error(`FAIL: ${validation.errors.length} errors, ${validation.warnings.length} warnings [mode: ${snapshot.mode}]`);
    }
  }

  process.exit(validation.errors.length === 0 ? 0 : 1);
}

if (require.main === module) {
  main();
}
