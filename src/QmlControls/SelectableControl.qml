/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Controls

import QGroundControl.FactSystem

Control {
    id:             control
    topInset:       0
    bottomInset:    0
    leftInset:      0
    rightInset:     0
    topPadding:     0
    bottomPadding:  0
    leftPadding:    0
    rightPadding:   0

    property Fact selectedControl               ///< Fact which has enumStrings/Values where values are the qml file for the control
    property var  innerControl:           loader.item
    property var  overlayRig:             null

    readonly property bool _editMode: overlayRig ? overlayRig.editMode : false

    function _cycleControl() {
        const values = selectedControl.enumValues
        const current = values.findIndex((value) => String(value) === String(selectedControl.rawValue))
        selectedControl.rawValue = values[(current + 1) % values.length]
    }

    contentItem: Item {
        implicitWidth:  loader.item.width
        implicitHeight: loader.item.height

        Loader {
            id:     loader
            source: selectedControl ? selectedControl.rawValue : ""
        }

        QGCMouseArea {
            anchors.fill:       parent
            acceptedButtons:    Qt.LeftButton | Qt.RightButton

            onClicked: (mouse) => {
                if (mouse.button === Qt.RightButton) {
                    if (overlayRig) {
                        overlayRig.editMode = !overlayRig.editMode
                    }
                } else if (control._editMode) {
                    control._cycleControl()
                }
            }

            onPressAndHold: {
                if (overlayRig) {
                    overlayRig.hold(control)
                }
            }
        }
    }
}
