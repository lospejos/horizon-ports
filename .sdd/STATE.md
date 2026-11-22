# SDD STATE (keep <= 20 lines)
Feature: worldmap (.sdd/features/worldmap/)
Task: none
Next: T7 offsets, T8 info lines, T9 README. Also fix jerky scroll: draw offset by (camera - pass centre) per frame, needs clipping.
Blockers: none
Open questions: none
Last ADR: 0002
Commit: yes
Context: sparse SDK clone at %TEMP%\hz\sdk (host API in crates/horizon-mod-vocab/src/host: client_view.rs, client_render.rs, draw.rs). Workflow template: copy a .github/workflows/<port>-wasm.yml from highlightitem-wasm. User rules in memory wasm-ports-handoff.
