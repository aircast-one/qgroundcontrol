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

    // Any view other than the fly view registers the surface its own chrome floats over here.
    // Without one, glass anywhere else fell through to the fly view's backdrop and frosted a
    // blurred picture of a map the panel was not actually sitting on - stale, and offset by
    // however far the two maps had drifted apart.
    property var scopes: []

    function addScope(source, backdrop) {
        scopes = [...scopes.filter(scope => scope.source !== source), { source: source, backdrop: backdrop }]
    }

    function removeScope(source) {
        scopes = scopes.filter(scope => scope.source !== source)
    }

    readonly property bool enabled: contentBackdrop !== null || fullBackdrop !== null || scopes.length > 0

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
        // Innermost registered scope wins, so a view nested inside another still frosts its own.
        for (let i = scopes.length - 1; i >= 0; --i) {
            if (scopes[i].source && _contains(scopes[i].source, item)) {
                return scopes[i].backdrop
            }
        }
        return fullBackdrop
    }
}
