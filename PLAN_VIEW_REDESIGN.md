# Plan View Redesign — Implementation Plan

Goal: rebuild the Plan view with the same Apple-HIG glass overlay language the Fly view already uses.
Interactive reference mock (canonical for look, spacing, and behavior):
https://claude.ai/code/artifact/0987ee56-bb62-4b6d-ac38-1308f2d394f5

## Status — phases 1–9 built and verified in the running app

Done: OverlaySegmentedControl, OverlayPopover, PlanGroupCard, PlanGroupRow (new components);
floating glass inspector with segmented layer control; glass dock with File/Waypoint/Add/Center/Map
type; floating toolbar with document capsule, one grouped stats capsule that pages, and the accent
Upload button carrying its own progress; mission items as one grouped list with numbered seals and
inline editing; drill-in detail page for complex items with a back header and a red delete row;
dock popover menus replacing all four drop panels; terrain profile as a glass sheet with a grabber;
map/satellite/hybrid checkmark menu; one accent — route, markers and profile line all system blue.

Verified with `tools/run-app.sh` + `tools/ui-probe.py` against a mock ArduPilot vehicle: no QML
runtime errors, all three layers switch, Add → Takeoff → waypoints works, Survey drills in, the
map-type menu checkmarks the current type, and the Fly view is unaffected.

Phase 10 is done too: `GeoFenceEditor.qml` was rebuilt as grouped cards — an empty state, an
"Add fence" card, one row per polygon/circle carrying its inclusion state, a radius field, a
pencil edit toggle and a red remove badge, plus settings and breach-return sections. The rally
panel became a grouped list of *all* points (it used to show only the current one under a
description card), each row led by a green seal and expanding inline to its facts with a red
delete row; `RallyPointEditorHeader.qml` and `RallyPointItemEditor.qml` were deleted. Rally
markers on the map are green to match their seals, since green means "somewhere the vehicle can
come down" and the accent belongs to the mission it is asked to fly.

