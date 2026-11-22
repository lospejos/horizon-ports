# SDD STATE (keep <= 20 lines)
Feature: worldmap step 3, fullscreen map (plan in .sdd/features/fullmap/)
Task: none
Next: T3 explorer scan into the store. Every key-adding task: check Options > Controls for conflicts. gh needs -R lospejos/horizon-ports.
Blockers: none
Open questions: none
Last ADR: 0003
Commit: yes
Context: branch worldmap-wasm lives in worktree .worktrees/worldmap (main dir sits on highlightitem-wasm). Pushed through abaf385f, CI green. Install a branch build: pwsh worldmap/update-dev.ps1 (it can pick the previous run right after a push; use gh run download <id> -n worldmap-hmod then). Agent can run the game: play instance Horizon, world worldmap-test (creative); mod settings at Options > Mods > Worldmap > Config; reset settings to defaults after tests. SDK clone may exist at %TEMP%\hz\sdk. User rules in memory wasm-ports-handoff.
