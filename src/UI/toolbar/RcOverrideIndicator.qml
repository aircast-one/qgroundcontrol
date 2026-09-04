/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.Palette
import QGroundControl.ScreenTools

// Bypassing the pilot's transmitter is the highest-consequence state the on-screen RC
// controls can create, so it is reported where flight-critical state already lives.
RowLayout {
    anchors.verticalCenter: parent.verticalCenter
    spacing:                ScreenTools.defaultFontPixelWidth / 2

    property bool showIndicator: _overriding

    property var  _activeVehicle: QGroundControl.multiVehicleManager.activeVehicle
    property bool _overriding:    _activeVehicle && _activeVehicle.rcChannelOverrideActive

    QGCPalette { id: qgcPal; colorGroupEnabled: enabled }

    Rectangle {
        Layout.alignment:   Qt.AlignVCenter
        width:              ScreenTools.defaultFontPixelHeight * 0.6
        height:             width
        radius:             width / 2
        color:              qgcPal.colorOrange

        SequentialAnimation on opacity {
            running:    _overriding
            loops:      Animation.Infinite
            NumberAnimation { from: 1;   to: 0.3; duration: 700 }
            NumberAnimation { from: 0.3; to: 1;   duration: 700 }
        }
    }

    // An icon rather than a word: the toolbar is read at a glance in flight, and every other
    // indicator beside it is a glyph.
    QGCColoredImage {
        Layout.alignment:   Qt.AlignVCenter
        source:             "/qmlimages/RcOverride.svg"
        color:              qgcPal.colorOrange
        height:             ScreenTools.defaultFontPixelHeight * 1.4
        width:              height
        sourceSize.height:  height
        fillMode:           Image.PreserveAspectFit
        mipmap:             true
        smooth:             true
    }

    // A TapHandler rather than a MouseArea: a MouseArea filling its parent would be anchored
    // inside this layout, which Qt refuses to manage and which silently breaks the row.
    TapHandler {
        onTapped: if (_activeVehicle) _activeVehicle.clearRcChannelOverrides()
    }
}
