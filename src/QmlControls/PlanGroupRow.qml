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

Rectangle {
    id: _root

    property string text
    property string description
    property string value
    property bool   showChevron:    false
    property bool   showSeparator:  y > 0
    property bool   interactive:    false
    property bool   current:        false
    property color  textColor:      QGroundControl.globalPalette.text

    default property alias trailingItems: trailingRow.data

    signal clicked()

    readonly property var  _qgcPal: QGroundControl.globalPalette
    readonly property real _hPad:   ScreenTools.defaultFontPixelWidth * 1.5
    readonly property real _vPad:   ScreenTools.defaultFontPixelHeight * 0.45

    width:          parent ? parent.width : 0
    implicitHeight: Math.max(ScreenTools.defaultFontPixelHeight * 2.6,
                             labelColumn.implicitHeight + _vPad * 2,
                             trailingRow.implicitHeight + _vPad * 2)
    color:          current                                  ? Qt.alpha(_qgcPal.colorBlue, 0.22)
                  : interactive && rowMouseArea.containsMouse ? Qt.alpha(_qgcPal.text, 0.05)
                                                              : "transparent"
    opacity:        enabled ? 1 : 0.4

    Behavior on color { ColorAnimation { duration: 100 } }

    Rectangle {
        anchors.top:    parent.top
        anchors.left:   parent.left
        anchors.right:  parent.right
        anchors.leftMargin: _root._hPad
        height:         1
        color:          Qt.alpha(_root._qgcPal.text, 0.09)
        visible:        _root.showSeparator
    }

    Column {
        id:                     labelColumn
        anchors.left:           parent.left
        anchors.leftMargin:     _root._hPad
        anchors.right:          trailingRow.left
        anchors.rightMargin:    ScreenTools.defaultFontPixelWidth
        anchors.verticalCenter: parent.verticalCenter
        spacing:                ScreenTools.defaultFontPixelHeight * 0.1

        QGCLabel {
            width:      parent.width
            text:       _root.text
            color:      _root.textColor
            elide:      Text.ElideRight
            visible:    text !== ""
        }

        QGCLabel {
            width:          parent.width
            text:           _root.description
            color:          Qt.alpha(_root._qgcPal.text, _root.enabled ? 0.5 : 0.9)
            font.pointSize: ScreenTools.smallFontPointSize
            elide:          Text.ElideRight
            visible:        text !== ""
        }
    }

    Row {
        id:                     trailingRow
        anchors.right:          parent.right
        anchors.rightMargin:    _root._hPad
        anchors.verticalCenter: parent.verticalCenter
        spacing:                ScreenTools.defaultFontPixelWidth

        QGCLabel {
            anchors.verticalCenter: parent.verticalCenter
            text:                   _root.value
            color:                  Qt.alpha(_root._qgcPal.text, 0.6)
            font.family:            ScreenTools.fixedFontFamily
            visible:                text !== ""
        }

        QGCLabel {
            anchors.verticalCenter: parent.verticalCenter
            text:                   "›"
            color:                  Qt.alpha(_root._qgcPal.text, 0.4)
            font.pointSize:         ScreenTools.mediumFontPointSize
            visible:                _root.showChevron
        }
    }

    QGCMouseArea {
        id:             rowMouseArea
        anchors.fill:   parent
        hoverEnabled:   !ScreenTools.isMobile
        enabled:        _root.interactive
        onClicked:      _root.clicked()
    }
}
