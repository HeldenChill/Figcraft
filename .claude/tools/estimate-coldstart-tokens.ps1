# estimate-coldstart-tokens.ps1
# Estimates Claude cold-start token cost for a project's AI config.
# Counts characters in each loaded file and divides by 4 (standard English approximation).
# Run from the project root, or pass -ProjectRoot explicitly.

param(
    [string]$ProjectRoot = ""
)
if (-not $ProjectRoot) {
    $ProjectRoot = (Get-Item $PSScriptRoot).Parent.Parent.FullName
}

$CHARS_PER_TOKEN = 4

function Get-TokenEstimate([string]$text) {
    return [math]::Ceiling($text.Length / $CHARS_PER_TOKEN)
}

function Read-FileTokens([string]$path, [string]$label) {
    if (-not (Test-Path $path)) {
        return [PSCustomObject]@{ Label = $label; Chars = 0; Tokens = 0; Note = "MISSING" }
    }
    $content = Get-Content $path -Raw -Encoding UTF8
    $chars   = $content.Length
    $tokens  = Get-TokenEstimate $content
    return [PSCustomObject]@{ Label = $label; Chars = $chars; Tokens = $tokens; Note = "" }
}

# Derive the Claude auto-memory project slug from the project root path.
# Claude Code format: lowercase drive letter + "--" + rest of path with "\" -> "-", casing preserved.
# e.g. "G:\Unity\Project\Foo" -> "g--Unity-Project-Foo"
function Get-MemorySlug([string]$path) {
    # Normalise to backslashes, strip trailing separator
    $path = $path.TrimEnd('\', '/')
    # Split on first backslash (after drive letter + colon)
    if ($path -match '^([A-Za-z]):\\(.+)$') {
        $drive    = $Matches[1].ToLower()
        $rest     = $Matches[2] -replace '[/\\]+', '-'   # inner separators -> single hyphen
        return "$drive--$rest"
    }
    # Fallback: just replace separators
    return ($path -replace '[/\\]+', '-').TrimStart('-')
}

$rows = @()

# --- CLAUDE.md (always loaded) ---
$rows += Read-FileTokens "$ProjectRoot\CLAUDE.md" "CLAUDE.md"

# --- Rules (all .md files in .claude/rules/) ---
$rulesDir = "$ProjectRoot\.claude\rules"
if (Test-Path $rulesDir) {
    Get-ChildItem "$rulesDir\*.md" | Sort-Object Name | ForEach-Object {
        $rows += Read-FileTokens $_.FullName "rule: $($_.Name)"
    }
}

# --- Hooks (loaded on every UserPromptSubmit) ---
$hooksDir = "$ProjectRoot\.claude\hooks"
if (Test-Path $hooksDir) {
    Get-ChildItem "$hooksDir\*" | Sort-Object Name | ForEach-Object {
        $rows += Read-FileTokens $_.FullName "hook: $($_.Name)"
    }
}

# --- Auto-memory MEMORY.md index (injected by hook) ---
# Path is derived from the project root — no hardcoded project name.
$slug        = Get-MemorySlug $ProjectRoot
$memoryIndex = "$env:USERPROFILE\.claude\projects\$slug\memory\MEMORY.md"
$rows += Read-FileTokens $memoryIndex "auto-memory: MEMORY.md"

# --- Print table ---
$sep = "-" * 80
Write-Host ""
Write-Host "  CLAUDE COLD-START TOKEN ESTIMATE" -ForegroundColor Cyan
Write-Host "  Project: $ProjectRoot" -ForegroundColor DarkGray
Write-Host "  Formula: chars / $CHARS_PER_TOKEN  (English prose approximation; code ~10-20% higher)" -ForegroundColor DarkGray
Write-Host ""
Write-Host ("  {0,-45} {1,8} {2,8}  {3}" -f "File", "Chars", "Tokens", "Note")
Write-Host "  $sep"

$totalChars  = 0
$totalTokens = 0

foreach ($r in $rows) {
    $color = if ($r.Note -eq "MISSING") { "DarkYellow" } elseif ($r.Tokens -gt 2000) { "Yellow" } else { "White" }
    Write-Host ("  {0,-45} {1,8} {2,8}  {3}" -f $r.Label, $r.Chars, $r.Tokens, $r.Note) -ForegroundColor $color
    $totalChars  += $r.Chars
    $totalTokens += $r.Tokens
}

Write-Host "  $sep"
Write-Host ("  {0,-45} {1,8} {2,8}" -f "TOTAL (one cold-start)", $totalChars, $totalTokens) -ForegroundColor Cyan

# --- Subagent scenario estimates ---
Write-Host ""
Write-Host "  SUBAGENT SCENARIO ESTIMATES" -ForegroundColor Cyan
Write-Host "  (cold-start cost x N, before any task-specific context is added)" -ForegroundColor DarkGray
Write-Host ""

$scenarios = @(
    @{ Label = "1 layer agent (direct dispatch)";             Multiplier = 1  },
    @{ Label = "subagent-driven-dev, 1 task (impl+2 review)"; Multiplier = 3  },
    @{ Label = "subagent-driven-dev, 3 tasks";                Multiplier = 9  },
    @{ Label = "subagent-driven-dev, 5 tasks";                Multiplier = 15 }
)

foreach ($s in $scenarios) {
    $est = $totalTokens * $s.Multiplier
    Write-Host ("  {0,-50} ~{1,8} tokens" -f $s.Label, $est)
}

Write-Host ""
Write-Host "  FILES OVER 1000 TOKENS (candidates for slimming):" -ForegroundColor Yellow
$heavy = $rows | Where-Object { $_.Tokens -gt 1000 } | Sort-Object Tokens -Descending
if ($heavy) {
    foreach ($r in $heavy) {
        Write-Host ("    {0,-45} {1,6} tokens" -f $r.Label, $r.Tokens) -ForegroundColor Yellow
    }
} else {
    Write-Host "    (none)" -ForegroundColor DarkGray
}
Write-Host ""
