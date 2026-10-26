#!/usr/bin/env node
// UserPromptSubmit hook: points the model at project memory within a fixed budget.
// Prints the HUB rows of the memory index plus how to grep the full index (repair plan Q1 c).
// Claude Code replaces hook output above its limit with a short preview, so the budget is hard.
const fs = require("fs");
const path = require("path");

const BUDGET = 3000;
const INDEX_REL = ".cursor/memory/mem-arch-assemblies-canonical.md";

function hubRows(text) {
  const rows = [];
  for (const line of text.split(/\r?\n/)) {
    const m = line.match(/^\|\s*HUB:\s*([^|]+?)\s*\|\s*`?([^|`\s]+\.md)`?\s*\|\s*$/);
    if (m) {
      const target = m[2].trim();
      if (path.isAbsolute(target) || target.startsWith('/') || target.startsWith('\\') || target.includes('..')) {
        continue;
      }
      rows.push(`  - ${m[1].trim()} -> ${target}`);
    }
  }
  return rows;
}

function build(text, budget) {
  const head =
    "PROJECT MEMORY (memory-first: .claude/rules/memory-first-workflow.md). Before reading source, check memory.\n" +
    `Find any topic: grep -i "<keyword>" ${INDEX_REL}  (rows: | description | \`mem-file.md\` |), then read that .cursor/memory file.\n` +
    "Domain hubs (special hubs like bugs/README.md index their domain children):";
  let out = head;
  for (const r of hubRows(text)) {
    if (out.length + 1 + r.length > budget) break;
    out += "\n" + r;
  }
  return out;
}

module.exports = { build, BUDGET, hubRows };

if (require.main === module) {
  process.stdin.on("data", () => {});
  process.stdin.on("end", () => {
    try {
      const root = process.env.CLAUDE_PROJECT_DIR || process.cwd();
      const file = path.join(root, INDEX_REL);
      if (!fs.existsSync(file)) return;
      const additionalContext = build(fs.readFileSync(file, "utf8"), BUDGET);
      process.stdout.write(JSON.stringify({ hookSpecificOutput: { hookEventName: "UserPromptSubmit", additionalContext } }));
    } catch (e) {
      // A hook must never break the prompt.
    }
  });
}
