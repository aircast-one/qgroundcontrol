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
import QGroundControl.ScreenTools

// A row that picks one of a fact's enum values. The alternative in a grouped list is an embedded
// combo box, which on macOS draws a system control with its own fill and a saturated chevron
// badge - a different language from every other row it sits next to. A picker is a row that says
// what it is set to and opens a menu.
RowLayout {
    id: _root

    property alias label: _label.text
    property var   fact

    Layout.preferredHeight: ScreenTools.settingsRowHeight
    spacing:                ScreenTools.defaultFontPixelWidth * 2

    readonly property var _qgcPal: QGroundControl.globalPalette

    QGCLabel {
        id:                 _label
        Layout.fillWidth:   true
        elide:              Text.ElideRight
        Layout.alignment:   Qt.AlignVCenter
    }

    QGCLabel {
        id:                 _value
        Layout.alignment:   Qt.AlignVCenter
        text:               _root.fact ? _root.fact.enumStringValue : ""
        color:              _root._qgcPal.colorGrey
    }

    QGCColoredImage {
        Layout.alignment:   Qt.AlignVCenter
        height:             ScreenTools.defaultFontPixelHeight / 2
        width:              height
        source:             "/res/DropArrow.svg"
        color:              _value.color
    }

    TapHandler {
        onTapped: menu.openFrom(_root)
    }

    OverlayPopover {
        id: menu

        Repeater {
            model: _root.fact ? _root.fact.enumStrings : []

            OverlayMenuItem {
                text:      modelData
                checkable: true
                checked:   _root.fact.enumStringValue === modelData
                onClicked: {
                    menu.close()
                    _root.fact.enumStringValue = modelData
                }
            }
        }
    }
}
