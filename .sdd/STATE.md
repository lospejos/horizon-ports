# SDD STATE (keep <= 20 lines)
Feature: none (worldmap step 1 finished, plan in .sdd/features/worldmap/)
Task: none
Next: run /sdd-new <feature description>. Candidates: worldmap jerky-scroll fix (see Follow-ups in worldmap/spec.md), tag worldmap-wasm-v0.1.0 on the user's word, README row in root README.
Blockers: none
Open questions: none
Last ADR: 0002
Commit: yes
Context: branch worldmap-wasm lives in worktree .worktrees/worldmap (main dir sits on highlightitem-wasm). Pushed through abaf385f, CI green. Install a branch build: pwsh worldmap/update-dev.ps1 (it can pick the previous run right after a push; use gh run download <id> -n worldmap-hmod then). Agent can run the game: play instance Horizon, world worldmap-test (creative); mod settings at Options > Mods > Worldmap > Config; reset settings to defaults after tests. SDK clone may exist at %TEMP%\hz\sdk. User rules in memory wasm-ports-handoff.
