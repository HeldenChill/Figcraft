#!/usr/bin/env node
'use strict';
// Compare two L3 summaries. Exit 1 on any unexpected pass -> fail.
const fs = require('fs');
const path = require('path');

function compare(base, cur, scenarios) {
  const key = (r) => `${r.id}/${r.client}`;
  const before = new Map(base.results.map((r) => [key(r), r.pass]));
  const out = { regressions: [], expected: [], fixed: [] };
  for (const r of cur.results) {
    const was = before.get(key(r));
    if (was === true && !r.pass) {
      (scenarios[r.id] && scenarios[r.id].expectedChange ? out.expected : out.regressions).push(key(r));
    } else if (was === false && r.pass) out.fixed.push(key(r));
  }
  return out;
}

if (require.main === module) {
  const [baseFile, curFile, dir] = process.argv.slice(2);
  const scenarios = Object.fromEntries(fs.readdirSync(dir).filter((f) => f.endsWith('.json'))
    .map((f) => JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'))).map((s) => [s.id, s]));
  const r = compare(JSON.parse(fs.readFileSync(baseFile, 'utf8')), JSON.parse(fs.readFileSync(curFile, 'utf8')), scenarios);
  console.log(JSON.stringify(r, null, 2));
  process.exit(r.regressions.length ? 1 : 0);
}

module.exports = { compare };
