/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl
import QGroundControl.Controls
import QGroundControl.ScreenTools

Item {
    id: _root

    property var options:      []
    property int currentIndex: 0

    property color contentColor: QGroundControl.globalPalette.text

    signal activated(int index)
    signal reselected(int index)

    readonly property var  _qgcPal:      QGroundControl.globalPalette
    readonly property real _pad:         ScreenTools.defaultFontPixelHeight * 0.2
    readonly property real _optionWidth: options.length ? (width - _pad * 2) / options.length : 0
    readonly property real _restX:       _pad + currentIndex * _optionWidth

    readonly property int _thumbIndex: (dragHandler.active && _optionWidth > 0)
                                           ? Math.max(0, Math.min(options.length - 1,
                                                                  Math.round((thumb.x - _pad) / _optionWidth)))
                                           : currentIndex

    implicitHeight: Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * 2.2)
    implicitWidth:  measureRow.implicitWidth + _pad * 2

    Row {
        id:      measureRow
        visible: false

        Repeater {
            model: _root.options

            QGCLabel {
                text:         modelData
                font.bold:    true
                leftPadding:  ScreenTools.defaultFontPixelWidth * 1.6
                rightPadding: ScreenTools.defaultFontPixelWidth * 1.6
            }
        }
    }

    Rectangle {
        id:         thumb
        objectName: "viewSwitchThumb"
        x:          _root._restX
        y:       _root._pad
        width:   _root._optionWidth
        height:  parent.height - _root._pad * 2
        radius:  height / 2
        color:   Qt.alpha(_root.contentColor, dragHandler.active ? 0.26 : 0.18)
        visible: _root.options.length > 0

        Behavior on x {
            enabled: !dragHandler.active
            NumberAnimation { duration: 220; easing.type: Easing.OutCubic }
        }

        Behavior on color { ColorAnimation { duration: 120 } }

        DragHandler {
            id:             dragHandler
            target:         thumb
            yAxis.enabled:  false
            xAxis.minimum:  _root._pad
            xAxis.maximum:  Math.max(_root._pad, _root.width - _root._pad - thumb.width)
            cursorShape:    Qt.PointingHandCursor

            onActiveChanged: {
                if (active) {
                    return
                }
                const landed = _root._thumbIndex
                thumb.x = Qt.binding(() => _root._restX)
                if (landed !== _root.currentIndex) {
                    _root.activated(landed)
                }
            }
        }
    }

    Repeater {
        model: _root.options

        Item {
            objectName: (_root.objectName === "" ? "viewSwitch" : _root.objectName) + "Option" + index
            x:          _root._pad + index * _root._optionWidth
            y:          _root._pad
            width:      _root._optionWidth
            height:     _root.height - _root._pad * 2

            QGCLabel {
                anchors.centerIn: parent
                text:             modelData
                font.bold:        index === _root._thumbIndex
                color:            index === _root._thumbIndex ? _root.contentColor
                                                                : Qt.alpha(_root.contentColor, 0.6)

                Behavior on color { ColorAnimation { duration: 120 } }
            }

            TapHandler {
                onTapped: index === _root.currentIndex ? _root.reselected(index)
                                                       : _root.activated(index)
            }
        }
    }
}
