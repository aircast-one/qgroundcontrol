/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl
import QGroundControl.ScreenTools

// Collision rig for the floating overlay items. Movable items (telemetry chips, the compass)
// register together with their DragToPosition; fixed chrome (camera cluster, tool strip, pip)
// registers as static obstacles. When a drag ends, overlaps resolve by pushing movables apart
// along the smallest translation vector - the dragged item wins, statics never move. Items
// animate to their pushed positions through their own Behaviors, which supplies the bounce.
QtObject {
    id: root

    property Item viewport
    property real topInset: 0
    property bool editMode: false

    readonly property int  _maxResolvePasses: 10
    readonly property int  _settleMSecs:      400
    readonly property real _positionEpsilon:  0.5

    // A reflow is queued but has not run yet. Anything that needs to observe a settled
    // layout waits on this rather than on the timer that happens to implement the delay.
    readonly property bool reflowPending: _settleTimer.running

    readonly property Timer _settleTimer: Timer {
        interval:   root._settleMSecs
        repeat:     false
        onTriggered: root.reflow()
    }

    // Throttled, never debounced: telemetry chips re-measure as their values change, and
    // restarting the timer on each of those would starve the reflow for the whole flight.
    function requestReflow() {
        if (!_settleTimer.running) {
            _settleTimer.start()
        }
    }

    // Every registered item is watched, so nothing has to remember to ask for a reflow: a chip
    // whose value got wider, chrome that expanded, an item that appeared - all of them are a
    // size or visibility change, and all of them can create an overlap.
    //
    // Statics are watched for position too. The video rail is bound to the pip's edge and so
    // travels whenever the pip is dragged or nudged; a rig deaf to that left telemetry chips
    // sitting under the rail's buttons until something else happened to resize. Movables are
    // deliberately not watched that way: the rig moves them itself, through a 350ms Behavior,
    // and a reflow landing mid-animation reads a position that is neither where the item was
    // nor where it is going.
    readonly property Component _geometryWatcher: Component {
        Connections {
            property var  rig
            property bool watchPosition: false
            function onXChanged()       { if (watchPosition) rig.requestReflow() }
            function onYChanged()       { if (watchPosition) rig.requestReflow() }
            function onWidthChanged()   { rig.requestReflow() }
            function onHeightChanged()  { rig.requestReflow() }
            function onVisibleChanged() { rig.requestReflow() }
        }
    }

    function _watch(item, watchPosition) {
        return _geometryWatcher.createObject(root, { target: item, rig: root,
                                                     watchPosition: watchPosition })
    }

    function _unwatch(entries, item) {
        entries.filter((entry) => entry.item === item && entry.watcher)
               .forEach((entry) => entry.watcher.destroy())
    }

    Component.onCompleted: _settleTimer.start()

    // One grid unit: keeps pushed items a full snap-cell apart, so grid quantization after a
    // push cannot land two items back in contact.
    readonly property real _gap: ScreenTools.defaultFontPixelHeight

    // Matches DragToPosition.edgeMargin: pushes and clamps respect the same breathing room
    // from every screen edge that drops do.
    readonly property real edgeMargin: ScreenTools.defaultFontPixelHeight / 2

    property var _movables: []
    property var _statics:  []

    function registerMovable(item, dragPosition) {
        _unwatch(_movables, item)
        _movables = [..._movables.filter((entry) => entry.item !== item),
                     { item: item, dragPosition: dragPosition, watcher: _watch(item, false) }]
        requestReflow()
    }

    function unregisterMovable(item) {
        _unwatch(_movables, item)
        _movables = _movables.filter(function(entry) { return entry.item !== item })
        requestReflow()
    }

    function registerStatic(item, owner) {
        _unwatch(_statics, item)
        _statics = [..._statics.filter((entry) => entry.item !== item),
                    { item: item, owner: owner || null, watcher: _watch(item, true) }]
        requestReflow()
    }

    function unregisterStatic(item) {
        _unwatch(_statics, item)
        _statics = _statics.filter((entry) => entry.item !== item)
        requestReflow()
    }

    // A resize does not rewrite anything: each window size keeps its own arrangement in
    // DragToPosition, so the rig only has to settle whatever the new size left overlapping.
    readonly property Connections _viewportWatcher: Connections {
        target: root.viewport

        function onWidthChanged()  { root._settleTimer.restart() }
        function onHeightChanged() { root._settleTimer.restart() }
    }

    // Per-item hide flags for overlay chrome (sliders, compass, camera buttons). Reads fall
    // through to persisted settings, so no registration pass is needed; setHidden replaces the
    // override map so bindings on isHidden() re-evaluate.
    property var _hiddenOverrides: ({})

    function isHidden(key) {
        if (_hiddenOverrides[key] !== undefined) {
            return _hiddenOverrides[key]
        }
        return QGroundControl.loadBoolGlobalSetting("OverlayRigHidden-" + key, false)
    }

    // Everything back to the shipped layout: positions to their defaults, nothing hidden.
    function resetLayout() {
        _movables.forEach((entry) => { if (entry.dragPosition) entry.dragPosition.reset() })
        Object.keys(_hiddenOverrides).forEach((key) => setHidden(key, false))
        _hiddenKeys.forEach((key) => setHidden(key, false))
    }

    property var _hiddenKeys: []

    function registerHideKey(key) {
        if (key !== "" && !_hiddenKeys.includes(key)) {
            _hiddenKeys = [..._hiddenKeys, key]
        }
    }

    function setHidden(key, hidden) {
        registerHideKey(key)
        var overrides = Object.assign({}, _hiddenOverrides)
        overrides[key] = hidden
        _hiddenOverrides = overrides
        QGroundControl.saveBoolGlobalSetting("OverlayRigHidden-" + key, hidden)
    }

    function hitTest(viewportX, viewportY) {
        const items = _movables.map(function(entry) { return entry.item }).concat(_statics.map(function(entry) { return entry.item }))
        return items.some(function(item) {
            if (!item || !item.visible) {
                return false
            }
            const r = _rectOf(item)
            return viewportX >= r.x && viewportX <= r.x + r.w &&
                   viewportY >= r.y && viewportY <= r.y + r.h
        })
    }

    function _rectOf(item) {
        const pos = item.parent.mapToItem(viewport, item.x, item.y)
        return { x: pos.x, y: pos.y, w: item.width, h: item.height, dx: 0, dy: 0 }
    }

    // dx/dy accumulate the travel so far, which is what gives an item momentum: see _mtv.
    function _slide(rect, x, y) {
        rect.x  += x
        rect.y  += y
        rect.dx += x
        rect.dy += y
    }

    function _fitsInViewport(rect, v) {
        return rect.x + v.x >= edgeMargin && rect.x + v.x + rect.w <= viewport.width - edgeMargin &&
               rect.y + v.y >= topInset + edgeMargin && rect.y + v.y + rect.h <= viewport.height - edgeMargin
    }

    // Push vector separating `a` from `b`, or null when the rects (padded by _gap) do not
    // overlap. Three filters narrow the four escapes down, in order:
    //
    // On screen: an item against an edge has its shortest escape pointing off it, and _clamp
    // would put it straight back, so the pair stayed overlapping through every pass.
    //
    // Momentum: a chip wedged between the video rail and the map used to be pushed left off the
    // rail, right off the map, left off the rail... for all sixty passes, because each obstacle
    // taken alone had a short sideways escape. An item that has started moving one way does not
    // reverse, so the second obstacle has to answer with the next-shortest escape - here, up -
    // and the chip slips out of the pocket instead of rattling inside it.
    //
    // Shortest of what is left: the least disturbance to a layout the user arranged.
    function _mtv(a, b) {
        const pushRight = (b.x + b.w + _gap) - a.x
        const pushLeft  = (a.x + a.w + _gap) - b.x
        const pushDown  = (b.y + b.h + _gap) - a.y
        const pushUp    = (a.y + a.h + _gap) - b.y
        if (pushRight <= 0 || pushLeft <= 0 || pushDown <= 0 || pushUp <= 0) {
            return null
        }
        const candidates = [ { x: pushRight, y: 0 }, { x: -pushLeft, y: 0 },
                             { x: 0, y: pushDown },  { x: 0, y: -pushUp } ]
        const onScreen = candidates.filter(function(v) { return _fitsInViewport(a, v) })
        const allowed  = onScreen.length > 0 ? onScreen : candidates
        const forward  = allowed.filter(function(v) { return v.x * a.dx >= 0 && v.y * a.dy >= 0 })
        const usable   = forward.length > 0 ? forward : allowed
        return usable.reduce(function(best, v) {
            return (Math.abs(v.x) + Math.abs(v.y)) < (Math.abs(best.x) + Math.abs(best.y)) ? v : best
        })
    }

    function _push(rect, other) {
        const v = _mtv(rect, other)
        if (!v) {
            return false
        }
        _slide(rect, v.x, v.y)
        return true
    }

    // Area is mass. Two overlapping movables split the separation in inverse proportion, so a
    // telemetry chip slides around the compass instead of shoving it: the light one travels
    // almost the whole way, the heavy one barely stirs. Statics and the item under the cursor
    // are infinitely heavy and never give ground. The escape direction is chosen for the
    // lighter rect, since that is the one that actually has to land somewhere on screen.
    function _separate(a, b, massA, massB) {
        const light     = massA <= massB ? a : b
        const heavy     = light === a ? b : a
        const lightMass = light === a ? massA : massB
        const heavyMass = light === a ? massB : massA
        const v = _mtv(light, heavy)
        if (!v) {
            return false
        }
        const share = heavyMass === Infinity ? 1 : heavyMass / (lightMass + heavyMass)
        _slide(light, v.x * share, v.y * share)
        _slide(heavy, -v.x * (1 - share), -v.y * (1 - share))
        return true
    }

    function _touches(a, b) {
        return a.x < b.x + b.w + _gap && b.x < a.x + a.w + _gap &&
               a.y < b.y + b.h + _gap && b.y < a.y + a.h + _gap
    }

    function _isClear(rect, item, items, statics, staticRects) {
        return !staticRects.some((obstacle, s) => statics[s].owner !== item.entry.item && _touches(rect, obstacle)) &&
               !items.some((other) => other !== item && _touches(rect, other.rect))
    }

    // concat rather than flatMap: Qt's JS engine has no Array.prototype.flatMap, and the
    // TypeError only surfaced the first time an item actually needed evicting.
    function _ringOffsets(ring) {
        const span = Array.from({ length: ring * 2 + 1 }, (_, i) => i - ring)
        return span.map((dx) => span.filter((dy) => Math.abs(dx) === ring || Math.abs(dy) === ring)
                                    .map((dy) => ({ dx: dx, dy: dy })))
                   .reduce((all, row) => all.concat(row), [])
    }

    // Relaxation gets it right for any layout with room to breathe, but a crowded corner can
    // pin an item against three obstacles at once and no single-axis push frees it. Overlap is
    // not allowed to survive a reflow, so whatever is still buried walks outwards in _gap steps
    // until it finds a spot that touches nothing.
    //
    // The ring loop stays a loop on purpose: it returns on the first free ring, normally the
    // first or second, where mapping every ring up front would search the whole viewport every
    // time - seventy rings instead of two.
    function _findFreeSpot(item, items, statics, staticRects) {
        const rings = Math.ceil(Math.max(viewport.width, viewport.height) / _gap)
        for (let ring = 1; ring <= rings; ring++) {
            const spot = _ringOffsets(ring)
                .map((offset) => ({ x: item.rect.x + offset.dx * _gap, y: item.rect.y + offset.dy * _gap,
                                    w: item.rect.w, h: item.rect.h }))
                .find((candidate) => _fitsInViewport(candidate, _noOffset) &&
                                     _isClear(candidate, item, items, statics, staticRects))
            if (spot) {
                return spot
            }
        }
        return null
    }

    readonly property var _noOffset: ({ x: 0, y: 0 })

    function _clamp(rect) {
        const x = Math.max(edgeMargin, Math.min(rect.x, viewport.width - rect.w - edgeMargin))
        const y = Math.max(topInset + edgeMargin, Math.min(rect.y, viewport.height - rect.h - edgeMargin))
        _slide(rect, x - rect.x, y - rect.y)
    }

    function reflow() {
        resolve(null)
    }

    function resolve(draggedItem) {
        if (!viewport) {
            return
        }

        const statics     = _statics.filter((entry) => entry.item && entry.item.visible)
        const staticRects = statics.map((entry) => _rectOf(entry.item))

        // One record per movable instead of four index-aligned arrays: a mismatched index here
        // pushes the wrong item, and nothing about the result would say so.
        //
        // Rects are normalized to un-nudged coordinates: a reflow has to be idempotent, and the
        // item is still animating towards its last nudge when the next one is computed.
        const items = _movables
            .filter((entry) => entry.item && entry.item.visible)
            .map((entry) => {
                const rect = _rectOf(entry.item)
                rect.x -= entry.dragPosition.nudgeX
                rect.y -= entry.dragPosition.nudgeY
                return { entry: entry, rect: rect, dragged: entry.item === draggedItem,
                         mass: entry.item === draggedItem ? Infinity : rect.w * rect.h }
            })

        _relax(items, statics, staticRects)
        _evictStragglers(items, statics, staticRects)
        _applyPositions(items)
    }

    // Mutation in place, deliberately: this is an iterative relaxation, and each pass has to see
    // where the previous one left every rect.
    function _relax(items, statics, staticRects) {
        for (let pass = 0; pass < _maxResolvePasses; pass++) {
            let moved = false

            for (const item of items) {
                for (let s = 0; s < staticRects.length; s++) {
                    if (statics[s].owner !== item.entry.item) {
                        moved = _push(item.rect, staticRects[s]) || moved
                    }
                }
            }

            for (let i = 0; i < items.length; i++) {
                for (let j = i + 1; j < items.length; j++) {
                    moved = _separate(items[i].rect, items[j].rect, items[i].mass, items[j].mass) || moved
                }
            }

            items.forEach((item) => _clamp(item.rect))

            if (!moved) {
                break
            }
        }
    }

    // Lightest first: a telemetry chip is what should go looking for a gap, never the compass.
    // The item under the cursor is exempt - the user is holding it, and relocating it would
    // persist a position they never chose.
    function _evictStragglers(items, statics, staticRects) {
        const stranded = items
            .filter((item) => !item.dragged)
            .sort((a, b) => a.mass - b.mass)
            .filter((item) => {
                if (_isClear(item.rect, item, items, statics, staticRects)) {
                    return false
                }
                const spot = _findFreeSpot(item, items, statics, staticRects)
                if (!spot) {
                    return true
                }
                _slide(item.rect, spot.x - item.rect.x, spot.y - item.rect.y)
                return false
            })

        // Not a hypothetical: it means the overlay does not fit the window at all. Silently
        // leaving items stacked is how this went unnoticed for three rounds of "still overlaps".
        if (stranded.length > 0) {
            console.warn("OverlayRig: no free space for", stranded.length,
                         "of", items.length, "items in", viewport.width + "x" + viewport.height)
        }
    }

    function _applyPositions(items) {
        items.forEach((item) => {
            const target = item.entry.item
            const dragPosition = item.entry.dragPosition
            const local = viewport.mapToItem(target.parent, item.rect.x, item.rect.y)
            const dx = local.x - (target.x - dragPosition.nudgeX)
            const dy = local.y - (target.y - dragPosition.nudgeY)

            // Only the item the user just dropped keeps its correction: that one is a placement
            // they made, so it persists. Everything else yields through a nudge, which is
            // transient - remove whatever crowded it and the item springs back.
            if (item.dragged) {
                if (Math.abs(dx) > _positionEpsilon || Math.abs(dy) > _positionEpsilon) {
                    // Not snapped: the rig has just guaranteed a _gap between these rects, and
                    // rounding each one to the drop grid afterwards can eat that gap entirely.
                    dragPosition.moveTo(local.x, local.y, false)
                }
                return
            }
            dragPosition.nudgeX = dx
            dragPosition.nudgeY = dy
        })
    }
}
