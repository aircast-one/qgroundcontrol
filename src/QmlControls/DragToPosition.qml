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

    required property Item      target
    required property string    settingsKeyPrefix
    property real               defaultX:       0
    property real               defaultY:       0
    property real               snapThreshold:  ScreenTools.defaultFontPixelHeight

    property real               snapGrid:       ScreenTools.defaultFontPixelHeight

    property real               edgeMargin:     ScreenTools.defaultFontPixelHeight / 2
    property var                aligner:        null

    readonly property bool      hasCustomPosition: _customPos

    property real               nudgeX:            0
    property real               nudgeY:            0

    property bool               physicsActive:     false
    readonly property bool      settling:          !physicsActive
    readonly property bool      loading:           _reloadTimer.running

    function setNudge(x, y) {
        nudgeX = x
        nudgeY = y
    }

    readonly property bool      displaced:         nudgeX !== 0 || nudgeY !== 0

    property bool _customPos:   false
    property real _customX:     0
    property real _customY:     0

    property real _sizeW:       0
    property real _sizeH:       0

    readonly property int  _saveDelayMSecs:   250
    readonly property int  _reloadDelayMSecs: 250

    readonly property int  _maxRememberedSizes: 8

    readonly property string _sizeKey:  _sizeW + "x" + _sizeH
    readonly property string _sizesKey: settingsKeyPrefix + "Sizes"

    property var _sizes: []

    function _group(sizeKey) {
        return settingsKeyPrefix + sizeKey + "-"
    }

    function _clampToParent(value, bound) {
        return Math.max(edgeMargin, Math.min(value, bound - edgeMargin))
    }

    function _snap(value, extent, size) {
        const low  = edgeMargin
        const high = Math.max(low, size - edgeMargin - extent)
        if (snapGrid <= 0) {
            return Math.max(low, Math.min(value, high))
        }
        const nearFarEdge = value + extent / 2 > size / 2
        const snapped = nearFarEdge ? high - Math.round((high - value) / snapGrid) * snapGrid
                                    : low + Math.round((value - low) / snapGrid) * snapGrid
        return Math.max(low, Math.min(snapped, high))
    }

    readonly property real homeX: !target || !target.parent ? defaultX
                                : _customPos ? _clampToParent(_customX, target.parent.width - target.width)
                                             : _snap(defaultX, target.width, target.parent.width)
    readonly property real homeY: !target || !target.parent ? defaultY
                                : _customPos ? _clampToParent(_customY, target.parent.height - target.height)
                                             : _snap(defaultY, target.height, target.parent.height)

    function _bindPosition() {
        target.x = Qt.binding(function() { return homeX + nudgeX })
        target.y = Qt.binding(function() { return homeY + nudgeY })
    }

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

    function _place(value, extent, size, snapToGrid) {
        if (!snapToGrid) {
            return Math.max(edgeMargin, Math.min(value, Math.max(edgeMargin, size - edgeMargin - extent)))
        }
        return _snap(value, extent, size)
    }

    function moveTo(newX, newY, snapToGrid = true) {
        const aligned = snapToGrid && aligner && target.parent ? aligner(target, newX, newY) : null
        _customX = aligned ? _place(aligned.x, target.width,  target.parent.width,  false)
                           : _place(newX, target.width,  target.parent ? target.parent.width  : newX + target.width,  snapToGrid)
        _customY = aligned ? _place(aligned.y, target.height, target.parent.height, false)
                           : _place(newY, target.height, target.parent ? target.parent.height : newY + target.height, snapToGrid)
        _customPos = true
        _saveTimer.restart()
        _bindPosition()
    }

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

    readonly property Timer _reloadTimer: Timer {
        interval:    root._reloadDelayMSecs
        onTriggered: { root._loadSizes(); root._load() }
    }

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

        if (_saveTimer.running) {
            _saveTimer.stop()
            _save()
        }
        _sizeW = w
        _sizeH = h
        _reloadTimer.restart()
    }

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
