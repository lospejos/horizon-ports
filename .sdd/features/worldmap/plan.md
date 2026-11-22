# worldmap: plan (branch worldmap-wasm, folder worldmap/)

- [x] T1: Scaffold mod folder, workflow and branch (done when: CI on branch worldmap-wasm builds worldmap.hmod with a test rectangle on gui_render_post; covers A1, A7)
- [x] T2: Terrain sampler as pure logic: columns around player to RGBA buffer via client_surface_heights + map_color (done when: unit tests pass in CI; covers A2, A6)
- [ ] T3: Draw buffer with mod_image and sprite in a corner, refresh on movement (done when: user sees terrain in game and it follows the player; covers A2, A6)
- [ ] T4: Shading: water from ocean-floor vs surface heights, height difference light/dark (done when: unit tests for shading pass; user sees sea and slopes; covers A3)
- [ ] T5: Settings enabled, corner, size, scale, shape (done when: each changes the live map; round clips via quads or mask; covers A4)
- [ ] T6: Player marker and coordinates text (done when: visible in game; covers A5)
- [ ] T7: README, icon, version 0.1.0 (done when: README has install/update; .hmod holds icon.png; tag only on the user's word; covers A8)
