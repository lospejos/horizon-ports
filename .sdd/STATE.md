# SDD STATE (keep <= 20 lines)
Feature: worldmap step 3, fullscreen map (plan in .sdd/features/fullmap/)
Task: none (T8 done, verified in game)
Next: T9 README + version 0.3.0 (README: M key, cache, disk use; tag only on the user word). Python edit scripts via scratchpad Write tool.
Blockers: none
Open questions: none
Last ADR: 0004
Commit: yes
Context: branch worldmap-wasm is checked out in the main dir. Pushed through abaf385f, CI green. Install a branch build: pwsh worldmap/update-dev.ps1 (it can pick the previous run right after a push; use gh run download <id> -n worldmap-hmod then). Agent can run the game: play instance Horizon, world worldmap-test (creative); mod settings at Options > Mods > Worldmap > Config; reset settings to defaults after tests. SDK clone may exist at %TEMP%\hz\sdk. User rules in memory wasm-ports-handoff.
