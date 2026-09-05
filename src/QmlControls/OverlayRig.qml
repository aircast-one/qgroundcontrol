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

QtObject {
    id: root

    property Item viewport
    property real topInset: 0
    property bool editMode: false

    property Item heldItem: null

    readonly property int dragThreshold: editMode ? Application.styleHints.startDragDistance : 32767

    readonly property Timer _holdPulse: Timer {
        interval:    700
        onTriggered: root.heldItem = null
    }

    function hold(item) {
        heldItem = _movableFor(item)
        editMode = true
        _holdPulse.restart()
    }

    function _movableFor(item) {
        for (let it = item; it; it = it.parent) {
            if (_movables.some((entry) => entry.item === it)) {
                return it
            }
        }
        return item
    }

    onEditModeChanged: {
        if (editMode) {
            QGroundControl.hapticFeedback()
        } else {
            heldItem = null
        }
    }

    readonly property real _referenceMass: 3500
    readonly property real _minPickup:     0.15
    readonly property real _minDragScale:  0.4
    readonly property real _maxDragScale:  2.0
    readonly property real _stictionSpeed: 120
    readonly property real _maxSpeed:      6000
    readonly property real _sleepSpeed:    12
    readonly property real _restRadius:    3
    readonly property int  _sleepFrames:   6
    readonly property int  _contactPasses: 3
    readonly property real _positionEpsilon: 0.5

    readonly property real _response:        _tuning("Response", 0.55)
    readonly property real _dampingFraction: _tuning("DampingFraction", 0.85)
    readonly property real _maxPull:         _tuning("MaxPull", 30000)
    readonly property real _friction:        _tuning("Friction", 0.6)
    readonly property real _restitution:     _tuning("Restitution", 0)
    readonly property real _omega:           2 * Math.PI / _response

    function _tuning(name, fallback) {
        const value = parseFloat(QGroundControl.loadGlobalSetting("OverlayRig" + name, ""))
        return isNaN(value) ? fallback : value
    }

    readonly property real _minDt: 0.004
    readonly property real _maxDt: 0.02

    readonly property OverlayPhysics _physics: OverlayPhysics {
        pull:         root._maxPull
        springRadius: root._maxPull / (root._omega * root._omega)
        damping:      2 * root._dampingFraction * root._omega
        friction:     root._friction
        restitution:  root._restitution
    }
    property real _wallsW: -1
    property real _wallsH: -1
    property real _wallsTop: -1

    property real _lastStepMSecs: 0
    property bool debugLog: QGroundControl.loadBoolGlobalSetting("OverlayRigDebug", false)

    property string awakeReport: ""
    property string worldReport: ""

    property real   largestStep:     0
    property string largestStepItem: ""

    property var    _traceLines: []
    property string trace:       ""
    property int    steps:       0
    readonly property int _traceLimit: 600

    function resetDiagnostics() {
        largestStep     = 0
        largestStepItem = ""
        steps           = 0
        _traceLines     = []
        trace           = ""
        awakeReport     = ""
    }

    function _note(text) {
        if (!debugLog) {
            return
        }
        _traceLines.push(steps + " " + text)
        if (_traceLines.length > _traceLimit) {
            _traceLines.splice(0, _traceLines.length - _traceLimit)
        }
    }

    readonly property bool reflowPending: _physicsTimer.running

    readonly property FrameAnimation _physicsTimer: FrameAnimation {
        onTriggered: root._solve(true)
    }

    function requestReflow() {
        _wake()
    }

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

    readonly property Component _homeWatcher: Component {
        Connections {
            property var rig
            function onHomeXChanged() { rig.requestReflow() }
            function onHomeYChanged() { rig.requestReflow() }
        }
    }

    function _watch(item, watchPosition) {
        return _geometryWatcher.createObject(root, { target: item, rig: root,
                                                     watchPosition: watchPosition })
    }

    function _watchHome(dragPosition) {
        return _homeWatcher.createObject(root, { target: dragPosition, rig: root })
    }

    function _unwatch(entries, item) {
        entries.filter((entry) => entry.item === item)
               .forEach((entry) => {
                   if (entry.watcher) {
                       entry.watcher.destroy()
                   }
                   if (entry.homeWatcher) {
                       entry.homeWatcher.destroy()
                   }
               })
    }

    Component.onCompleted: _wake()

    readonly property real _gap: ScreenTools.defaultFontPixelHeight

    readonly property real edgeMargin: ScreenTools.defaultFontPixelHeight / 2

    property var _movables: []
    property var _statics:  []

    function registerMovable(item, dragPosition) {
        _register(item, dragPosition, false)
    }

    function registerAnchor(item, dragPosition) {
        _register(item, dragPosition, true)
    }

    function _register(item, dragPosition, anchored) {
        _unwatch(_movables, item)
        _movables = [..._movables.filter((entry) => entry.item !== item),
                     { item: item, dragPosition: dragPosition, watcher: _watch(item, true),
                       homeWatcher: _watchHome(dragPosition), vx: 0, vy: 0, anchored: anchored }]

        dragPosition.physicsActive = _physicsTimer.running
        requestReflow()
    }

    function unregisterMovable(item) {
        _movables.filter((entry) => entry.item === item).forEach(_dropBody)
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
        _statics.filter((entry) => entry.item === item).forEach(_dropBody)
        _unwatch(_statics, item)
        _statics = _statics.filter((entry) => entry.item !== item)
        requestReflow()
    }

    readonly property Connections _viewportWatcher: Connections {
        target: root.viewport

        function onWidthChanged()  { root._wake() }
        function onHeightChanged() { root._wake() }
    }

    property var _hiddenOverrides: ({})

    function isHidden(key) {
        if (_hiddenOverrides[key] !== undefined) {
            return _hiddenOverrides[key]
        }
        return QGroundControl.loadBoolGlobalSetting("OverlayRigHidden-" + key, false)
    }

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

    function _homeRectOf(entry) {
        const item = entry.item
        const pos  = item.parent.mapToItem(viewport, entry.dragPosition.homeX, entry.dragPosition.homeY)
        return { x: pos.x, y: pos.y, w: item.width, h: item.height, dx: 0, dy: 0 }
    }

    function reflow() {
        resolve(null)
    }

    function resolve(draggedItem) {
        _wake()
    }

    function _wake() {
        if (!viewport) {
            return
        }
        if (!_physicsTimer.running) {
            _lastStepMSecs = Date.now()
            _physicsTimer.start()
            _movables.forEach((entry) => {
                if (entry.dragPosition) {
                    entry.dragPosition.physicsActive = true
                }
            })
        }
    }

    function _sleep() {
        _physicsTimer.stop()
        _movables.forEach((entry) => {
            if (entry.dragPosition) {
                entry.dragPosition.physicsActive = false
            }
        })
    }

    readonly property real _bodyMargin: 3
    function _bodyRect(rect) {
        return { x: rect.x - _bodyMargin, y: rect.y - _bodyMargin, w: rect.w + (_bodyMargin * 2), h: rect.h + (_bodyMargin * 2) }
    }

    function _groupOf(entry) {
        const owner = entry.owner ? _movables.find((movable) => movable.item === entry.owner) : entry
        if (!owner) {
            return 0
        }
        if (owner.group === undefined) {
            _groups += 1
            owner.group = -_groups
        }
        return owner.group
    }
    property int _groups: 0

    function _ensureBody(entry, kind, rect) {
        const body = _bodyRect(rect)
        if (entry.body === undefined || entry.body === 0) {
            entry.body = _physics.create(kind, body.x, body.y, body.w, body.h, _groupOf(entry))
            entry.bodyKind = kind
            return true
        }
        _physics.setSize(entry.body, body.w, body.h)
        if (entry.bodyKind !== kind) {
            _physics.setKind(entry.body, kind)
            _physics.place(entry.body, body.x, body.y)
            entry.bodyKind = kind
        }
        return false
    }

    function _dropBody(entry) {
        if (entry.body) {
            _physics.remove(entry.body)
            entry.body = 0
        }
    }

    function _solve(integrate) {
        if (!viewport || viewport.width <= 0 || viewport.height <= 0) {
            return
        }

        const now = Date.now()
        const dt  = Math.max(_minDt, Math.min((now - _lastStepMSecs) / 1000, _maxDt))
        _lastStepMSecs = now
        steps += 1

        if (_movables.some((entry) => entry.dragPosition && entry.dragPosition.loading)) {
            return
        }

        if (viewport.width !== _wallsW || viewport.height !== _wallsH || topInset !== _wallsTop) {
            _wallsW = viewport.width
            _wallsH = viewport.height
            _wallsTop = topInset
            _physics.setWalls(edgeMargin - _bodyMargin, topInset + edgeMargin - _bodyMargin,
                              viewport.width - edgeMargin + _bodyMargin, viewport.height - edgeMargin + _bodyMargin)

            _movables.filter((entry) => entry.body && entry.bodyKind === OverlayPhysics.Free).forEach((entry) => {
                const x = _physics.x(entry.body)
                const y = _physics.y(entry.body)
                if (x < 0 || y < topInset || x > viewport.width || y > viewport.height) {
                    const home = _bodyRect(_homeRectOf(entry))
                    _physics.place(entry.body, home.x, home.y)
                    entry.dragPosition.setNudge(0, 0)
                }
            })
        }

        const movableItems = _movables.map((entry) => entry.item)
        const bolted = (entry) => entry.owner && movableItems.indexOf(entry.owner) >= 0

        _statics.forEach((entry) => {
            if (!entry.item || !entry.item.visible || bolted(entry)) {
                _dropBody(entry)
                return
            }
            const rect = _rectOf(entry.item)
            const fresh = _ensureBody(entry, OverlayPhysics.Driven, rect)
            const body = _bodyRect(rect)
            if (fresh) {
                _physics.place(entry.body, body.x, body.y)
            } else {
                _physics.drive(entry.body, body.x, body.y, dt)
            }
        })

        const report = []
        let held = false

        _movables.forEach((entry) => {
            if (!entry.item || !entry.item.visible || !entry.dragPosition) {
                _dropBody(entry)
                return
            }
            const home = _homeRectOf(entry)
            const live = _rectOf(entry.item)
            const simulated = { x: home.x + entry.dragPosition.nudgeX, y: home.y + entry.dragPosition.nudgeY }

            const isHeld   = Math.abs(live.x - simulated.x) > 1 || Math.abs(live.y - simulated.y) > 1
            const driven   = isHeld || entry.anchored === true
            const rect     = driven ? live : { x: simulated.x, y: simulated.y, w: home.w, h: home.h }
            const fresh    = _ensureBody(entry, driven ? OverlayPhysics.Driven : OverlayPhysics.Free, rect)
            const body     = _bodyRect(rect)
            const homeBody = _bodyRect(home)

            const attachments = _statics
                .filter((furniture) => furniture.owner === entry.item && furniture.item && furniture.item.visible)
                .map((furniture) => {
                    const part = _bodyRect(_rectOf(furniture.item))
                    return { x: Math.round(part.x - body.x), y: Math.round(part.y - body.y),
                             w: Math.round(part.w), h: Math.round(part.h) }
                })
            _physics.setAttachments(entry.body, attachments)

            if (driven) {
                held = held || isHeld
                if (fresh) {
                    _physics.place(entry.body, body.x, body.y)
                } else {
                    _physics.drive(entry.body, body.x, body.y, dt)
                }
                if (entry.anchored && entry.dragPosition.displaced) {
                    entry.dragPosition.setNudge(0, 0)
                }
                return
            }

            if (fresh || Math.abs(_physics.x(entry.body) - body.x) > _positionEpsilon
                      || Math.abs(_physics.y(entry.body) - body.y) > _positionEpsilon) {
                _physics.place(entry.body, body.x, body.y)
            }
            _physics.setHome(entry.body, homeBody.x, homeBody.y)
        })

        if (integrate) {
            _physics.step(dt)
        }

        _movables.forEach((entry) => {
            if (!entry.body || entry.bodyKind !== OverlayPhysics.Free) {
                return
            }
            const target = entry.item
            const px = _physics.x(entry.body) + _bodyMargin
            const py = _physics.y(entry.body) + _bodyMargin
            const local = viewport.mapToItem(target.parent, px, py)
            const nudgeX = local.x - entry.dragPosition.homeX
            const nudgeY = local.y - entry.dragPosition.homeY
            const travelled = Math.abs(nudgeX - entry.dragPosition.nudgeX) + Math.abs(nudgeY - entry.dragPosition.nudgeY)

            if (integrate && travelled > largestStep) {
                largestStep     = travelled
                largestStepItem = _labelOf(entry) + " " + travelled.toFixed(0) + "px " + _physics.describe(entry.body)
            }
            if (Math.abs(nudgeX) < _positionEpsilon && Math.abs(nudgeY) < _positionEpsilon) {
                if (entry.dragPosition.displaced) {
                    entry.dragPosition.setNudge(0, 0)
                }
            } else if (travelled > 0.01) {
                entry.dragPosition.setNudge(nudgeX, nudgeY)
            }
            if (debugLog && !_physics.asleep(entry.body)) {
                report.push(_labelOf(entry) + " " + _physics.describe(entry.body) + " nudge "
                            + nudgeX.toFixed(0) + "," + nudgeY.toFixed(0))
            }
        })

        if (debugLog && integrate) {
            worldReport = _physics.report()
            _statics.filter((entry) => entry.body).forEach((entry) => {
                report.push(_labelOfStatic(entry) + " " + _physics.describe(entry.body))
            })
            _movables.filter((entry) => entry.body && entry.bodyKind === OverlayPhysics.Driven).forEach((entry) => {
                report.push(_labelOf(entry) + " " + _physics.describe(entry.body))
            })
            awakeReport = report.join("\n")
            if (report.length > 0) {
                _note("step dt=" + (dt * 1000).toFixed(0) + "ms awake=" + report.length + " :: " + report.join(" | "))
            }
            trace = _traceLines.join("\n")
        }

        if (integrate && !held && _physics.allAsleep()) {
            _sleep()
        }
    }

    function _labelOf(entry) {
        const name = entry.item.objectName
        return name !== "" ? name : (entry.dragPosition ? entry.dragPosition.settingsKeyPrefix : "?")
    }

    function _labelOfStatic(entry) {
        const name = entry.item.objectName
        return (name !== "" ? name : "static") + (entry.owner ? " (owned)" : "")
    }
}
