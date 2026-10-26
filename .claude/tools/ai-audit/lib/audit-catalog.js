'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { splitRow, resolveInside } = require('./bug-memory');

function normalizePath(p) {
  return p.replace(/\\/g, '/');
}

function extractMarkdownLink(cell) {
  const match = cell.match(/\[([^\]]+)\]\(([^)]+)\)/);
  if (match) {
    return {
      text: match[1].trim(),
      url: match[2].trim()
    };
  }
  return null;
}

function readAuditCatalog(root, config = {}) {
  const auditRoot = config.auditRoot || '.cursor/audits';
  const catalogRel = normalizePath(path.join(auditRoot, 'README.md'));
  const catalogFull = path.join(root, catalogRel);

  const entries = [];
  const findings = [];

  if (!fs.existsSync(catalogFull)) {
    return { catalogPath: catalogRel, auditRoot: normalizePath(auditRoot), entries, findings };
  }

  const content = fs.readFileSync(catalogFull, 'utf8');
  const lines = content.split(/\r?\n/);
  const catalogDir = path.dirname(catalogRel);

  let inTable = false;

  for (let i = 0; i < lines.length; i++) {
    const lineNum = i + 1;
    const line = lines[i].trim();

    if (line.startsWith('|') && line.includes('|')) {
      const cells = splitRow(line);
      const lower = cells.map(c => c.toLowerCase());

      if (lower.length >= 6 && lower[0] === 'audit' && lower[1] === 'scope' && lower[2] === 'date' && lower[3] === 'report') {
        inTable = true;
        continue;
      }

      if (cells.every(c => /^:?-+:?$/.test(c))) {
        continue;
      }

      if (inTable && cells.length >= 6) {
        const auditTitle = cells[0];
        const scope = cells[1];
        const date = cells[2];
        const reportCell = cells[3];
        const evidenceCell = cells[4];
        const outcome = cells[5];

        const reportLink = extractMarkdownLink(reportCell);
        let reportRel = null;
        if (reportLink) {
          try {
            reportRel = normalizePath(path.join(catalogDir, decodeURIComponent(reportLink.url)));
          } catch {
            reportRel = normalizePath(path.join(catalogDir, reportLink.url));
          }
        } else {
          reportRel = reportCell.trim();
        }

        const evidenceLink = extractMarkdownLink(evidenceCell);
        let evidenceRel = null;
        if (evidenceLink) {
          try {
            evidenceRel = normalizePath(path.join(catalogDir, decodeURIComponent(evidenceLink.url)));
          } catch {
            evidenceRel = normalizePath(path.join(catalogDir, evidenceLink.url));
          }
        } else {
          evidenceRel = evidenceCell.trim();
        }

        entries.push({
          title: auditTitle,
          scope,
          date,
          report: reportRel,
          evidenceDir: evidenceRel,
          outcome,
          location: `${catalogRel}:${lineNum}`
        });
      }
    } else if (line.startsWith('#')) {
      inTable = false;
    }
  }

  return {
    catalogPath: catalogRel,
    auditRoot: normalizePath(auditRoot),
    entries,
    findings
  };
}

function findMarkdownLinks(text) {
  const links = [];
  const regex = /\[([^\]]*)\]\(([^)]+)\)/g;
  let match;
  while ((match = regex.exec(text)) !== null) {
    links.push({
      text: match[1],
      url: match[2].trim()
    });
  }
  return links;
}

