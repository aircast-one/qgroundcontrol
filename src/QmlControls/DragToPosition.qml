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

// Persists a user-dragged position for `target`. Install a drag (MouseArea drag.target or
// DragHandler) on the target and call commit() when the drag ends. Until the user drags,
// the target follows the defaultX/defaultY bindings. Dropping the target so it mostly
// overlaps its default spot snaps it there and clears the custom position.
//
// One arrangement per window size: the position is stored under the size it was made at, so
// the laptop layout and the docked layout are separate and each comes back when the window
// returns to that size. A size never seen before starts from the nearest one on record.
QtObject {
    id: root

    required property Item      target
    required property string    settingsKeyPrefix
    property real               defaultX:       0
    property real               defaultY:       0
    property real               snapThreshold:  ScreenTools.defaultFontPixelHeight

    // Dropped positions quantize to this grid so arrangements line up instead of landing on
    // arbitrary pixels. 0 disables snapping.
    property real               snapGrid:       ScreenTools.defaultFontPixelHeight

    // No item sits flush against a screen edge; drops past an edge dock at this margin.
    property real               edgeMargin:     ScreenTools.defaultFontPixelHeight / 2

    readonly property bool      hasCustomPosition: _customPos

    // Transient displacement applied by the collision rig while a piece of chrome is
    // temporarily larger (the video rail in grid mode). Never persisted, so collapsing the
    // chrome returns the item to its dragged or default spot.
    property real               nudgeX:            0
    property real               nudgeY:            0

    // True for as long as the rig is holding this item off something else - not a pulse on the
    // move, a standing state. An item parked away from where it was put says so.
    readonly property bool      displaced:         nudgeX !== 0 || nudgeY !== 0

    property bool _customPos:   false
    property real _customX:     0
    property real _customY:     0

    property real _sizeW:       0
    property real _sizeH:       0

    readonly property int  _saveDelayMSecs:   250
    readonly property int  _reloadDelayMSecs: 250

    // Window sizes worth remembering an arrangement for, most recently used last. Capped: the
    // list is walked on every load, and a resize drag can otherwise stop at hundreds of sizes.
    readonly property int  _maxRememberedSizes: 8

    readonly property string _sizeKey:  _sizeW + "x" + _sizeH
    readonly property string _sizesKey: settingsKeyPrefix + "Sizes"

    // Read once and kept in step by _save: only this object ever writes the list, so a settings
    // read per item per resize buys nothing.
    property var _sizes: []

    function _group(sizeKey) {
        return settingsKeyPrefix + sizeKey + "-"
    }

    function _clampToParent(value, bound) {
        return Math.max(edgeMargin, Math.min(value, bound - edgeMargin))
    }

    function _bindPosition() {
        target.x = Qt.binding(function() {
            return (_customPos && target.parent
                ? _clampToParent(_customX, target.parent.width - target.width)
                : defaultX) + nudgeX
        })
        target.y = Qt.binding(function() {
            return (_customPos && target.parent
                ? _clampToParent(_customY, target.parent.height - target.height)
                : defaultY) + nudgeY
        })
    }

    // Where the user let go is where it belongs, nudge included: the drop is a placement, so
    // the transient displacement it may have been carrying becomes part of the stored position.
    function commit() {
        const x = target.x
        const y = target.y
        nudgeX = 0
        nudgeY = 0
        var snapX = Math.max(snapThreshold, target.width / 2)
        var snapY = Math.max(snapThreshold, target.height / 2)
        if (Math.abs(x - defaultX) < snapX && Math.abs(y - defaultY) < snapY) {
            reset()
            return
        }
        moveTo(x, y)
    }

    // A position past an edge docks at the edge margin: the grid is for arranging items in
    // open space, and rounding an edge-docked item to the nearest grid line leaves it floating
    // off the edge it was dropped against.
    function _place(value, limit, snapToGrid) {
        const bound = Math.max(edgeMargin, limit - edgeMargin)
        if (value <= edgeMargin) {
            return edgeMargin
        }
        if (value >= bound) {
            return bound
        }
        if (!snapToGrid || snapGrid <= 0) {
            return value
        }
        return Math.max(edgeMargin, Math.min(Math.round(value / snapGrid) * snapGrid, bound))
    }

    // Programmatic placement (e.g. the collision rig shifting an item aside): persists like a
    // user drag, without the snap-to-default check.
    function moveTo(newX, newY, snapToGrid = true) {
        _customX = _place(newX, target.parent ? target.parent.width  - target.width  : newX, snapToGrid)
        _customY = _place(newY, target.parent ? target.parent.height - target.height : newY, snapToGrid)
        _customPos = true
        _saveTimer.restart()
        _bindPosition()
    }

    // Three settings writes per call, and the rig moves every registered item at once - a
    // window drag would turn that into thousands of disk writes a second. The position lives in
    // memory immediately; only the write waits for things to stop moving.
    readonly property Timer _saveTimer: Timer {
        interval:    root._saveDelayMSecs
        onTriggered: root._save()
    }

    function _save() {
        QGroundControl.saveBoolGlobalSetting(_group(_sizeKey) + "CustomPosition", true)
        QGroundControl.saveGlobalSetting(_group(_sizeKey) + "PositionX", _customX.toString())
        QGroundControl.saveGlobalSetting(_group(_sizeKey) + "PositionY", _customY.toString())
        _rememberSize()
    }

    function _rememberSize() {
        const kept = [..._sizes.filter((size) => size !== _sizeKey), _sizeKey]
        _sizes = kept.slice(Math.max(0, kept.length - _maxRememberedSizes))
        QGroundControl.saveGlobalSetting(_sizesKey, _sizes.join(","))
    }

    function reset() {
        _customPos = false
        nudgeX = 0
        nudgeY = 0
        _saveTimer.stop()
        QGroundControl.saveBoolGlobalSetting(_group(_sizeKey) + "CustomPosition", false)
        _bindPosition()
    }

    property bool _completed: false

    function _loadSizes() {
        _sizes = QGroundControl.loadGlobalSetting(_sizesKey, "")
                               .split(",")
                               .filter((size) => isFinite(_distanceTo(size)))
    }

    function _distanceTo(sizeKey) {
        const parts = sizeKey.split("x")
        return parts.length !== 2
            ? NaN
            : Math.abs(parseFloat(parts[0]) - _sizeW) + Math.abs(parseFloat(parts[1]) - _sizeH)
    }

    // A window size with no arrangement of its own inherits the closest one on record rather
    // than dropping back to the shipped defaults: the user arranged something, and the nearest
    // size is the closest thing to what they meant. The rig separates whatever overlaps after.
    function _storedKey() {
        if (_sizes.includes(_sizeKey)) {
            return _sizeKey
        }
        return _sizes.length === 0
            ? ""
            : _sizes.reduce((best, size) => _distanceTo(size) < _distanceTo(best) ? size : best)
    }

    function _load() {
        _customPos = false
        const key = _storedKey()
        if (key !== "" && QGroundControl.loadBoolGlobalSetting(_group(key) + "CustomPosition", false)) {
            var x = parseFloat(QGroundControl.loadGlobalSetting(_group(key) + "PositionX", "0"))
            var y = parseFloat(QGroundControl.loadGlobalSetting(_group(key) + "PositionY", "0"))
            if (isFinite(x) && isFinite(y)) {
                _customX = x
                _customY = y
                _customPos = true
            }
        }
        _bindPosition()
    }

    // Only the settings read is debounced. The key itself follows the window immediately: a
    // drop made just after a resize belongs to the size that was on screen when it was made,
    // and a lagging key filed it under the previous one.
    readonly property Timer _reloadTimer: Timer {
        interval:    root._reloadDelayMSecs
        onTriggered: { root._loadSizes(); root._load() }
    }

    // The target gets reparented (PipView swaps its two items), which moves it into a container
    // of a different size. The Connections target re-binds itself, but the size is only read
    // when _adoptSize runs, so the reparent has to ask for one.
    readonly property Connections _targetWatcher: Connections {
        target: root.target

        function onParentChanged() { root._adoptSize() }
    }

    readonly property Connections _parentWatcher: Connections {
        target: root.target ? root.target.parent : null

        function onWidthChanged()  { root._adoptSize() }
        function onHeightChanged() { root._adoptSize() }
    }

    function _adoptSize() {
        if (!_completed || !target || !target.parent) {
            return
        }
        const w = Math.round(target.parent.width)
        const h = Math.round(target.parent.height)
        if (w <= 0 || h <= 0 || (w === _sizeW && h === _sizeH)) {
            return
        }
        // Whatever is queued was arranged at the old size, so it is written under the old key.
        if (_saveTimer.running) {
            _saveTimer.stop()
            _save()
        }
        _sizeW = w
        _sizeH = h
        _reloadTimer.restart()
    }

    // The key can change at runtime (a telemetry chip re-keyed to a new fact): pick up the
    // position stored under the new key instead of dragging the old one along.
    onSettingsKeyPrefixChanged: {
        if (_completed) {
            _load()
        }
    }

    Component.onCompleted: {
        _completed = true
        _adoptSize()
        _reloadTimer.stop()
        _loadSizes()
        _load()
    }
}
