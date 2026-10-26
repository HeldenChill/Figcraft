#!/usr/bin/env node
'use strict';
// L3 golden scenarios. Runs each scenario N times per client in one reusable sparse worktree.
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');
const lib = require('./lib/index');
const { fromClaude, fromCodex } = require('./lib/normalize');
const { evaluateAll } = require('./lib/checks');
const wtLib = require('./lib/worktree');
const { guardDir, guardRepo, resolveGuardRepos } = require('./lib/guard');

const CLAUDE_ARGS = [
  '-p', '--output-format', 'stream-json', '--verbose', '--no-session-persistence',
  '--permission-mode', 'acceptEdits',
  '--allowedTools', 'Bash(dotnet build:*)', 'Bash(git status:*)', 'Bash(git diff:*)', 'Bash(ls:*)', 'Bash(grep:*)',
];
const CODEX_ARGS = (wt) => ['exec', '--json', '--sandbox', 'workspace-write', '-C', wt, '-'];

function defaultExec(cmd, args, opts) {
  // shell:true resolves claude.cmd on Windows; args are fixed tokens, the prompt goes via stdin.
  const quoted = args.map((a) => (/[\s()*:]/.test(a) ? `"${a}"` : a));
  return spawnSync(cmd, quoted, { ...opts, shell: true, encoding: 'utf8', maxBuffer: 512 * 1024 * 1024 });
}

// Usage / rate-limit exhaustion (5-hour window etc.). Such a run says nothing about the config.
function isQuota(text) {
  return /usage limit|session limit|limit reached|hit your .{0,20}limit|rate.?limit|too many requests|\b429\b|quota|try again at|resets? (at|in)\b/i.test(String(text || ''));
}

function runOne({ client, prompt, wt, timeoutMs, exec = defaultExec, models = {} }) {
  const base = client === 'claude' ? CLAUDE_ARGS : CODEX_ARGS(wt);
  const pin = models[client];
  // Pinned model goes before the trailing args so codex keeps '-' (stdin) last.
  const flag = client === 'claude' ? ['--model', pin] : ['-m', pin];
  const args = pin ? [base[0], ...flag, ...base.slice(1)] : base;
  const cmd = client;
  const r = exec(cmd, args, { cwd: wt, input: prompt, timeout: timeoutMs });
  const timedOut = r.error && (r.error.code === 'ETIMEDOUT' || r.signal === 'SIGTERM');
  const reason = timedOut ? 'timeout' : r.status === 0 ? 'ok' : `exit ${r.status}`;
  // Claude reports an exhausted window as an ordinary exit-0 reply, so the short final answer is checked too.
  const parsed = client === 'claude' ? fromClaude(r.stdout || '') : fromCodex(r.stdout || '');
  // rate_limit_event lines are status telemetry (also emitted when allowed), not a stop; drop them before matching.
  const noTelemetry = (r.stdout || '').split('\n').filter((l) => !l.includes('"type":"rate_limit_event"')).join('\n');
  const quota = (reason !== 'ok' && isQuota(noTelemetry + ' ' + (r.stderr || '')))
    || (parsed.answer.length < 200 && isQuota(parsed.answer));
  return { raw: r.stdout || '', exitCode: r.status, reason, quota };
}

// Pairs safe to skip on --resume: every run executed (reason ok). Errored/timed-out pairs are redone.
function completedKeys(summary, runs) {
  const keys = new Set();
  for (const r of (summary && summary.results) || []) {
    if (r.runs_detail.length >= runs && r.runs_detail.every((d) => d.reason === 'ok')) keys.add(`${r.id}/${r.client}`);
  }
  return keys;
}

function verdict(passes, runs) { return passes >= Math.ceil((runs * 2) / 3); }

function mermaidOk(code) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'mmd-'));
  const inFile = path.join(dir, 'd.mmd');
  fs.writeFileSync(inFile, code);
  const r = spawnSync('mmdc', ['-i', inFile, '-o', path.join(dir, 'd.svg')], { shell: true, encoding: 'utf8' });
  fs.rmSync(dir, { recursive: true, force: true });
  return r.status === 0;
}

function claudeSlugDir(wt) {
  return path.join(os.homedir(), '.claude', 'projects', path.resolve(wt).replace(/[:\\/]/g, '-'));
}

function version(cmd) {
  const r = spawnSync(cmd, ['--version'], { shell: true, encoding: 'utf8' });
  return (r.stdout || '').trim();
}

