# SDD STATE (keep <= 20 lines)
Feature: worldmap (.sdd/features/worldmap/)
Task: T5 (code written, unverified: needs CI + in-game check of each setting)
Next: T5 settings (enabled, corner, size, scale, shape). Also fix jerky scroll: draw offset by (camera - pass centre) per frame, needs clipping; marker is T6.
Blockers: none
Open questions: push worldmap-wasm to run CI for T5? (spec: no push without the user word)
Last ADR: 0002
Commit: yes
Context: sparse SDK clone at %TEMP%\hz\sdk (host API in crates/horizon-mod-vocab/src/host: client_view.rs, client_render.rs, draw.rs). Workflow template: copy a .github/workflows/<port>-wasm.yml from highlightitem-wasm. User rules in memory wasm-ports-handoff.
