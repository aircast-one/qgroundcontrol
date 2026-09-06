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

    readonly property real _freeSpanCentre: leftInset + (width - leftInset - rightInset) / 2

    readonly property real _chipHeight:   ScreenTools.defaultFontPixelHeight * 2.2
    readonly property real _chipSpacing:  ScreenTools.defaultFontPixelWidth
    readonly property real _slotWidth:    ScreenTools.defaultFontPixelWidth * 16
    readonly property real _bottomMargin: ScreenTools.defaultFontPixelHeight * 4
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
                    width:      chipRow.width + ScreenTools.defaultFontPixelWidth * 2.5
                    highlight:  control._editMode || chipMouseArea.containsMouse
                    lifted:     chipDragHandler.active || (control._editMode && dragPosition.displaced)

                    readonly property var instrumentValueData:  object
                    readonly property int rowIndex:             index

                    Component.onCompleted: {
                        mainWindow.registerWindowDragExclusion(chip)
                        overlayRig.registerMovable(chip, dragPosition)
                    }

                    Component.onDestruction: overlayRig.unregisterMovable(chip)

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
                        spacing:            ScreenTools.defaultFontPixelWidth * 0.75

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
                                                ((columnHolder.columnIndex - (grid.columns.count / 2)) * _slotWidth) +
                                                (_slotWidth - chip.width) / 2
                        defaultY:           control.height - _bottomMargin -
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
