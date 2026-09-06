/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Controls

import QGroundControl
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.Templates as T

Item {
    id: control

    required property var overlayRig

    readonly property bool _editMode: overlayRig.editMode

    property real leftInset:  0
    property real rightInset: 0

    readonly property real _freeSpan:       Math.max(0, width - leftInset - rightInset)
    readonly property real _freeSpanCentre: leftInset + _freeSpan / 2

    property var _chipWidths: ({})

    readonly property real _widestChip: {
        const widths = Object.keys(_chipWidths).map((uid) => _chipWidths[uid])
        return widths.length === 0 ? 0 : Math.max(...widths)
    }

    function _noteChipWidth(uid, width) {
        if (_chipWidths[uid] === width) {
            return
        }
        const next = Object.assign({}, _chipWidths)
        next[uid] = width
        _chipWidths = next
    }

    function _forgetChipWidth(uid) {
        _chipWidths = Object.keys(_chipWidths)
                            .filter((key) => key !== uid)
                            .reduce((kept, key) => Object.assign({}, kept, { [key]: _chipWidths[key] }), {})
    }

    readonly property real _columnSpan:  Math.max(_slotWidth, _widestChip + _chipSpacing)
    readonly property int  _bandColumns: Math.max(1, Math.floor(_freeSpan / _columnSpan))
    readonly property int  _bandCount:   Math.ceil(grid.columns.count / _bandColumns)
    readonly property real _bandHeight:  grid.rowCount * (_chipHeight + _chipSpacing)

    function _bandOf(columnIndex)     { return Math.floor(columnIndex / _bandColumns) }
    function _bandWidth(columnIndex)  { return Math.min(_bandColumns, grid.columns.count - _bandOf(columnIndex) * _bandColumns) }

    readonly property bool _compact:      ScreenTools.isMobile
    readonly property real _chipHeight:   ScreenTools.defaultFontPixelHeight * (_compact ? 1.7 : 2.2)
    readonly property real _chipPadding:  ScreenTools.defaultFontPixelWidth  * (_compact ? 1.4 : 2.5)
    readonly property real _chipSpacing:  ScreenTools.defaultFontPixelWidth  * (_compact ? 0.6 : 1)
    readonly property real _slotWidth:    ScreenTools.defaultFontPixelWidth  * (_compact ? 12 : 16)
    readonly property real _bottomMargin: ScreenTools.defaultFontPixelHeight * (_compact ? 1 : 4)
    readonly property int  _resetArmMSecs: 4000
    readonly property var  _qgcPal:       QGroundControl.globalPalette

    component ChipCapsule: OverlayCapsule {
        height:   _chipHeight
    }

    T.HorizontalFactValueGrid {
        id:                     grid
        settingsGroup:          "TelemetryChips"
        specificVehicleForCard: null
    }

    Component {
        id: editDialogComponent

        InstrumentValueEditDialog { }
    }

    Repeater {
        model: grid.columns

        Item {
            id:             columnHolder
            anchors.fill:   parent

            readonly property int columnIndex: index

            Repeater {
                model: object

                ChipCapsule {
                    id:         chip
                    objectName: "telemetryChip-" + chip.instrumentValueData.uid
                    width:      chipRow.width + _chipPadding * 2
                    highlight:  control._editMode || chipMouseArea.containsMouse
                    lifted:     chipDragHandler.active || (control._editMode && dragPosition.displaced)

                    readonly property var instrumentValueData:  object
                    readonly property int rowIndex:             index

                    onWidthChanged: control._noteChipWidth(chip.instrumentValueData.uid, width)

                    Component.onCompleted: {
                        mainWindow.registerWindowDragExclusion(chip)
                        overlayRig.registerMovable(chip, dragPosition)
                    }

                    Component.onDestruction: {
                        overlayRig.unregisterMovable(chip)
                        control._forgetChipWidth(chip.instrumentValueData.uid)
                    }

                    Behavior on x {
                        enabled: !chipDragHandler.active && dragPosition.settling
                        NumberAnimation { duration: 350; easing.type: Easing.OutBack; easing.overshoot: 2 }
                    }

                    Behavior on y {
                        enabled: !chipDragHandler.active && dragPosition.settling
                        NumberAnimation { duration: 350; easing.type: Easing.OutBack; easing.overshoot: 2 }
                    }

                    Row {
                        id:                 chipRow
                        anchors.centerIn:   parent
                        spacing:            ScreenTools.defaultFontPixelWidth * (control._compact ? 0.5 : 0.75)

                        InstrumentValueLabel {
                            anchors.verticalCenter: parent.verticalCenter
                            instrumentValueData:    chip.instrumentValueData
                        }

                        InstrumentValueValue {
                            anchors.verticalCenter: parent.verticalCenter
                            instrumentValueData:    chip.instrumentValueData
                        }
                    }

                    DragToPosition {
                        id:                 dragPosition
                        target:             chip
                        settingsKeyPrefix:  "TelemetryChip-" + chip.instrumentValueData.uid
                        defaultX:           control._freeSpanCentre +
                                                (((columnHolder.columnIndex % control._bandColumns) -
                                                  (control._bandWidth(columnHolder.columnIndex) / 2)) * control._columnSpan) +
                                                (control._columnSpan - chip.width) / 2
                        defaultY:           control.height - _bottomMargin -
                                                ((control._bandCount - 1 - control._bandOf(columnHolder.columnIndex)) * control._bandHeight) -
                                                ((grid.rowCount - chip.rowIndex) * (_chipHeight + _chipSpacing)) + _chipSpacing
                    }

                    JiggleAnimation {
                        target:  chip
                        running: control._editMode
                        lifted:  chipDragHandler.active || overlayRig.heldItem === chip
                    }

                    DragHandler {
                        id:            chipDragHandler
                        dragThreshold: overlayRig.dragThreshold

                        onActiveChanged: {
                            if (!active) {
                                dragPosition.commit()
                                overlayRig.requestReflow()
                            }
                        }
                    }

                    QGCMouseArea {
                        id:                 chipMouseArea
                        anchors.fill:       parent
                        acceptedButtons:    Qt.LeftButton | Qt.RightButton
                        hoverEnabled:       !ScreenTools.isMobile

                        onClicked: (mouse) => {
                            if (mouse.button === Qt.RightButton) {
                                overlayRig.editMode = !overlayRig.editMode
                            } else if (control._editMode) {
                                editDialogComponent.createObject(mainWindow, { instrumentValueData: chip.instrumentValueData }).open()
                            }
                        }

                        onPressAndHold: overlayRig.hold(chipMouseArea)
                    }

                    QGCToolTip {
                        visible:    chipMouseArea.containsMouse && !chipDragHandler.active
                        text:       control._editMode ? qsTr("Click to edit value • Drag to move")
                                                      : qsTr("Right-click or hold to edit layout")
                    }

                    OverlayEditBadge {
                        rig:        overlayRig
                        visible:    control._editMode && grid.columns.count > 1
                        onClicked:  grid.deleteColumn(columnHolder.columnIndex)
                    }
                }
            }
        }
    }

    Row {
        anchors.horizontalCenter:   parent.horizontalCenter
        anchors.bottom:             parent.bottom
        anchors.bottomMargin:       _chipSpacing
        spacing:                    _chipSpacing
        visible:                    control._editMode

        OverlayPill {
            objectName: "editModeSizePill"
            text:       qsTr("Size: %1").arg(grid.fontSizeNames[grid.fontSize])
            onClicked:  grid.fontSize = (grid.fontSize + 1) % grid.fontSizeNames.length
        }

        OverlayPill {
            text:       qsTr("+ Add Value")
            onClicked: {
                const column = grid.appendColumn()
                Qt.callLater(function() { overlayRig.requestReflow() })
                editDialogComponent.createObject(mainWindow, { instrumentValueData: column.get(0) }).open()
            }
        }

        OverlayPill {
            id:         resetPill
            objectName: "editModeResetPill"
            text:       armed ? qsTr("Tap again to reset") : qsTr("Reset Layout")
            onVisibleChanged: if (!visible) armed = false
            onClicked: {
                if (armed) {
                    overlayRig.resetLayout()
                    armed = false
                    return
                }
                armed = true
                resetArmTimer.restart()
            }

            property bool armed: false

            Timer {
                id:          resetArmTimer
                interval:    control._resetArmMSecs
                onTriggered: resetPill.armed = false
            }
        }
    }
}
