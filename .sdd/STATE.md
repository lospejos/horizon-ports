# SDD STATE (keep <= 20 lines)
Feature: worldmap step 3, fullscreen map (plan in .sdd/features/fullmap/)
Task: T8 cursor info + teleport (code pushed, CI + in-game check pending)
Next: T8 cursor info and teleport on map (X Z Y under cursor, click then Teleport via /tp @s, same dimension only). Every key-adding task: Controls check. Python edit scripts via scratchpad Write tool. Game test: game_key needs action=tap; game_drag_at points="x1 y1 x2 y2"; game_scroll_at x,y,steps; game_click_button label; game_click_at x,y,right=true.
Blockers: none
Open questions: none
Last ADR: 0004
Commit: yes
Context: branch worldmap-wasm is checked out in the main dir. Pushed through abaf385f, CI green. Install a branch build: pwsh worldmap/update-dev.ps1 (it can pick the previous run right after a push; use gh run download <id> -n worldmap-hmod then). Agent can run the game: play instance Horizon, world worldmap-test (creative); mod settings at Options > Mods > Worldmap > Config; reset settings to defaults after tests. SDK clone may exist at %TEMP%\hz\sdk. User rules in memory wasm-ports-handoff.
