/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

pragma Singleton

import QtQuick

QtObject {
    id: _root

    property Item contentSource:   null
    property Item contentBackdrop: null
    property Item fullSource:      null
    property Item fullBackdrop:    null
    property bool isDark:          true

    readonly property bool enabled: contentBackdrop !== null || fullBackdrop !== null

    property int refreshHz: 10

    property bool sourceAnimating: false

    onSourceAnimatingChanged: if (!sourceAnimating) refresh()

    signal refreshed()

    function refresh() {
        refreshed()
    }

    property Timer _pulse: Timer {
        interval:       Math.round(1000 / _root.refreshHz)
        repeat:         true
        running:        _root.enabled && _root.sourceAnimating && Qt.application.active
        onTriggered:    _root.refreshed()
    }

    function _contains(root, item) {
        for (let it = item; it; it = it.parent) {
            if (it === root) {
                return true
            }
        }
        return false
    }

    function forItem(item) {
        if (!item || !enabled) {
            return null
        }
        if (contentSource && _contains(contentSource, item)) {
            return null
        }
        if (fullSource && _contains(fullSource, item)) {
            return contentBackdrop
        }
        return fullBackdrop
    }
}
