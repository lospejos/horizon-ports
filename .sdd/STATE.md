# SDD STATE (keep <= 20 lines)
Feature: worldmap (.sdd/features/worldmap/)
Task: T1
Next: T1 scaffold (branch worldmap-wasm, folder worldmap/); see plan.md
Blockers: none
Open questions: T1 files written, NOT committed or pushed; waiting for user ok to commit+push (CI build is the done-when check)
Last ADR: 0002
Commit: yes
Context: sparse SDK clone at %TEMP%\hz\sdk (host API in crates/horizon-mod-vocab/src/host: client_view.rs, client_render.rs, draw.rs). Workflow template: copy a .github/workflows/<port>-wasm.yml from highlightitem-wasm. User rules in memory wasm-ports-handoff.
