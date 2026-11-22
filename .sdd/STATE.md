# SDD STATE (keep <= 20 lines)
Feature: worldmap step 2, waypoints (plan in .sdd/features/worldmap/)
Task: none (T12 coded and committed; CI run and in-game check of markers pending)
Next: confirm T12 CI green, install branch build and check markers in game (create waypoint with N, walk away, other dimension), then T13. Every key-adding task: check Options > Controls for conflicts (spec). M reserved for the world map. gh needs -R lospejos/horizon-ports.
Blockers: none
Open questions: none
Last ADR: 0002
Commit: yes
Context: branch worldmap-wasm lives in worktree .worktrees/worldmap (main dir sits on highlightitem-wasm). Pushed through abaf385f, CI green. Install a branch build: pwsh worldmap/update-dev.ps1 (it can pick the previous run right after a push; use gh run download <id> -n worldmap-hmod then). Agent can run the game: play instance Horizon, world worldmap-test (creative); mod settings at Options > Mods > Worldmap > Config; reset settings to defaults after tests. SDK clone may exist at %TEMP%\hz\sdk. User rules in memory wasm-ports-handoff.
