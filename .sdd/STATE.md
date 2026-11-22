# SDD STATE (keep <= 20 lines)
Feature: worldmap (.sdd/features/worldmap/)
Task: none
Next: T2 terrain sampler (pure logic + unit tests; CI must run cargo test, add a step)
Blockers: none
Open questions: MOD_IMAGE_SIZE not read yet (check in T3); in-game check of the T1 test rectangle by the user (not done)
Last ADR: 0002
Commit: yes
Context: sparse SDK clone at %TEMP%\hz\sdk (host API in crates/horizon-mod-vocab/src/host: client_view.rs, client_render.rs, draw.rs). Workflow template: copy a .github/workflows/<port>-wasm.yml from highlightitem-wasm. User rules in memory wasm-ports-handoff.
