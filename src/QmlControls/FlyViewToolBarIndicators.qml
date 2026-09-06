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

Row {
    id:      indicatorRow
    spacing: ScreenTools.defaultFontPixelWidth * 1.75

    property var _activeVehicle: QGroundControl.multiVehicleManager.activeVehicle
    property var _overlayRig:    globals.overlayRigFlyView

    readonly property bool _editMode: _overlayRig ? _overlayRig.editMode : false

    readonly property string _orderSettingsKey: "FlyViewIndicatorOrder"

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
            id:         slot
            objectName: "indicatorSlot" + key
            height:     indicatorRow.height
            width:   visible ? (loader.item ? loader.item.width : 0) : 0
            z:       dragHandler.active ? 2 : 1
            opacity: hidden && indicatorRow._overlayRig ? indicatorRow._overlayRig.hiddenOpacity : 1
            visible: loader.item && loader.item.showIndicator && (!hidden || indicatorRow._editMode)

            readonly property string key:    indicatorRow.keyOf(modelData)
            readonly property bool   hidden: indicatorRow._overlayRig
                                                 ? indicatorRow._overlayRig.isHidden(key) : false

            property real dragOffset: 0

            transform: Translate { x: slot.dragOffset }

            Behavior on dragOffset { enabled: !dragHandler.active
                                     NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }

            Loader {
                id:             loader
                anchors.top:    parent.top
                anchors.bottom: parent.bottom
                source:         modelData
                enabled:        !indicatorRow._editMode
            }

            JiggleAnimation {
                target:  slot
                running: indicatorRow._editMode && slot.visible
                lifted:  dragHandler.active || (indicatorRow._overlayRig && indicatorRow._overlayRig.heldItem === slot)
            }

            OverlayEditBadge {
                rig:     indicatorRow._overlayRig
                editKey: slot.key
            }

            QGCMouseArea {
                anchors.fill:   parent
                enabled:        !indicatorRow._editMode
                visible:        enabled
                onPressAndHold: if (indicatorRow._overlayRig) indicatorRow._overlayRig.hold(slot)
            }

            DragHandler {
                id:            dragHandler
                target:        null
                enabled:       slot.visible
                dragThreshold: indicatorRow._overlayRig ? indicatorRow._overlayRig.dragThreshold : 32767
                yAxis.enabled: false

                onActiveChanged: if (!active) slot.dragOffset = 0

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
