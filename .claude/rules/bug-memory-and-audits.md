---
description: Bug records use an indexed domain memory collection; audit reports use a project-owned audit catalog; shared sync delivers policy and tools only
---
# Rule: Bug Memory and Audit Organization

Default bug hub: .cursor/memory/bugs/README.md.
Default audit catalog: .cursor/audits/README.md.
Project values may override paths. Resolve every path relative to the active project.

Register the bug hub once in the canonical memory index. Its Domain, File, Scope table registers domain memories. Search the whole collection before filing; one authoritative record per ID. Status changes do not move files. Keep terminal records and identity history permanently.

Follow the installed Framework lifecycle rule when available. Preserve its ten-column schema and status markers. Organization does not replace lifecycle verification.

Catalog standalone audit entry reports using Audit, Scope, Date, Report, Evidence folder, Outcome. Raw sessions and fixtures remain under their group entry. Preserve historical outcomes and link bugs to evidence. Store new audits under category/date-topic paths inside the configured audit root.

Shared sync owns rules and tools. Project bug data, catalogs, history, audit evidence and project values are never synchronized from master. Legacy migrations require a read-only inventory, complete reconciliation, conservation proof, and scoped cutover. A pull must not perform them automatically.

After bug edits, run the installed ledger validator. After bug hub, catalog or report edits, run the shared index checker. Tools without Claude edit hooks run these checks explicitly.
