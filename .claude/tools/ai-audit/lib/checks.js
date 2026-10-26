'use strict';
const path = require('path');
const lib = require('./index');
const { isRead, toolPath } = require('./normalize');

const tools = (ctx, name) => ctx.events.filter((e) => e.kind === 'tool' && lib.re(name).test(e.name));
// Multiline variant that keeps the (?i) flag lib.re() set (rebuilding from .source alone drops it).
const reM = (p) => { const r = lib.re(p); return new RegExp(r.source, r.flags + 'm'); };
const changedMatching = (ctx, pathRegex) => ctx.changed.filter((p) => lib.re(pathRegex).test(p));
const mermaidBlocks = (md) => [...String(md).matchAll(/```mermaid\r?\n([\s\S]*?)```/g)].map((m) => m[1]);

const HANDLERS = {
  readBefore(c, ctx) {
    const t = ctx.events.findIndex((e) => isRead(e, lib.re(c.target)));
    const n = ctx.events.findIndex((e) => isRead(e, lib.re(c.notBefore)));
    return [t >= 0 && (n < 0 || t < n), `target@${t} notBefore@${n}`];
  },
  toolUsed(c, ctx) {
    const hit = tools(ctx, c.name).some((e) => !c.inputRegex || lib.re(c.inputRegex).test(JSON.stringify(e.input)));
    return [hit, hit ? 'used' : 'not used'];
  },
  noTool(c, ctx) { const n = tools(ctx, c.name).length; return [n === 0, `${n} call(s)`]; },
  commandRan(c, ctx) {
    const hit = tools(ctx, '^Bash$').some((e) => lib.re(c.regex).test(lib.toSlash(e.input.command || '')));
    return [hit, hit ? 'ran' : 'not run'];
  },
  pathTouched(c, ctx) {
    const hit = tools(ctx, c.name).some((e) => lib.re(c.pathRegex).test(toolPath(e)));
    return [hit, hit ? 'touched' : 'not touched'];
  },
  fileChanged(c, ctx) { const m = changedMatching(ctx, c.pathRegex); return [m.length > 0, m.join(', ') || 'none']; },
  noFileChanged(c, ctx) { const m = changedMatching(ctx, c.pathRegex); return [m.length === 0, m.join(', ') || 'none']; },
  fileContains(c, ctx) {
    const ok = changedMatching(ctx, c.pathRegex).some((p) => {
      const t = ctx.readFile(p) || '';
      return reM(c.contentRegex).test(t) && !(c.contentNotRegex && reM(c.contentNotRegex).test(t));
    });
    return [ok, ok ? 'match' : 'no matching file'];
  },
  diffAdds(c, ctx) {
    const ok = changedMatching(ctx, c.pathRegex).some((p) => lib.re(c.contentRegex).test(ctx.addedLines(p)));
    return [ok, ok ? 'added' : 'not added'];
  },
  diffAddsNot(c, ctx) {
    const bad = changedMatching(ctx, c.pathRegex).filter((p) => lib.re(c.contentRegex).test(ctx.addedLines(p)));
    return [bad.length === 0, bad.join(', ') || 'clean'];
  },
  answerRegex(c, ctx) { const ok = lib.re(c.regex).test(ctx.answer); return [ok, ok ? 'match' : 'no match']; },
  answerNotRegex(c, ctx) { const bad = lib.re(c.regex).test(ctx.answer); return [!bad, bad ? 'forbidden text present' : 'clean']; },
  textBeforeTool(c, ctx) {
    const t = ctx.events.findIndex((e) => e.kind === 'tool' && lib.re(c.name).test(e.name));
    if (t < 0) return [true, 'tool never called'];
    const ok = ctx.events.slice(0, t).some((e) => e.kind === 'text' && lib.re(c.textRegex).test(e.text));
    return [ok, ok ? 'text first' : 'tool before text'];
  },
  mermaidParses(c, ctx) {
    const blocks = changedMatching(ctx, c.pathRegex).flatMap((p) => mermaidBlocks(ctx.readFile(p) || ''));
    if (!blocks.length) return [false, 'no mermaid block'];
    const failed = blocks.filter((b) => !ctx.mermaid(b)).length;
    return [failed === 0, `${blocks.length - failed}/${blocks.length} parse`];
  },
  noNewDupBugIds(c, ctx) {
    const bugs = require('./bug-memory');
    const snapshot = bugs.readProjectLedger(ctx.root, { readText: ctx.readFile });
    if (snapshot.findings.length) {
      return [false, snapshot.findings.map(x => x.kind + ': ' + x.detail).join('; ')];
    }
    const fresh = [...bugs.countIds(snapshot.records)]
      .filter(([id, n]) => n > 1 && !ctx.baseDupIds.has(id)).map(([id]) => id);
    return [fresh.length === 0, fresh.join(', ') || 'none new'];
  },
  indexRegistered(c, ctx) {
    const reg = lib.parseRegistry(ctx.root);
    const rows = new Set(reg ? reg.rows : []);
    const bugs = require('./bug-memory');
    const config = bugs.readConfig(ctx.root);
    let declaredByHub = new Set();
    if (config.mode === 'collection' && config.ledger) {
      const snap = bugs.readProjectLedger(ctx.root, { readText: ctx.readFile });
      for (const f of snap.files) declaredByHub.add(path.posix.basename(f));
    }
    const missing = changedMatching(ctx, '^\\.cursor/memory/.*\\.md$')
      .map((p) => path.posix.basename(p))
      .filter((f) => {
        if (f === path.posix.basename(lib.INDEX_REL)) return false;
        if (f === path.posix.basename(lib.LEDGER_REL)) return false;
        if (declaredByHub.has(f)) return false;
        if (rows.has(f)) return false;
        return true;
      });
    return [missing.length === 0, missing.join(', ') || 'all registered'];
  },
};

function evaluate(check, ctx) {
  const h = HANDLERS[check.type];
  if (!h) return { type: check.type, pass: false, detail: 'unknown check type' };
  try {
    const [pass, detail] = h(check, ctx);
    return { type: check.type, pass: Boolean(pass), detail: String(detail) };
  } catch (e) {
    return { type: check.type, pass: false, detail: `error: ${e.message}` };
  }
}

function evaluateAll(checks, ctx, client) {
  return checks.filter((c) => !c.clients || c.clients.includes(client)).map((c) => evaluate(c, ctx));
}

module.exports = { evaluate, evaluateAll };