function validateAuditCatalog(root, catalog) {
  const errors = [];
  const warnings = [];
  const auditRootNorm = catalog.auditRoot.replace(/\/$/, '') + '/';

  for (const f of catalog.findings || []) {
    errors.push(f);
  }

  const seenReports = new Set();
  const coveredDirs = new Set();
  const coveredReports = new Set();

  for (const entry of catalog.entries) {
    if (!entry.report) {
      errors.push({
        kind: 'emptyAuditReport',
        detail: `Catalog entry "${entry.title}" has no report link`,
        location: entry.location
      });
      continue;
    }

    // Validate path escape
    try {
      resolveInside(root, entry.report);
    } catch (err) {
      errors.push({
        kind: 'reportOutsideAuditRoot',
        detail: `Report path escapes root: ${entry.report} (${err.message})`,
        location: entry.location
      });
      continue;
    }

    const normReport = normalizePath(entry.report);
    if (!normReport.startsWith(auditRootNorm) && normReport !== catalog.auditRoot) {
      errors.push({
        kind: 'reportOutsideAuditRoot',
        detail: `Report "${normReport}" is outside configured audit root "${catalog.auditRoot}"`,
        location: entry.location
      });
    }

    if (seenReports.has(normReport)) {
      errors.push({
        kind: 'duplicateAuditReport',
        detail: `Duplicate report path in catalog: ${normReport}`,
        location: entry.location
      });
    }
    seenReports.add(normReport);
    coveredReports.add(normReport);

    const fullReport = path.join(root, normReport);
    if (!fs.existsSync(fullReport)) {
      errors.push({
        kind: 'missingReportFile',
        detail: `Cataloged report file does not exist: ${normReport}`,
        location: entry.location
      });
      continue;
    }

    if (entry.evidenceDir && entry.evidenceDir !== '—' && entry.evidenceDir !== '-') {
      const normEv = normalizePath(entry.evidenceDir).replace(/\/$/, '') + '/';
      coveredDirs.add(normEv);
    }
    // Also cover the directory containing the report if it's not the audit root
    const repDir = normalizePath(path.dirname(normReport)).replace(/\/$/, '') + '/';
    if (repDir !== auditRootNorm) {
      coveredDirs.add(repDir);
    }

    // Validate local links inside the report file
    try {
      const reportContent = fs.readFileSync(fullReport, 'utf8');
      const reportDir = path.dirname(fullReport);
      const links = findMarkdownLinks(reportContent);

      for (const link of links) {
        const u = link.url;
        if (/^https?:\/\/|^mailto:/i.test(u)) continue;
        if (u.startsWith('#')) continue;

        const pathPart = u.split('#')[0].trim();
        if (!pathPart) continue;

        let decoded;
        try {
          decoded = decodeURIComponent(pathPart);
        } catch {
          decoded = pathPart;
        }

        const targetFull = path.resolve(reportDir, decoded);
        if (!fs.existsSync(targetFull)) {
          errors.push({
            kind: 'brokenReportLink',
            detail: `Broken local link in ${normReport}: "${link.url}" target not found`,
            path: normReport
          });
        }
      }
    } catch (err) {
      errors.push({
        kind: 'readError',
        detail: `Failed to read report ${normReport}: ${err.message}`,
        path: normReport
      });
    }
  }

  // Check uncataloged reports below auditRoot
  const fullAuditRoot = path.join(root, catalog.auditRoot);
  if (fs.existsSync(fullAuditRoot)) {
    const walk = (dir) => {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        const rel = normalizePath(path.relative(root, full));
        if (entry.isDirectory()) {
          walk(full);
        } else if (entry.name.endsWith('.md')) {
          if (rel === catalog.catalogPath) continue;

          // Check if covered by a cataloged entry report or evidence dir
          if (coveredReports.has(rel)) continue;
          let isCovered = false;
          for (const cDir of coveredDirs) {
            if (rel.startsWith(cDir)) {
              isCovered = true;
              break;
            }
          }
          if (isCovered) continue;

          // Check file content for role annotations or report heading
          try {
            const content = fs.readFileSync(full, 'utf8');
            if (/<!--\s*audit-role:\s*(evidence|fixture)\s*-->/i.test(content)) {
              continue;
            }

            const isReadme = entry.name.toLowerCase() === 'readme.md';
            const hasReportRole = /<!--\s*audit-role:\s*report\s*-->/i.test(content);
            const headingMatch = content.match(/^#+\s*(.+)$/m);
            const hasReportHeading = headingMatch && /audit|report|review|results/i.test(headingMatch[1]);

            if (isReadme || hasReportRole || hasReportHeading) {
              errors.push({
                kind: 'uncatalogedAuditReport',
                detail: `Uncataloged audit report found: ${rel}`,
                path: rel
              });
            }
          } catch {
            // ignore read error
          }
        }
      }
    };
    walk(fullAuditRoot);
  }

  return { errors, warnings };
}

module.exports = {
  readAuditCatalog,
  validateAuditCatalog
};
