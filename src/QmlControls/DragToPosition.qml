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
QtObject {
    id: root

    required property Item      target
    required property string    settingsKeyPrefix
    property real               defaultX:       0
    property real               defaultY:       0
    property real               snapThreshold:  ScreenTools.defaultFontPixelHeight

    readonly property bool      hasCustomPosition: _customPos

    property bool _customPos:   false
    property real _customX:     0
    property real _customY:     0

    readonly property string _customPosSettingsKey: settingsKeyPrefix + "CustomPosition"
    readonly property string _xSettingsKey:         settingsKeyPrefix + "PositionX"
    readonly property string _ySettingsKey:         settingsKeyPrefix + "PositionY"

    function _bindPosition() {
        target.x = Qt.binding(function() { return _customPos && target.parent ? Math.max(0, Math.min(_customX, target.parent.width - target.width)) : defaultX })
        target.y = Qt.binding(function() { return _customPos && target.parent ? Math.max(0, Math.min(_customY, target.parent.height - target.height)) : defaultY })
    }

    function commit() {
        var snapX = Math.max(snapThreshold, target.width / 2)
        var snapY = Math.max(snapThreshold, target.height / 2)
        if (Math.abs(target.x - defaultX) < snapX && Math.abs(target.y - defaultY) < snapY) {
            reset()
            return
        }
        _customX = target.x
        _customY = target.y
        _customPos = true
        QGroundControl.saveBoolGlobalSetting(_customPosSettingsKey, true)
        QGroundControl.saveGlobalSetting(_xSettingsKey, _customX.toString())
        QGroundControl.saveGlobalSetting(_ySettingsKey, _customY.toString())
        _bindPosition()
    }

    function rebind() {
        _bindPosition()
    }

    function reset() {
        _customPos = false
        QGroundControl.saveBoolGlobalSetting(_customPosSettingsKey, false)
        _bindPosition()
    }

    Component.onCompleted: {
        if (QGroundControl.loadBoolGlobalSetting(_customPosSettingsKey, false)) {
            var x = parseFloat(QGroundControl.loadGlobalSetting(_xSettingsKey, "0"))
            var y = parseFloat(QGroundControl.loadGlobalSetting(_ySettingsKey, "0"))
            if (isFinite(x) && isFinite(y)) {
                _customX = x
                _customY = y
                _customPos = true
            }
        }
        _bindPosition()
    }
}
