# Visual and interaction testing

Linura intends to be beautiful, but visual quality must be testable rather than maintained only by screenshots in pull requests.

`design/tokens.json` defines the initial semantic design-token vocabulary. `visual/baselines/manifest.json` reserves reviewed baselines for first boot, Control Center, and approval surfaces.

`tools/visual.py` uses ImageMagick for pixel-difference evidence once baselines exist. A `null` baseline explicitly means "not yet qualified" and must not count as a visual pass.

Visual testing should cover representative resolutions, scaling, light/dark or theme variants, focus/keyboard navigation, destructive approvals, offline/error states, drift/recovery states, and reduced-motion/accessibility behavior.
