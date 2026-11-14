---
name: local-player-iconography
description: Design, integrate and verify consistent, accessible SVG icons for every Aysha Music Player control without CDN dependencies.
---

# Local player icon skill

Owner: UI/UX Design & Frontend. QA may use the verification checklist while fixing broken controls. This skill is a repository procedure, not a claim of an automatically installed platform capability.

## Source of truth

- Artwork: src/icons.svg, embedded in the Rust executable and served as /icons.svg.
- Served page: src/index.html. Do not silently update only src/www/index.html; it is not the Rust root page.
- Rendering: static SVG use references and the page's icon(name) helper.
- Tests: tests/test_icons.py and tests/frontend.test.cjs.
- Icons are original project-owned geometric drawings; no downloaded asset licenses or runtime icon packages are required.

## Semantic inventory

| Action/location | Symbol | Required accessible meaning |
| --- | --- | --- |
| Brand / favorites | heart | Brand text / Like song with aria-pressed |
| Home | home | Home |
| Explore | compass | Explore |
| Library | library | Library |
| Bluetooth | bluetooth | Bluetooth |
| Search | search | Search input has its own label |
| Play | play | Play |
| Pause | pause | Pause |
| Stop | stop | Stop |
| Previous track | previous | Previous track, never rewind |
| Next track | next | Next track, never fast-forward |
| Volume high | volume-high | Mute control plus labelled volume slider |
| Volume low | volume-low | Same control, current level reflected visually |
| Muted | volume-off | Unmute control |
| Add to queue | queue | Add to Queue |
| Already queued | check | Already queued |

## Integration procedure

1. Identify the actual action and its states before choosing a symbol. An icon cannot substitute for implementing the action.
2. Reuse the inventory before creating a new symbol. Use a 24 by 24 viewBox, 1.8-unit currentColor stroke, rounded joins/caps and consistent optical size.
3. Use local SVG use references; never use emoji, icon-font code points, remote SVG scripts or a runtime CDN for core controls.
4. Keep all interactive controls native buttons or inputs. Add accessible labels, visible keyboard focus, and aria-pressed for toggle state. Decorative SVGs are aria-hidden and not focusable.
5. Keep play/pause symbol, accessible label and actual playback state synchronized. Failed playback must not leave a Pause icon on a stopped player.
6. Bind track actions with event listeners or closures, never serialized JSON inside HTML event attributes.
7. Do not insert user-provided names into icon markup. The icon name must come from this fixed internal inventory.
8. If a new third-party icon source is requested, verify its license, record attribution and vendor only the needed sanitized SVGs. Ask about ambiguous semantics rather than guessing.
9. Check at mobile and desktop sizes, with keyboard navigation and light/dark contrast. Aim for 44px touch targets; document cramped controls instead of claiming accessibility without testing.
10. Keep symbols embedded so moving only the Rust binary does not break icons. Serve the correct image/svg+xml content type without immutable caching of an unversioned URL.

## Acceptance checks

```bash
python3 -m unittest discover -s tests -p 'test_*.py' -v
node --test tests/frontend.test.cjs
cargo test
cargo build --release
```

Manually inspect every inventory item, toggle state and focus state in the target browser. Confirm icons still load with external icon websites blocked. Passing XML/reference tests does not establish visual polish or screen-reader usability; do not award 5/5 without manual evidence.
