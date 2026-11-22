# SDD STATE (keep <= 20 lines)
Feature: none
Task: none
Next: run /sdd-new <feature description>. Fullmap (T1-T9) done on branch worldmap-wasm, version 0.3.0, untagged: tag worldmap-wasm-v0.3.0 only on the user word; merge to main is the user call.
Blockers: none
Open questions: none
Last ADR: 0004
Commit: yes
Context: branch worldmap-wasm checked out in the main dir. Install a branch build: pwsh worldmap/update-dev.ps1. Agent can run the game: play instance Horizon, world worldmap-test (creative, in the nether at 200 80 0); mod settings at Options > Mods > Worldmap > Config. Game tools: game_key needs action=tap; game_drag_at points="x1 y1 x2 y2"; game_scroll_at x,y,steps; game_click_button label; game_click_at x,y,right=true. Python edit scripts via the scratchpad Write tool. User rules in memory wasm-ports-handoff.
