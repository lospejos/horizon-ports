# SDD STATE (keep <= 20 lines)
Feature: worldmap step 3, fullscreen map (plan in .sdd/features/fullmap/)
Task: T6 pan/zoom/recentre (code written, ADR 0004 done, spec line added; NOT ticked: CI + in-game check pending)
Next: finish T6: push, wait CI green (last error was f64 vs f32 in zoom, fixed), install build (gh run download <id> -R lospejos/horizon-ports -n worldmap-hmod -D C:/Users/User/AppData/Roaming/horizon/horizon-mods after deleting worldmap.hmod), play Horizon/worldmap-test, check drag, wheel 1/2/4/8, Centre button, game_screenshot; then tick T6 in plan.md and commit. After: T7 waypoints on map. Every key-adding task: Controls check. Tests: write python edit scripts to the scratchpad with the Write tool, not bash heredocs with quotes.
Blockers: none
Open questions: none
Last ADR: 0004
Commit: yes
Context: branch worldmap-wasm is checked out in the main dir. Pushed through abaf385f, CI green. Install a branch build: pwsh worldmap/update-dev.ps1 (it can pick the previous run right after a push; use gh run download <id> -n worldmap-hmod then). Agent can run the game: play instance Horizon, world worldmap-test (creative); mod settings at Options > Mods > Worldmap > Config; reset settings to defaults after tests. SDK clone may exist at %TEMP%\hz\sdk. User rules in memory wasm-ports-handoff.
