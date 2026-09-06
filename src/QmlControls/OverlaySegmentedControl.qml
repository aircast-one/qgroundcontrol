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

Item {
    id: _root

    property var  segments:     []
    property int  currentIndex: 0

    property color contentColor: QGroundControl.globalPalette.text
    property bool  glass:        false

    signal activated(int index)

    implicitHeight: ScreenTools.defaultFontPixelHeight * 2.1
    height:         implicitHeight
    layer.enabled:  glass
    layer.effect:   OverlayShadowEffect { }

    readonly property var  _qgcPal:       QGroundControl.globalPalette
    readonly property real _padding:      ScreenTools.defaultFontPixelHeight * 0.11
    readonly property real _trackRadius:  ScreenTools.defaultFontPixelHeight * 0.45
    readonly property real _segmentWidth: segments.length ? (width - _padding * 2) / segments.length : 0

    function _textOf(entry)    { return entry.text !== undefined ? entry.text : entry }
    function _enabledOf(entry) { return entry.enabled !== undefined ? entry.enabled : true }

    Rectangle {
        anchors.fill: parent
        radius:       _root._trackRadius
        visible:      !_root.glass
        color:        Qt.alpha(_root.contentColor, 0.10)
        border.width: 1
        border.color: Qt.alpha(_root.contentColor, 0.08)
    }

    OverlayGlass {
        anchors.fill: parent
        radius:       _root._trackRadius
        visible:      _root.glass
    }

    Rectangle {
        id:      thumb
        x:       _root._padding + _root.currentIndex * _root._segmentWidth
        y:       _root._padding
        width:   _root._segmentWidth
        height:  parent.height - _root._padding * 2
        radius:  _root._trackRadius - _root._padding
        visible: _root.segments.length > 0
        color:   Qt.alpha(_root.contentColor, 0.14)

        Behavior on x { NumberAnimation { duration: 150; easing.type: Easing.OutCubic } }
    }

    Row {
        anchors.fill:         parent
        anchors.margins:      _root._padding
        anchors.leftMargin:   _root._padding
        anchors.rightMargin:  _root._padding

        Repeater {
            model: _root.segments

            Item {
                objectName: "segment" + index
                width:      _root._segmentWidth
                height:     parent.height

                readonly property bool _isCurrent: index === _root.currentIndex
                readonly property bool _isEnabled: _root._enabledOf(modelData)

                QGCLabel {
                    anchors.centerIn:    parent
                    width:               parent.width - ScreenTools.defaultFontPixelWidth
                    horizontalAlignment: Text.AlignHCenter
                    elide:               Text.ElideRight
                    text:                _root._textOf(modelData)
                    font.bold:           parent._isCurrent
                    color:               !parent._isEnabled ? Qt.alpha(_root.contentColor, 0.3)
                                       : parent._isCurrent  ? _root.contentColor
                                                            : Qt.alpha(_root.contentColor, 0.6)

                    Behavior on color { ColorAnimation { duration: 120 } }
                }

                QGCMouseArea {
                    anchors.fill: parent
                    enabled:      parent._isEnabled
                    // Only reports. Assigning currentIndex here would overwrite whatever binding
                    // the owner set it from, so the control would stop following its own model.
                    onClicked: _root.activated(index)
                }
            }
        }
    }
}
