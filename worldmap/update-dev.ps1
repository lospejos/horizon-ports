# Installs a worldmap build into the Horizon mods folder (old worldmap*.hmod removed).
# No argument: the latest successful build of any branch or tag. -Tag <tag>: the release asset of that tag.
param([string]$Tag)
$ErrorActionPreference = 'Stop'
$repo = 'lospejos/horizon-ports'
$dir = Join-Path $env:APPDATA 'horizon\horizon-mods'
if ($Tag) {
    $tmp = Join-Path ([IO.Path]::GetTempPath()) "worldmap-$Tag"
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    gh release download $Tag -R $repo -p worldmap.hmod -D $tmp
    Remove-Item "$dir\worldmap*.hmod" -ErrorAction SilentlyContinue
    Move-Item "$tmp\worldmap.hmod" $dir
    Write-Host "Installed release $Tag into $dir. Restart the game."
    return
}
$run = gh run list -R $repo -w worldmap-wasm.yml -s success -L 1 --json databaseId,headBranch --jq '.[0] | "\(.databaseId) \(.headBranch)"'
if (-not $run) { throw 'No successful worldmap-wasm run found' }
$id, $ref = $run -split ' '
Remove-Item "$dir\worldmap*.hmod" -ErrorAction SilentlyContinue
gh run download $id -R $repo -n worldmap-hmod -D $dir
Write-Host "Installed run $id ($ref) into $dir. Restart the game."
