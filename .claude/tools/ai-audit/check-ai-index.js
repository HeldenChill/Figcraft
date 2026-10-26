#!/usr/bin/env node
'use strict';
// C3 integrity check (L2). Usage: node check-ai-index.js --project <path> [--json]
// Hook mode (Claude PostToolUse): node check-ai-index.js --hook   (JSON on stdin; exit 2 + stderr on findings)
const fs = require('fs');
const path = require('path');
const lib = require('./lib/index');
const bugs = require('./lib/bug-memory');
const audits = require('./lib/audit-catalog');

const SELF = path.basename(lib.INDEX_REL);

function check(root) {
  const findings = [];
  const add = (kind, detail) => findings.push({ kind, detail });

  const reg = lib.parseRegistry(root);
  if (reg && reg.missingSentinels) add('indexSentinelsMissing', lib.INDEX_REL);
  if (reg && !reg.missingSentinels) {
    const set = new Set(reg.rows);
    for (const f of lib.listMemoryFiles(root)) {
      if (f !== SELF && !set.has(f)) add('memUnregistered', f);
    }
    for (const f of set) {
      if (!fs.existsSync(path.join(root, '.cursor', 'memory', f))) add('rowMissingFile', f);
    }
    for (const line of reg.bad) add('rowUnparseable', line);
  }

  // Validate ledger via bug-memory
  const snapshot = bugs.readProjectLedger(root);
  const result = bugs.validateLedger(snapshot);
  for (const finding of result.errors) add(finding.kind, finding.detail);

  // Validate audit catalog
  const config = bugs.readConfig(root);
  const catalog = audits.readAuditCatalog(root, config);
  for (const finding of audits.validateAuditCatalog(root, catalog).errors) {
    add(finding.kind, finding.detail);
  }

  for (const rel of lib.linkSources(root)) {
    const abs = path.join(root, rel);
    const text = lib.readText(abs);
    if (text == null) continue;
    for (const target of lib.extractLinks(text)) {
      if (!fs.existsSync(path.resolve(path.dirname(abs), target))) add('brokenLink', `${rel} -> ${target}`);
    }
  }
  return findings;
}

// Paths whose edit can break the index: memory files, audit files, rule files, rules README, root files.
function isRelevant(filePath) {
  const p = lib.toSlash(filePath || '');
  return /(?:^|\/)\.cursor\/(memory|audits)\/|(?:^|\/)\.claude\/rules\/|(?:^|\/)\.claude\/rules-README\.md$|(?:^|\/)(CLAUDE|AGENTS)\.md$/.test(p);
}

function runHook() {
  let raw = '';
  process.stdin.on('data', (c) => (raw += c));
  process.stdin.on('end', () => {
    let input = {};
    try { input = JSON.parse(raw); } catch (e) { process.exit(0); }
    const file = (input.tool_input && (input.tool_input.file_path || input.tool_input.notebook_path)) || '';
    if (!isRelevant(file)) process.exit(0);
    const root = path.resolve(process.env.CLAUDE_PROJECT_DIR || input.cwd || process.cwd());
    const findings = check(root);
    if (!findings.length) process.exit(0);
    process.stderr.write(`AI index check (check-ai-index.js) found ${findings.length} problem(s) after editing ${file}:\n` +
      findings.slice(0, 20).map((f) => `  ${f.kind}: ${f.detail}`).join('\n') + '\nFix them before continuing.\n');
    process.exit(2);
  });
}

function argValue(name) {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

if (require.main === module) {
  if (process.argv.includes('--hook')) {
    runHook();
  } else {
    const root = path.resolve(argValue('--project') || process.cwd());
    const findings = check(root);
    if (process.argv.includes('--json')) {
      process.stdout.write(JSON.stringify({ project: root, count: findings.length, findings }, null, 2));
    } else {
      for (const f of findings) console.log(`${f.kind}\t${f.detail}`);
      console.log(`${findings.length} finding(s) in ${root}`);
    }
    process.exit(findings.length ? 1 : 0);
  }
}

module.exports = { check, isRelevant };
