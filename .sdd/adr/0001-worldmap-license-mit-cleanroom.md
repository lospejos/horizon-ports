# 0001: worldmap is MIT and written clean-room

Status: accepted
Context: The new map mod combines ideas from Eternity World Map (MIT) and Minimap-Worldmap-and-waypoints-reworked (GPL-3.0). A licence had to be chosen before code is written.
Options: MIT with own code; GPL-3.0 with code ported from the GPL mod.
Decision: MIT (user's choice, 2026-11-22). The code is written from scratch on the Horizon wasm API. Eternity code may be reused with its notice kept in NOTICE. Nothing is copied from the GPL mod; only its public feature list (README) is used as inspiration, and its source was not read.
Consequences: Any GPL code, even a snippet, would force a licence change, so it must not enter the repo. Features are re-implemented, not ported.
