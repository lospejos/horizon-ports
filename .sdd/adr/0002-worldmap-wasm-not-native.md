# 0002: worldmap is a wasm mod, not native

Status: accepted
Context: Both reference mods are native (mixins, HudBatch, direct chunk access). A native mod needs --trust-native-code and a per-instance compile; the existing ports in this repo ship as wasm .hmod built by GitHub Actions.
Options: native mod with mixins; wasm mod on the host API only.
Decision: wasm (user's choice, 2026-11-22). SDK 2b77287 gives what a map needs: client_surface_heights, client_far_surface, block_state_map_color, mod_image, gui_render_post, mod_screen_open, project, data_files.
Consequences: No mixins and no engine internals, no own GPU atlas or threads. Host calls are costly, so terrain is sampled in batches under a per-tick budget. mod_image is one image shared by all client mods. Build only through CI (no local cargo).
