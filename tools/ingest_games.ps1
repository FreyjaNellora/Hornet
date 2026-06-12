# Ingest collected chess.com games into the corpus (one command per collection batch).
#
#   powershell -File tools\ingest_games.ps1 [-Source <dir>]
#
# Default source: __pycache__\collected_games (where the collector lands when run with the
# wrong cwd) and collected_games_new if present. For each *.pgn4 in the source:
#   1. RULES FILTER: keep only standard gameplay (FFA + DeadKingWalking + EnPassant +
#      PromoteTo=D; Anonymous/SemiAnonymous privacy flags ignored). Non-standard → skipped+listed.
#   2. DEDUPE by GameNr against collected_games/, human_games/, baselines/.
#   3. RENUMBER continuing from the highest existing cc_game_NNNN.
#   4. COPY into collected_games\ (raw archive) and human_games\ (the tuning corpus).
# Prints a summary. After ingesting, re-run the instruments to re-baseline (see
# experiments/NOTE-behavior-mining.md and the texel/move_match docs).

param(
    [string]$Source = ""
)

$root = Split-Path -Parent $PSScriptRoot
$sources = @()
if ($Source) { $sources += (Join-Path $root $Source) }
else {
    foreach ($cand in "__pycache__\collected_games", "collected_games_new") {
        $p = Join-Path $root $cand
        if (Test-Path $p) { $sources += $p }
    }
}
if (-not $sources) { Write-Host "no source dir found"; exit 1 }

$std = '^\[RuleVariants "(Anonymous )?DeadKingWalking EnPassant PromoteTo=D( SemiAnonymous)?"\]$'
$getnr = {
    param($f)
    (Get-Content $f -TotalCount 8 | Where-Object { $_ -match '^\[GameNr' }) -replace '.*"(\d+)".*', '$1'
}

# Existing GameNrs across all corpus dirs.
$existing = @{}
Get-ChildItem (Join-Path $root "collected_games"), (Join-Path $root "human_games"), (Join-Path $root "baselines") -Filter "*.pgn4" |
    ForEach-Object { $nr = & $getnr $_.FullName; if ($nr) { $existing[$nr] = $true } }

# Next free number.
$maxn = (Get-ChildItem (Join-Path $root "collected_games") -Filter "cc_game_*.pgn4" |
    ForEach-Object { [int]($_.BaseName -replace 'cc_game_', '') } | Measure-Object -Maximum).Maximum
$n = $maxn + 1

$ingested = 0; $dups = 0; $badrules = 0; $nogamenr = 0
foreach ($src in $sources) {
    Write-Host "source: $src"
    foreach ($f in (Get-ChildItem $src -Filter "*.pgn4" | Sort-Object Name)) {
        $nr = & $getnr $f.FullName
        if (-not $nr) { $nogamenr++; continue }
        if ($existing[$nr]) { $dups++; continue }
        $rv = Get-Content $f.FullName -TotalCount 25 | Where-Object { $_ -match '^\[RuleVariants' }
        if (-not ($rv -match $std)) {
            Write-Host "  SKIP non-standard rules: $($f.Name)  $rv"
            $badrules++
            continue
        }
        $name = "cc_game_{0:D4}.pgn4" -f $n
        Copy-Item $f.FullName (Join-Path $root "collected_games\$name")
        Copy-Item $f.FullName (Join-Path $root "human_games\$name")
        $existing[$nr] = $true
        $n++; $ingested++
    }
}

$total = (Get-ChildItem (Join-Path $root "human_games") -Filter "*.pgn4").Count
Write-Host ""
Write-Host "ingested: $ingested   duplicates skipped: $dups   non-standard skipped: $badrules   no-GameNr skipped: $nogamenr"
Write-Host "human_games corpus now: $total games"
Write-Host "next: re-run texel_tune + move_match baselines; commit the new games."
