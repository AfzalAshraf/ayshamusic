# Aysha Music — Velvet Rose (design proposal)

Refinement of the user's supplied SVG, not an AI redraw of the rejected heart logo.

- `aysha-velvet-rose.svg`: self-contained1024×1024 vector. Text is outlined; no installed fonts, remote assets, scripts or filters required.
- `aysha-velvet-rose.png`:1024×1024 RGB export, rendered from the SVG in Chromium at2× and downsampled for clean edges.
- `Plus-Jakarta-Sans-OFL.txt`: typeface license for the outlined Plus Jakarta Sans glyphs (Google Fonts `ofl/plusjakartasans/PlusJakartaSans[wght].ttf`). MUSIC weight750, Aysha weight600.

Changes: repaired malformed font-weight source; stronger open-counter geometric A; wider/clearer MUSIC crossbar; quieter groove rings; balanced equalizer accents and wordmark spacing; consistent warm rose-gold/plum palette. Decorative detail is intentionally secondary to the A at small sizes. The MUSIC micro-lettering cannot be guaranteed readable at every small launcher size.

**Proposal only.** Packaged Android launcher resources and versionCode20 remain unchanged so the user can approve this direction before another APK is generated. No playback, shuffle/repeat, account/session, theme or dedication changes. A production launcher export must still check adaptive/circle/OEM masks and tiny-size appearance.
