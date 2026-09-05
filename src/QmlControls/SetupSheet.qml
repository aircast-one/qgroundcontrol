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
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.ScreenTools

Item {
    id:             _root
    parent:         Overlay.overlay
    anchors.fill:   parent
    opacity:        open ? 1 : 0
    visible:        opacity > 0
    z:              QGroundControl.zOrderTopMost + 1

    Behavior on opacity { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }

    property bool   open:  false
    property string title

    default property alias content: contentColumn.data
    property alias         footer:  footerRow.data

    readonly property var  _qgcPal: QGroundControl.globalPalette
    readonly property real _fh:     ScreenTools.defaultFontPixelHeight
    readonly property real _pad:    _fh * 1.2

    DeadMouseArea {
        anchors.fill: parent
    }

    Rectangle {
        anchors.fill:   parent
        color:          Qt.rgba(0, 0, 0, 0.45)
    }

    Rectangle {
        id:                 sheet
        anchors.centerIn:   parent
        width:              Math.min(ScreenTools.defaultFontPixelWidth * 78, parent.width - _pad * 2)
        height:             Math.min(sheetLayout.implicitHeight + _pad * 2, parent.height - _pad * 2)
        radius:             _fh * 1.4
        color:              _root._qgcPal.window
        border.width:       1
        border.color:       Qt.alpha(_root._qgcPal.text, 0.1)
        scale:              _root.open ? 1 : 0.96

        Behavior on scale { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }

        ColumnLayout {
            id:                 sheetLayout
            anchors.fill:       parent
            anchors.margins:    _pad
            spacing:            _fh * 0.8

            QGCLabel {
                Layout.fillWidth:   true
                text:               _root.title
                font.pointSize:     ScreenTools.largeFontPointSize
                font.bold:          true
                elide:              Text.ElideRight
                visible:            text !== ""
            }

            ColumnLayout {
                id:                 contentColumn
                Layout.fillWidth:   true
                Layout.fillHeight:  true
                spacing:            _fh * 0.6
            }

            RowLayout {
                id:                 footerRow
                Layout.fillWidth:   true
                layoutDirection:    Qt.RightToLeft
                spacing:            ScreenTools.defaultFontPixelWidth
            }
        }
    }
}
