# SDD STATE (keep <= 20 lines)
Feature: worldmap (.sdd/features/worldmap/)
Task: T4 (code written, unverified: needs CI + in-game check)
Next: T4 shading (water from far-surface `water`, slope light/dark). Also fix jerky scroll: draw offset by (camera - pass centre) per frame, needs clipping; marker is T6.
Blockers: none
Open questions: push worldmap-wasm to run CI for T4? (spec: no push without the user word)
Last ADR: 0002
Commit: yes
Context: sparse SDK clone at %TEMP%\hz\sdk (host API in crates/horizon-mod-vocab/src/host: client_view.rs, client_render.rs, draw.rs). Workflow template: copy a .github/workflows/<port>-wasm.yml from highlightitem-wasm. User rules in memory wasm-ports-handoff.
