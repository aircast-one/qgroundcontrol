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
import QGroundControl.ScreenTools

ProgressBar {
    id: control

    readonly property var _qgcPal: QGroundControl.globalPalette

    background: Rectangle {
        implicitHeight: ScreenTools.defaultFontPixelHeight * 0.4
        radius:         height / 2
        color:          Qt.alpha(control._qgcPal.text, 0.1)
    }

    contentItem: Item {
        Rectangle {
            width:  parent.width * control.visualPosition
            height: parent.height
            radius: height / 2
            color:  control._qgcPal.colorBlue

            Behavior on width { NumberAnimation { duration: 120 } }
        }
    }
}
