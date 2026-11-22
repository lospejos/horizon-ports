# Installs the latest green worldmap-wasm branch build into the Horizon mods folder (old worldmap*.hmod removed).
$ErrorActionPreference = 'Stop'
$repo = 'lospejos/horizon-ports'
$dir = Join-Path $env:APPDATA 'horizon\horizon-mods'
$id = gh run list -R $repo -b worldmap-wasm -w worldmap-wasm.yml -s success -L 1 --json databaseId --jq '.[0].databaseId'
if (-not $id) { throw 'No successful worldmap-wasm run found' }
Remove-Item "$dir\worldmap*.hmod" -ErrorAction SilentlyContinue
gh run download $id -R $repo -n worldmap-hmod -D $dir
Write-Host "Installed run $id into $dir. Restart the game."
