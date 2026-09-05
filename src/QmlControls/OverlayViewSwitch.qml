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

// One thumb that moves, not a highlight that jumps between two boxes. The switch used to be a
// pair of segments each drawing its own fill, so changing view swapped which box was lit - there
// was nothing continuous to grab, and nothing that read as a physical control.
Item {
    id: _root

    // Entries are either a plain string or { text, enabled } - a layer that the vehicle does not
    // support is still shown, so the control keeps its shape, but cannot be moved onto.
    property var options:      []
    property int currentIndex: 0

    signal activated(int index)

    readonly property var  _qgcPal:      QGroundControl.globalPalette
    readonly property real _pad:         ScreenTools.defaultFontPixelHeight * 0.2
    readonly property real _optionWidth: options.length ? (width - _pad * 2) / options.length : 0
    readonly property real _restX:       _pad + currentIndex * _optionWidth

    function _textOf(entry)    { return entry.text !== undefined ? entry.text : entry }
    function _enabledOf(entry) { return entry.enabled !== undefined ? entry.enabled : true }

    // Where the thumb is now, not where it is headed. During a drag this leads currentIndex, so
    // the labels take emphasis as the thumb crosses them rather than after it lands.
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
                text:         _root._textOf(modelData)
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
        color:   Qt.alpha(_root._qgcPal.text, dragHandler.active ? 0.26 : 0.18)
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

            // Dragging assigns x directly, which replaces the binding that parks the thumb on the
            // current option. Handing that binding back on release is what makes it spring home
            // when the drag did not change anything.
            onActiveChanged: {
                if (active) {
                    return
                }
                const landed = _root._thumbIndex
                thumb.x = Qt.binding(() => _root._restX)
                if (landed !== _root.currentIndex && _root._enabledOf(_root.options[landed])) {
                    _root.activated(landed)
                }
            }
        }
    }

    Repeater {
        model: _root.options

        Item {
            // Derived from the control, not fixed: two switches on screen would otherwise both
            // answer to the same names and a lookup would find whichever came first.
            objectName: (_root.objectName === "" ? "viewSwitch" : _root.objectName) + "Option" + index
            x:          _root._pad + index * _root._optionWidth
            y:          _root._pad
            width:      _root._optionWidth
            height:     _root.height - _root._pad * 2

            readonly property bool _isEnabled: _root._enabledOf(modelData)

            QGCLabel {
                anchors.centerIn: parent
                text:             _root._textOf(modelData)
                font.bold:        index === _root._thumbIndex
                color:            !parent._isEnabled          ? Qt.alpha(_root._qgcPal.text, 0.3)
                                : index === _root._thumbIndex ? _root._qgcPal.text
                                                              : Qt.alpha(_root._qgcPal.text, 0.6)

                Behavior on color { ColorAnimation { duration: 120 } }
            }

            TapHandler {
                enabled:  parent._isEnabled
                onTapped: if (index !== _root.currentIndex) _root.activated(index)
            }
        }
    }
}
