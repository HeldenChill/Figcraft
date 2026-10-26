'use strict';
const fs = require('fs');
const path = require('path');

const INDEX_REL = '.cursor/memory/mem-arch-assemblies-canonical.md';
const LEDGER_REL = '.cursor/memory/mem-known-bugs-index.md';
const BEGIN = '<!-- LOCAL-MEMORY-INDEX:BEGIN -->';
const END = '<!-- LOCAL-MEMORY-INDEX:END -->';
// Two columns, second column is exactly one mem file or special hub bugs/README.md (the form the hook parses).
const ROW_OK = /^\|[^|]+\|\s*`?(mem-[^|`\s]+\.md|bugs\/README\.md)`?\s*\|\s*$/;
// Two-column row pointing at a non-memory path (e.g. `portable-experience/README.md`): valid, not a mem row.
const ROW_OTHER = /^\|[^|]+\|\s*`[^`|]+`\s*\|\s*$/;

function readText(p) {
  try { return fs.readFileSync(p, 'utf8'); } catch { return null; }
}

function toSlash(s) { return String(s).replace(/\\/g, '/'); }

function re(pattern) {
  if (pattern.startsWith('(?i)')) return new RegExp(pattern.slice(4), 'i');
  return new RegExp(pattern);
}

function parseRegistry(root) {
  const text = readText(path.join(root, INDEX_REL));
  if (text == null) return null;
  const s = text.indexOf(BEGIN);
  const e = text.indexOf(END);
  if (s < 0 || e < s) return { rows: [], bad: [], missingSentinels: true };
  const h = /^## Memory index\s*$/m.exec(text);
  const from = h && h.index < s ? h.index : s;
  const next = /^## /m.exec(text.slice(e));
  const to = next ? e + next.index : text.length;
  const rows = [];
  const bad = [];
  for (const line of text.slice(from, to).split(/\r?\n/)) {
    if (!line.startsWith('|')) continue;
    if (/^\|\s*:?-{3,}/.test(line) || /^\|\s*Topic\s*\|/i.test(line)) continue;
    const m = line.match(ROW_OK);
    if (m) rows.push(toSlash(m[1])); else if (!ROW_OTHER.test(line)) bad.push(line.slice(0, 120));
  }
  return { rows, bad, missingSentinels: false };
}

function listMemoryFiles(root) {
  const dir = path.join(root, '.cursor', 'memory');
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir).filter((f) => /^mem-.*\.md$/.test(f));
}

function extractLinks(text) {
  const out = [];
  const rx = /\]\(([^)\s]+)\)/g;
  let m;
  while ((m = rx.exec(text))) {
    const target = m[1];
    if (/^(https?:|mailto:|#)/.test(target)) continue;
    const bare = target.split('#')[0];
    try { out.push(decodeURI(bare)); } catch { out.push(bare); }
  }
  return out.filter((t) => t.length > 0);
}

function ledgerSection(text) {
  const m = /^## Ledger\s*$/m.exec(text);
  if (!m) return text;
  const rest = text.slice(m.index + m[0].length);
  const next = /^## /m.exec(rest);
  return next ? rest.slice(0, next.index) : rest;
}

function ledgerIds(text) {
  const counts = new Map();
  for (const line of ledgerSection(text).split(/\r?\n/)) {
    const m = line.match(/^\|\s*(BUG-[0-9A-Za-z-]+)\s*\|/);
    if (m) counts.set(m[1], (counts.get(m[1]) || 0) + 1);
  }
  return counts;
}

function linkSources(root) {
  const out = ['CLAUDE.md', 'AGENTS.md', '.claude/rules-README.md'];
  const rules = path.join(root, '.claude', 'rules');
  if (fs.existsSync(rules)) {
    for (const f of fs.readdirSync(rules)) if (f.endsWith('.md')) out.push(`.claude/rules/${f}`);
  }
  return out;
}

module.exports = {
  INDEX_REL, LEDGER_REL, readText, toSlash, re, parseRegistry,
  listMemoryFiles, extractLinks, ledgerIds, linkSources,
};
