# rebuild_and_fit.ps1 — rebuild the curated human corpus, then re-fit the eval weights.
# Run after collecting more games (your clipboard tool drops them in collected_games/).
# Curates human_games/ = baselines + verified rated collected (GameNr + Elo), deduped by move-content,
# malformed excluded; then dumps the feature CSV and fits with bootstrap CIs.
#
# Usage:  pwsh tools/rebuild_and_fit.ps1     (from the project root)
# Watching the convergence: note the weights + game count each time N grows (32 -> 62 -> 100 ...).

$root = Split-Path -Parent $PSScriptRoot
$hg = Join-Path $root "human_games"
New-Item -ItemType Directory -Force $hg | Out-Null
Get-ChildItem $hg -Filter *.pgn4 | Remove-Item -Force   # rebuild fresh

function MoveSig($file) {
    $m = [regex]::Matches((Get-Content $file.FullName -Raw), '[a-n]\d{1,2}-[a-n]\d{1,2}')
    if ($m.Count -eq 0) { return $null }
    $md5 = [System.Security.Cryptography.MD5]::Create()
    return [System.BitConverter]::ToString($md5.ComputeHash([System.Text.Encoding]::UTF8.GetBytes((($m | ForEach-Object { $_.Value }) -join ','))))
}

# 1) baselines (already verified human)
Copy-Item (Join-Path $root "baselines\*.pgn4") $hg -Force
$sigs = @{}
foreach ($f in Get-ChildItem $hg -Filter *.pgn4) { $s = MoveSig $f; if ($s) { $sigs[$s] = $true } }

# 2) verified rated FFA collected (GameNr + RedElo + Variant FFA), not a dup, not malformed.
#    The [Variant "FFA"] requirement excludes Chaturaji / any other variant that slips into collection.
$added = 0; $mal = 0; $dup = 0; $nonffa = 0
foreach ($f in Get-ChildItem (Join-Path $root "collected_games") -Filter *.pgn4) {
    $t = Get-Content $f.FullName -Raw
    if ($t -notmatch '\[Variant\s+"FFA"') { $nonffa++; continue }   # wrong variant (e.g. Chaturaji)
    if ($t -notmatch '\[RedElo "' -or $t -notmatch '\[GameNr "') { $mal++; continue }
    $s = MoveSig $f
    if (-not $s -or $sigs.ContainsKey($s)) { $dup++; continue }
    Copy-Item $f.FullName $hg; $sigs[$s] = $true; $added++
}
$n = (Get-ChildItem $hg -Filter *.pgn4).Count
Write-Host "human_games/: $n clean rated FFA games  (+$added new; skipped: $nonffa non-FFA, $mal malformed, $dup dups)" -ForegroundColor Cyan

# 3) dump features + fit
$env:CARGO_TARGET_DIR = "C:\rust-target\hornet-flash"
$env:HORNET_HUMAN_ONLY = "1"; $env:HORNET_DUMP_CSV = "1"
cargo run --release --quiet --example texel_tune --manifest-path (Join-Path $root "hornet-engine\Cargo.toml") 2>&1 |
    Select-String "dataset|dumped" | ForEach-Object { $_.Line }
$env:HORNET_HUMAN_ONLY = $null; $env:HORNET_DUMP_CSV = $null

Write-Host "`n--- fit at N=$n games ---" -ForegroundColor Cyan
py -3.11 (Join-Path $PSScriptRoot "fit_weights.py") 200
