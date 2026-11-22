# SDD STATE (keep <= 20 lines)
Feature: worldmap step 2, waypoints (plan in .sdd/features/worldmap/)
Task: T10 code written and pushed, waiting for CI (tick when green)
Next: T11 after T10 CI green. Old next: run /sdd-new <feature description>. Candidates: worldmap step 2 waypoints (T10-T15 in plan, A11-A14 in spec), worldmap jerky-scroll fix (see Follow-ups in worldmap/spec.md), tag worldmap-wasm-v0.1.0 on the user's word, README row in root README.
Blockers: none
Open questions: none
Last ADR: 0002
Commit: yes
Context: branch worldmap-wasm lives in worktree .worktrees/worldmap (main dir sits on highlightitem-wasm). Pushed through abaf385f, CI green. Install a branch build: pwsh worldmap/update-dev.ps1 (it can pick the previous run right after a push; use gh run download <id> -n worldmap-hmod then). Agent can run the game: play instance Horizon, world worldmap-test (creative); mod settings at Options > Mods > Worldmap > Config; reset settings to defaults after tests. SDK clone may exist at %TEMP%\hz\sdk. User rules in memory wasm-ports-handoff.
