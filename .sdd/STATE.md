# SDD STATE (keep <= 20 lines)
Feature: worldmap (.sdd/features/worldmap/)
Task: T3 (code written, CI + in-game check pending)
Next: T3 draw buffer with mod_image (read MOD_IMAGE_SIZE; wire Scan + far_surface + map colour)
Blockers: none
Open questions: sprite name "horizon/modimage" unverified in game; T1 rectangle check superseded
Last ADR: 0002
Commit: yes
Context: sparse SDK clone at %TEMP%\hz\sdk (host API in crates/horizon-mod-vocab/src/host: client_view.rs, client_render.rs, draw.rs). Workflow template: copy a .github/workflows/<port>-wasm.yml from highlightitem-wasm. User rules in memory wasm-ports-handoff.