function loadScenarios(dir) {
  return fs.readdirSync(dir).filter((f) => f.endsWith('.json'))
    .map((f) => JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8')));
}

function arg(name, dflt) {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : dflt;
}

function main() {
  const project = path.resolve(arg('--project'));
  const only = arg('--only') ? arg('--only').split(',') : null;
  const scenarios = loadScenarios(arg('--scenarios')).filter((s) => !only || only.includes(s.id))
    .map((s) => ({ ...s, guardRepos: resolveGuardRepos(s.guardRepos || [], { project, sharedRepo: arg('--shared-repo') }) }));
  if (process.argv.includes('--preflight')) {
    console.log(JSON.stringify({ project, scenarios: scenarios.map((s) => ({ id: s.id, guardRepos: s.guardRepos })) }, null, 2));
    return;
  }
  const out = path.resolve(arg('--out'));
  const runs = Number(arg('--runs', 3));
  const clients = arg('--clients', 'claude,codex').split(',');
  const wt = path.resolve(arg('--worktree', path.join(os.tmpdir(), 'ai-audit-wt')));
  const timeoutMs = Number(arg('--timeout-min', 15)) * 60 * 1000;
  const models = { claude: arg('--model-claude'), codex: arg('--model-codex') };

  // The worktree session writes auto-memory into the PROJECT's dir, not the worktree's.
  const projectMemDir = path.join(os.homedir(), '.claude', 'projects', path.resolve(project).replace(/[:\\/]/g, '-'), 'memory');
  fs.mkdirSync(path.join(out, 'raw'), { recursive: true });
  wtLib.ensureWorktree(project, wt, { sparse: true });
  wtLib.resetWorktree(wt);
  const bugs = require('./lib/bug-memory');
  const baseline = bugs.readProjectLedger(wt);
  const baseDupIds = new Set([...bugs.countIds(baseline.records)].filter((entry) => entry[1] > 1).map((entry) => entry[0]));

  const summaryFile = path.join(out, 'summary.json');
  const prior = process.argv.includes('--resume') && fs.existsSync(summaryFile)
    ? JSON.parse(fs.readFileSync(summaryFile, 'utf8')) : null;
  const done = prior ? completedKeys(prior, runs) : new Set();
  const summary = prior
    ? { ...prior, resumedAt: new Date().toISOString(), results: prior.results.filter((r) => done.has(`${r.id}/${r.client}`)) }
    : { startedAt: new Date().toISOString(), claudeVersion: version('claude'), codexVersion: version('codex'), runs, models, results: [] };

  for (const s of scenarios) {
    for (const client of s.clients.filter((c) => clients.includes(c))) {
      if (done.has(`${s.id}/${client}`)) continue;
      const detail = [];
      for (let n = 1; n <= runs; n++) {
        wtLib.resetWorktree(wt);
        const guards = [guardDir(projectMemDir), ...(s.guardRepos || []).map(guardRepo)];
        let r;
        try { r = runOne({ client, prompt: s.prompt, wt, timeoutMs, models }); } finally { guards.forEach((g) => g.restore()); }
        fs.writeFileSync(path.join(out, 'raw', `${s.id}-${client}-${n}.jsonl`), r.raw);
        if (r.quota) {
          console.log(`QUOTA: ${client} limit hit at ${s.id} #${n}; stopped, run again with --resume`);
          fs.writeFileSync(summaryFile, JSON.stringify(summary, null, 2));
          process.exit(75);
        }
        const { events, answer } = client === 'claude' ? fromClaude(r.raw) : fromCodex(r.raw);
        const ctx = {
          events, answer, root: wt, baseDupIds, mermaid: mermaidOk,
          changed: wtLib.changedFiles(wt),
          readFile: (rel) => lib.readText(path.join(wt, rel)),
          addedLines: (rel) => wtLib.addedLines(wt, rel),
        };
        const checks = evaluateAll(s.checks, ctx, client);
        const pass = r.reason === 'ok' && checks.every((c) => c.pass);
        detail.push({ n, pass, reason: r.reason, checks });
        console.log(`${s.id} ${client} #${n}: ${pass ? 'PASS' : 'FAIL'} (${r.reason})`);
      }
      const passes = detail.filter((d) => d.pass).length;
      summary.results.push({ id: s.id, client, passes, runs, pass: verdict(passes, runs), runs_detail: detail });
      fs.writeFileSync(summaryFile, JSON.stringify(summary, null, 2));
    }
  }
  wtLib.resetWorktree(wt);
  fs.rmSync(claudeSlugDir(wt), { recursive: true, force: true }); // F12 auto-memory writes land here

  const md = ['| Flow | Client | Passes | Verdict |', '|---|---|---|---|',
    ...summary.results.map((r) => `| ${r.id} | ${r.client} | ${r.passes}/${r.runs} | ${r.pass ? 'pass' : '**FAIL**'} |`)];
  fs.writeFileSync(path.join(out, 'summary.md'), md.join('\n') + '\n');
}

if (require.main === module) main();

module.exports = { runOne, verdict, isQuota, completedKeys };
