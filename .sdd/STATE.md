# SDD STATE (keep <= 20 lines)
Feature: worldmap step 2, waypoints (plan in .sdd/features/worldmap/)
Task: none (T10 done, CI green on 69013383)
Next: T11 create/edit/delete waypoint UI (needs design: key bind, edit screen widgets, how a screen opens). gh needs -R lospejos/horizon-ports (default repo resolves to lavoide).
Blockers: none
Open questions: none
Last ADR: 0002
Commit: yes
Context: branch worldmap-wasm lives in worktree .worktrees/worldmap (main dir sits on highlightitem-wasm). Pushed through abaf385f, CI green. Install a branch build: pwsh worldmap/update-dev.ps1 (it can pick the previous run right after a push; use gh run download <id> -n worldmap-hmod then). Agent can run the game: play instance Horizon, world worldmap-test (creative); mod settings at Options > Mods > Worldmap > Config; reset settings to defaults after tests. SDK clone may exist at %TEMP%\hz\sdk. User rules in memory wasm-ports-handoff.
