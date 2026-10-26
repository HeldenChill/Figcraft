#!/usr/bin/env node
'use strict';
// L1 load snapshot: what each client loads at session start, and which policies are reachable.
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const lib = require('./lib/index');

function fileEntry(root, rel) {
  const text = lib.readText(path.join(root, rel));
  return text == null ? null : { path: rel, bytes: Buffer.byteLength(text), text };
}

function runHook(root) {
  const hook = path.join(root, '.claude', 'hooks', 'memory-index-injector.js');
  if (!fs.existsSync(hook)) return '';
  const r = spawnSync('node', [hook], {
    input: '{}', encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, env: { ...process.env, CLAUDE_PROJECT_DIR: root },
  });
  try { return JSON.parse(r.stdout).hookSpecificOutput.additionalContext || ''; } catch { return ''; }
}

function snapshot(root, policies, opts) {
  const rulesDir = path.join(root, '.claude', 'rules');
  const ruleFiles = fs.existsSync(rulesDir)
    ? fs.readdirSync(rulesDir).filter((f) => f.endsWith('.md')).map((f) => `.claude/rules/${f}`)
    : [];
  const claudeFiles = ['CLAUDE.md', ...ruleFiles].map((r) => fileEntry(root, r)).filter(Boolean);
  const codexFiles = ['AGENTS.md'].map((r) => fileEntry(root, r)).filter(Boolean);

  const hook = runHook(root);
  const visible = hook.length > opts.hookLimit ? hook.slice(0, opts.previewBytes) : hook;
  const claudeText = claudeFiles.map((f) => f.text).join('\n') + '\n' + visible;
  const codexText = codexFiles.map((f) => f.text).join('\n');

  return {
    claude: {
      files: claudeFiles.map(({ path: p, bytes }) => ({ path: p, bytes })),
      hookBytes: hook.length,
      hookVisibleBytes: visible.length,
    },
    codex: { files: codexFiles.map(({ path: p, bytes }) => ({ path: p, bytes })) },
    pkgRules: ruleFiles.filter((r) => /\/pkg-[^/]+\.md$/.test(r)).length,
    policies: policies.map((p) => ({
      id: p.id, flows: p.flows, claude: lib.re(p.anchor).test(claudeText), codex: lib.re(p.anchor).test(codexText),
    })),
  };
}

function argValue(name, dflt) {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : dflt;
}

if (require.main === module) {
  const root = path.resolve(argValue('--project', process.cwd()));
  const policies = JSON.parse(fs.readFileSync(argValue('--policies'), 'utf8'));
  const snap = snapshot(root, policies, {
    hookLimit: Number(argValue('--hook-limit', 10000)),
    previewBytes: Number(argValue('--preview', 2000)),
  });
  process.stdout.write(JSON.stringify({ project: root, ...snap }, null, 2));
}

module.exports = { snapshot };
