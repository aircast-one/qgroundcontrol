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
import QtQuick.Effects
import QtQuick.Dialogs
import QtQuick.Layouts

import QGroundControl
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.MultiVehicleManager
import QGroundControl.Palette

Item {
    id: root

    default property alias contentItem: mainLayout.data

    readonly property real contentWidth:  mainLayout.implicitWidth
    readonly property real contentHeight: mainLayout.height

    readonly property real _readableMaxWidth: ScreenTools.defaultFontPixelWidth * 120

    function scrollToItem(item) {
        _flickable.contentY = Math.max(0, Math.min(item.mapToItem(mainLayout, 0, 0).y,
                                                     _flickable.contentHeight - _flickable.height))
    }

    readonly property real _fadeHeight: ScreenTools.defaultFontPixelHeight * 1.5

    QGCFlickable {
        id:             _flickable
        objectName:     "settingsFlickable"
        anchors.fill:   parent
        contentWidth:   mainLayout.width
        contentHeight:  mainLayout.height

        readonly property real _topFade:    Math.max(0, Math.min(1, contentY / root._fadeHeight))
        readonly property real _bottomFade: Math.max(0, Math.min(1, (contentHeight - height - contentY) / root._fadeHeight))

        layer.enabled: _topFade > 0 || _bottomFade > 0
        layer.effect: MultiEffect {
            maskEnabled:        true
            maskSource:         scrollEdgeMask
            maskThresholdMin:   0
            maskSpreadAtMin:    1
        }

        ColumnLayout {
            id:         mainLayout
            x:          0
            width:      Math.min(_flickable.width, _readableMaxWidth)
            spacing:    ScreenTools.defaultFontPixelHeight
        }
    }

    Item {
        id:             scrollEdgeMask
        anchors.fill:   _flickable
        visible:        false
        layer.enabled:  true

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop { position: 0;                                     color: Qt.rgba(1, 1, 1, 1 - _flickable._topFade) }
                GradientStop { position: root._fadeHeight / scrollEdgeMask.height;     color: "white" }
                GradientStop { position: 1 - root._fadeHeight / scrollEdgeMask.height; color: "white" }
                GradientStop { position: 1;                                     color: Qt.rgba(1, 1, 1, 1 - _flickable._bottomFade) }
            }
        }
    }
}
