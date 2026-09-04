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

// Telemetry as free-floating chips: every value is its own themed capsule that drags anywhere
// and keeps its place. The fact/icon configuration reuses the FactValueGrid model, so existing
// telemetry-bar settings (and the per-value edit dialog with its icon picker) carry over.
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
        height: _chipHeight
    }

    // A fresh settings group (not the old telemetry bar's): the DJI default set applies
    // instead of any stored grid layout, and chip edits stay isolated from vehicle cards.
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
                    // Named so tools/ui-probe.py can see it. Without a name the overlap check
                    // skips every chip, which is the one control most likely to end up under
                    // the video rail. Keyed by uid for the same reason settingsKeyPrefix is:
                    // a column index names a different chip once a column is deleted.
                    objectName: "telemetryChip-" + chip.instrumentValueData.uid
                    width:      chipRow.width + ScreenTools.defaultFontPixelWidth * 2.5
                    highlight:  control._editMode || chipMouseArea.containsMouse
                    // Raised for as long as the rig holds it off something, so a chip parked
                    // away from where it was put does not read as the place the user chose.
                    lifted:     chipDragHandler.active || (control._editMode && dragPosition.displaced)

                    readonly property var instrumentValueData:  object
                    readonly property int rowIndex:             index

                    Component.onCompleted: {
                        mainWindow.registerWindowDragExclusion(chip)
                        overlayRig.registerMovable(chip, dragPosition)
                    }

                    Component.onDestruction: overlayRig.unregisterMovable(chip)

                    Behavior on x {
                        enabled: !chipDragHandler.active
                        NumberAnimation { duration: 350; easing.type: Easing.OutBack; easing.overshoot: 2 }
                    }

                    Behavior on y {
                        enabled: !chipDragHandler.active
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
                        // Keyed by the value's own uid: neither a model index (which shifts
                        // when a chip is deleted) nor the fact (two chips may show the same one)
                        // identifies a chip uniquely.
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
                        lifted:  chipDragHandler.active
                    }

                    DragHandler {
                        id:      chipDragHandler
                        enabled: control._editMode

                        onActiveChanged: {
                            if (!active) {
                                dragPosition.commit()
                                overlayRig.resolve(chip)
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

                        onPressAndHold: overlayRig.editMode = true
                    }

                    QGCToolTip {
                        visible:    chipMouseArea.containsMouse
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
            text:       qsTr("+ Add Value")
            onClicked: {
                const column = grid.appendColumn()
                Qt.callLater(function() { overlayRig.resolve(null) })
                editDialogComponent.createObject(mainWindow, { instrumentValueData: column.get(0) }).open()
            }
        }

        // Arms rather than fires: one stray tap used to wipe every position and unhide
        // everything, with no undo, from a button that sat right next to Done. The arm times
        // out so a forgotten tap does not lie in wait.
        OverlayPill {
            id:         resetPill
            objectName: "editModeResetPill"
            text:       armed ? qsTr("Tap again to reset") : qsTr("Reset Layout")
            // Leaving edit mode disarms: an arm left standing meant re-entering within the
            // timeout put a single tap between the user and a wiped layout.
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
