'use strict';
const { toSlash } = require('./index');

// Codex item.type -> normalized tool name. Keep in sync with test/fixtures/codex-sample.jsonl.
const CODEX_ITEM = { command_execution: 'Bash', file_change: 'Edit', agent_message: null };

function lines(text) {
  const out = [];
  for (const l of String(text).split(/\r?\n/)) {
    if (!l.trim()) continue;
    try { out.push(JSON.parse(l)); } catch { /* skip non-JSON noise */ }
  }
  return out;
}

function fromClaude(text) {
  const events = [];
  let answer = '';
  for (const o of lines(text)) {
    if (o.type === 'assistant' && o.message && Array.isArray(o.message.content)) {
      for (const c of o.message.content) {
        if (c.type === 'tool_use') events.push({ kind: 'tool', name: c.name, input: c.input || {} });
        else if (c.type === 'text' && c.text) events.push({ kind: 'text', text: c.text });
      }
    }
    if (o.type === 'result' && typeof o.result === 'string') answer = o.result;
  }
  return { events, answer };
}

function fromCodex(text) {
  const events = [];
  let answer = '';
  for (const o of lines(text)) {
    if (o.type !== 'item.completed' || !o.item) continue;
    const it = o.item;
    if (it.type === 'agent_message') {
      events.push({ kind: 'text', text: it.text || '' });
      answer = it.text || answer;
    } else if (CODEX_ITEM[it.type] === 'Bash') {
      events.push({ kind: 'tool', name: 'Bash', input: { command: it.command || '' } });
    } else if (CODEX_ITEM[it.type] === 'Edit') {
      for (const ch of it.changes || []) events.push({ kind: 'tool', name: 'Edit', input: { file_path: ch.path } });
    }
  }
  return { events, answer };
}

function toolPath(e) {
  if (!e || e.kind !== 'tool') return '';
  const p = e.input.file_path || e.input.path || e.input.notebook_path || '';
  return toSlash(p);
}

function isRead(e, rx) {
  if (!e || e.kind !== 'tool') return false;
  if (['Read', 'Grep', 'Glob'].includes(e.name)) return rx.test(toolPath(e)) || rx.test(toSlash(e.input.pattern || ''));
  if (e.name === 'Bash') return rx.test(toSlash(e.input.command || ''));
  return false;
}

module.exports = { fromClaude, fromCodex, isRead, toolPath };