Not done, still open (see also the backlog at the end):
- Phase 8 map furniture: per-leg distance labels on the map, a split handle at *every* leg midpoint
  (only QGC's single split-segment indicator exists), leg direction arrowheads are the stock
  `MapLineArrow` rather than the mock's mid-leg triangles.
- Upload's "blocked — waiting on terrain data" dim state: the button dims for offline / empty /
  syncing only. Terrain-block still surfaces through the existing dialog rather than a disabled
  button with a reason, because `readyForSaveState()` is a non-notifying invokable.
- The complex-item editors themselves (Survey, Corridor, Structure, landing patterns) render
  inside a glass card but keep their own internal section/grid layout.

Everything below is the original phase plan, kept as the spec these were built against.

Work through phases **in order**. Each phase is independently shippable and must build and run
before starting the next. Do not start a phase until the previous phase's acceptance checks pass.

## Ground rules

- Reuse the existing overlay rig in `src/QmlControls/`: `OverlayGlass.qml`, `OverlayCapsule.qml`,
  `OverlayPill.qml`, `OverlayRoundButton.qml`, `OverlayMenuItem.qml`, `OverlayMenuSeparator.qml`,
  `OverlayEditBadge.qml`, `OverlayShadowEffect.qml`, `OverlayBackdrop.qml`. Look at how
  `src/FlightDisplay/CameraControlLayer.qml` and `src/FlightDisplay/FlyView.qml` use them before
  writing anything new.
- All colors come from `QGroundControl.globalPalette` (`qgcPal`) tokens and the overlay components'
  own `contentColor`. Never hardcode hex values in Plan view QML.
- All sizes in `ScreenTools.defaultFontPixelHeight` / `defaultFontPixelWidth` multiples, matching the
  overlay components' existing conventions.
- Semantic color rules (enforce everywhere in this view):
  - accent (system blue): mission path, waypoint seals, survey transects, primary action
  - green: launch/takeoff, terrain ground line, upload success
  - orange: geofence boundary only
  - red: live vehicle marker and terrain collision only — nothing else
- Do not remove or break Fence, Rally, or UTM-SP functionality. UTM-SP adds a fourth layer segment
  when `_utmspEnabled` is true (see `src/QmlControls/PlanView.qml` around line 83).
- New QML files must be added to `src/QmlControls/CMakeLists.txt` like the existing Overlay files.
- After editing QML, run with `QML_DISABLE_DISK_CACHE=1` or delete the qmlcache — the app can hang
  in the QML disk-cache alias resolver after QML edits.
- No code comments. Match the surrounding code's naming and style.
- Commit after each phase, message `feat(plan): <phase title>`.

## Key existing files

- `src/QmlControls/PlanView.qml` (~1250 lines) — the whole view: map (`editorMap`, ~line 356),
  tool strip (`toolStrip`, ~line 575), right panel (`rightPanel`, ~line 654), layer tab bar
  (`layerSelector` QGCTabBar, ~line 683), mission item editor host (~line 722), terrain profile
  (`_terrainProfileOpen` / `_terrainProfileHeight`, ~lines 39–42).
- `src/QmlControls/MissionItemEditor.qml` (~355 lines) — per-item row + editor host.
- `src/QmlControls/PlanToolBarIndicators.qml` (~191 lines) — toolbar stats.
- `src/QmlControls/DropPanel.qml` — current tool strip drop panels.
- `src/PlanView/` — mission controllers and complex item editors (Survey etc.). Do not modify
  controllers in phases 1–8; UI only.

---

## Phase 1 — OverlaySegmentedControl

New file `src/QmlControls/OverlaySegmentedControl.qml`.

Spec:
- Properties: `model` (list of strings), `currentIndex` (int, two-way), signal `activated(int index)`.
- Visual: a rounded track (`radius: height / 2` is wrong here — use `radius: ScreenTools.defaultFontPixelHeight * 0.45`)
  filled with `qgcPal.overlayInset` or `Qt.rgba(0,0,0,0.32)` if no such token exists; inner padding
  `ScreenTools.defaultFontPixelHeight * 0.11`.
- Segments: equal width (`Row` of items, each `width: (parent.width - padding*2) / model.length`),
  centered label, `font.pointSize: ScreenTools.defaultFontPointSize`, inactive color `qgcPal.colorGrey`-like
  secondary, active color primary text.
- Active segment: rounded rect behind the label, `Qt.rgba(1,1,1,0.14)` over a subtle top-light
  gradient, small drop shadow. Animate its `x` with a 150 ms easing when `currentIndex` changes.
- Height: `ScreenTools.defaultFontPixelHeight * 2.1`.

Acceptance: component renders standalone with 3 and 4 segments, keyboard/mouse switching works,
active pill slides.

## Phase 2 — Floating inspector shell

File: `src/QmlControls/PlanView.qml`, `rightPanel` block (~line 654).

- Replace the hard-edged full-height rectangle with an `OverlayCapsule`-style glass panel: a
  `Rectangle` wrapping `OverlayGlass` (copy the pattern from `OverlayCapsule.qml` but with
  `radius: ScreenTools.defaultFontPixelHeight * 1.3` and full-height geometry), inset
  `ScreenTools.defaultFontPixelHeight * 1.1` from top, right, and bottom edges.
- Keep `_rightPanelWidth` logic; the map must render behind the panel (it already fills the view).
- Replace `layerSelector` (QGCTabBar + QGCTabButton, ~line 683) with `OverlaySegmentedControl`,
  model `["Mission", "Fence", "Rally"]` plus `"UTM-SP"` when `_utmspEnabled`. Wire to the existing
  `_editingLayer` / layer index property so all existing show/hide bindings keep working.
- Panel content padding: `ScreenTools.defaultFontPixelHeight * 0.9` all around, column spacing
  `ScreenTools.defaultFontPixelHeight * 0.8`.

Acceptance: map visible behind a rounded floating panel; Mission/Fence/Rally (and UTM-SP when
enabled) all still switch and function; nothing clipped at small window sizes.

## Phase 3 — Glass dock (tool strip)

File: `src/QmlControls/PlanView.qml`, `toolStrip` (~line 575).

- Replace `ToolStrip` usage with a vertical glass dock: a rounded `OverlayGlass` container
  (`radius: width / 2` capsule), anchored `left` with margin `ScreenTools.defaultFontPixelHeight * 1.1`,
  `verticalCenter` of the map area.
- One `OverlayRoundButton` per existing action, same order: File, (separator), Waypoint, Add,
  (separator), Center. Separators: 1 px hairline `Qt.rgba(1,1,1,0.09)`, width ~65 % of button.
- Active/checked state: tinted accent fill exactly as `CameraControlLayer.qml` does it.
- Keep the existing `dropPanelComponent` behavior for now (File/Add/Center still open the old
  DropPanel). Menu conversion is Phase 7.
- Add a fifth button "Map type" at the bottom (icon: stacked layers). It does nothing until Phase 8.

Acceptance: all four existing actions work identically; active add-waypoint mode shows tinted fill.

## Phase 4 — Toolbar consolidation

Files: `src/QmlControls/PlanToolBarIndicators.qml`, the plan toolbar block in `PlanView.qml`
(`planToolBar`, ~line 344).

- Leading: glass "‹ Exit" capsule (existing exit action).
- Next: document capsule — plan filename (no extension) on top, subtitle
  `"<Edited|Uploaded> · N items"` below (`missionController.dirty` / item count; "Uploaded" state
  wiring completes in Phase 9 — until then show "Edited" whenever dirty, filename subtitle otherwise).
- Center: ONE grouped stats capsule with hairline dividers, four slots: Distance, Time, Max telem,
  Alt diff. Values in monospaced figures (`font.family: ScreenTools.fixedFontFamily` if available,
  else `Qt.font({family: "Menlo"})` is forbidden — use the ScreenTools fixed font). Clicking the
  capsule swaps to a second page: Azimuth, Prev WP, (blank), (blank). Click again to swap back.
- Trailing: Upload as the single filled-accent capsule button (`qgcPal.primaryButton`-style but
  accent-filled). Every other toolbar element is glass/neutral.
- Delete the now-unused individual stat pill code.

Acceptance: all six original stats reachable (four visible + two on page two), Upload triggers the
same upload action as before, layout holds at 1100 px window width.

## Phase 5 — Mission Start + item list restyle (simple items)

Files: `PlanView.qml` mission panel content, `MissionItemEditor.qml`.

Mission Start card (inset grouped card: `Rectangle` `Qt.rgba(1,1,1,0.055)`,
`radius: ScreenTools.defaultFontPixelHeight * 0.9`):
- Row 1: "Initial altitude" label + value field.
- Row 2: "Altitude mode" label + tappable value that opens the altitude-mode options (reuse the
  existing altitude mode control/dialog; presentation becomes a menu in Phase 7).
- Row 3: "Vehicle & launch position" disclosure row (chevron) that expands/collapses the existing
  Vehicle Info and Launch Position sections. Collapsed by default.

Items list (each `MissionItemEditor` row):
- Leading numbered seal: circle `ScreenTools.defaultFontPixelHeight * 1.5` diameter, accent fill,
  white bold sequence number; green for Takeoff/launch; rounded-square (radius 30 %) for complex
  items (Survey, Corridor, Structure, Landing).
- Label: command name. Trailing: `"<alt> m · <leg dist> m"` in fixed-width figures, then chevron
  only for complex items.
- Rows separated by 1 px hairlines inside one grouped card; no per-row borders/frames.
- Selected simple item: row tinted `Qt.rgba(accent, 0.16)`, and the editor expands INLINE directly
  under the row inside the same card (keep using the item's existing editor component, restyled
  container only). No separate floating editor.
- Last row in card: "＋ Add waypoint" centered accent text row, triggers existing insert logic
  (insert after selected item).

Acceptance: select/edit/insert all work for simple waypoints; values update live; Takeoff shows
green seal; visual matches the mock's grouped list.

## Phase 6 — Drill-in for complex items

Files: `PlanView.qml` mission panel, complex editors under `src/PlanView/`.

- Selecting a complex item (Survey, Corridor Scan, Structure Scan, Landing) does NOT expand inline.
  Instead the panel content is replaced by a detail page: header row `"‹ Items"` back button
  (accent text) + centered item title, then the item's existing editor QML hosted below, restyled
  into grouped cards where cheap (do not rewrite editor internals).
- Slide transition 180 ms (x-offset + opacity) both directions.
- A destructive "Delete <Item>" red centered row at the bottom of the detail page, wired to the
  existing delete action.
- Back returns to the list with the item still selected.

Acceptance: Survey fully editable via drill-in; back/forward navigation clean; delete works;
simple items still expand inline.

## Phase 7 — Dock popover menus

Files: `PlanView.qml`, reuse `OverlayMenuItem.qml` + `OverlayMenuSeparator.qml` (see FlyView.qml
~line 412 for the existing popover pattern).

- Replace the File, Add, and Center DropPanels with glass popovers anchored to the right of their
  dock buttons. Items map 1:1 to the existing DropPanel actions:
  - File: Open…, Save, Save As…, Export KML…, separator, Download from Vehicle, separator,
    Clear Mission (red, last).
  - Add: the existing pattern/complex-item choices.
  - Center: Mission, All Items, Launch, Vehicle (existing options).
- Altitude mode row (Phase 5) now opens the same popover style with a leading checkmark on the
  current mode.
- Popover dismisses on outside click and Esc.

Acceptance: every action reachable from the old DropPanels is reachable from the menus; destructive
item styled red and last.

## Phase 8 — Map furniture

Files: `PlanView.qml` map layer, mission line/waypoint delegates under `src/FlightMap/` used by the
Plan view.

- Route: single accent-colored polyline with a white ~35 % opacity casing under it (two stacked
  MapPolylines). Remove the multi-color leg styling.
- Waypoint markers: match the seals from Phase 5 (accent circle, white number, white ring;
  selected = larger with soft accent halo; launch green).
- Leg distance labels: small dark pill offset perpendicular from each leg midpoint, hidden for
  legs shorter than ~25 m on screen.
- Split handles: the existing split-segment affordance becomes a small translucent circle at every
  leg midpoint; hover/press reveals ＋; activating runs the existing split logic.
- Direction arrowheads: small white triangle at ~55 % along each leg, rotated to leg heading.
- Map type: the Phase 3 dock button opens a checkmark menu "Map / Satellite" wired to the existing
  map provider/type setting (`QGCMapEngine` map type). Do not add a third option.
- Terrain profile: keep the existing plot, but host it in a floating glass sheet (inset from dock
  and inspector, rounded, grabber bar at top). Tapping the grabber collapses to a header-only bar;
  state persists in `_planViewSettings`.

Acceptance: route legible on satellite and map types; split works from every leg; profile collapses
and restores; no color other than the semantic set appears on the map layer.

## Phase 9 — Upload lifecycle

Files: `PlanView.qml` toolbar, `PlanMasterController` signals (read-only — UI wiring only).

Upload button states driven by existing controller state:
- dirty + syncable: accent "Upload"
- uploading: in-button progress fill (white 28 % overlay width bound to progress), label "Uploading…"
- success: green fill, "✓ Uploaded", document capsule subtitle "Uploaded · N items"; reverts to
  dirty state on any plan edit
- blocked (waiting on terrain data / no vehicle): dimmed neutral fill, disabled, tooltip with the
  reason (reuse the exact message strings already in `PlanView.qml` ~line 196).

Acceptance: all four states reachable against a real vehicle or SITL; no dialogs replaced yet —
existing dialogs still fire where they do today.

## Phase 10 — Fence & Rally polish

- Fence and Rally geometry render whenever defined: dimmed (30 % opacity) while another layer is
  active, full strength when their layer is active. Fence boundary orange dashed; rally points as
  green flag seals.
- Fence panel: grouped rows (polygon summary, max altitude, breach action) + "＋ Add fence" row.
- Rally panel: list rows (name/coords/alt) + "＋ Add rally point" row.
- Empty states for both: centered icon, one-line purpose text, one tinted action button.

Acceptance: editing mission with a fence defined shows the dimmed boundary; all existing
fence/rally editing still works.

## Backlog (do NOT attempt without a separate plan)

- Undo stack on MissionController (C++), ⌘Z + toast, removal of confirmation dialogs
- Document autosave (drop Save/Save As, File menu becomes Duplicate/Rename/Move To)
- Empty-plan coaching state (tap-to-place-takeoff flow)
- List edit mode (red delete badges via `OverlayEditBadge`, drag reorder)
- Camera-action chip on waypoint rows; ROI map visualization (hollow seal + dashed sight lines)
- Terrain collision red profile segment + clearance callout
- Compact-width bottom-sheet inspector layout
- Command-type change via menu on the expanded row

## Verification per phase

1. Build the macOS app and launch with `QML_DISABLE_DISK_CACHE=1`.
2. Open Plan view, run through the phase's acceptance list manually.
3. Check both with and without a connected vehicle, and with `_utmspEnabled` off (default).
4. Take a screenshot and compare against the reference artifact before committing.
