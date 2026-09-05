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
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.Toolbar

//-------------------------------------------------------------------------
//-- Toolbar Indicators
//
// One ordered strip rather than three fixed Repeaters. The core-plugin, vehicle-tool and
// vehicle-mode indicators are flattened into a single list, so an item can be dragged past any
// other and the order the user lands on is what gets stored.
//
// Edit mode is the fly view's edit mode: hold any indicator, or enter it from anywhere else,
// and the whole screen becomes arrangeable at once. Hiding runs through the same rig, so
// "Reset Layout" restores these along with everything else.
//
// The Row keeps ownership of x. Reordering happens in the model and the Row re-lays out; the
// dragged item is offset on top of that with a Translate, which the Row does not manage. Laying
// the strip out by hand instead meant reading widths back out of the delegates, which is not a
// bindable dependency - positions went stale the moment a battery percentage changed width.
Row {
    id:      indicatorRow
    spacing: ScreenTools.defaultFontPixelWidth * 1.75

    property var _activeVehicle: QGroundControl.multiVehicleManager.activeVehicle
    property var _overlayRig:    globals.overlayRigFlyView

    readonly property bool _editMode: _overlayRig ? _overlayRig.editMode : false

    readonly property string _orderSettingsKey: "FlyViewIndicatorOrder"

    // Flight mode stays in the toolbar's fixed status cluster, beside the main status pill.
    // The two are read together - what the vehicle is doing and whether it will let you - and
    // splitting them to make one of them draggable is a bad trade.
    readonly property var _available: {
        const app   = QGroundControl.corePlugin.toolBarIndicators
        const tools = _activeVehicle ? _activeVehicle.toolIndicators : []
        const modes = _activeVehicle ? _activeVehicle.modeIndicators : []
        return [...app, ...tools, ...modes]
                   .map((source) => source.toString())
                   .filter((source) => source.indexOf("FlightModeIndicator") < 0)
    }

    function keyOf(source) {
        return source.substring(source.lastIndexOf("/") + 1).replace(".qml", "")
    }

    property var _order: []

    // Stored order first, then anything the vehicle has offered since. A stored entry whose
    // indicator is not present is kept in _order but skipped here, so unplugging hardware does
    // not forget where its indicator was.
    readonly property var _sources: {
        const available = _available
        const ordered   = _order.map((key) => available.find((source) => keyOf(source) === key))
                                .filter((source) => source !== undefined)
        const rest      = available.filter((source) => !_order.includes(keyOf(source)))
        return [...ordered, ...rest]
    }

    Component.onCompleted: {
        const saved = QGroundControl.loadGlobalSetting(_orderSettingsKey, "")
        _order = saved.split(",").filter((key) => key !== "")
    }

    // Called mid-drag, so it must not disturb anything but the order.
    function _moveKey(key, delta) {
        const keys = _sources.map(keyOf)
        const from = keys.indexOf(key)
        const to   = from + delta
        if (from < 0 || to < 0 || to >= keys.length) {
            return false
        }
        keys.splice(to, 0, keys.splice(from, 1)[0])
        _order = keys
        QGroundControl.saveGlobalSetting(_orderSettingsKey, keys.join(","))
        return true
    }

    Repeater {
        model: indicatorRow._sources

        Item {
            id:      slot
            height:  indicatorRow.height
            width:   visible ? (loader.item ? loader.item.width : 0) : 0
            z:       dragHandler.active ? 2 : 1
            opacity: hidden ? 0.35 : 1
            visible: loader.item && loader.item.showIndicator && (!hidden || indicatorRow._editMode)

            readonly property string key:    indicatorRow.keyOf(modelData)
            readonly property bool   hidden: indicatorRow._overlayRig
                                                 ? indicatorRow._overlayRig.isHidden(key) : false

            // Carried across a reorder: the Row moves the item by a whole slot the instant the
            // order changes, and without cancelling that out by the same amount the item would
            // jump out from under the finger.
            property real dragOffset: 0

            transform: Translate { x: slot.dragOffset }

            Behavior on dragOffset { enabled: !dragHandler.active
                                     NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }

            Loader {
                id:             loader
                anchors.top:    parent.top
                anchors.bottom: parent.bottom
                source:         modelData
            }

            JiggleAnimation {
                target:  slot
                running: indicatorRow._editMode && slot.visible
                lifted:  dragHandler.active
            }

            OverlayEditBadge {
                rig:     indicatorRow._overlayRig
                editKey: slot.key
            }

            // Hold to arrange, the same gesture as every other overlay control.
            QGCMouseArea {
                anchors.fill:   parent
                enabled:        !indicatorRow._editMode
                visible:        enabled
                onPressAndHold: if (indicatorRow._overlayRig) indicatorRow._overlayRig.editMode = true
            }

            DragHandler {
                id:            dragHandler
                enabled:       indicatorRow._editMode && slot.visible
                yAxis.enabled: false

                onActiveChanged: if (!active) slot.dragOffset = 0

                // Swap when the dragged item's own edge clears the neighbour it is heading for,
                // rather than when the pointer does: the swap then matches what is on screen.
                onTranslationChanged: {
                    if (!active) {
                        return
                    }
                    slot.dragOffset = translation.x
                    const step = slot.width + indicatorRow.spacing
                    while (slot.dragOffset > step / 2 && indicatorRow._moveKey(slot.key, 1)) {
                        slot.dragOffset -= step
                    }
                    while (slot.dragOffset < -step / 2 && indicatorRow._moveKey(slot.key, -1)) {
                        slot.dragOffset += step
                    }
                }
            }
        }
    }
}
