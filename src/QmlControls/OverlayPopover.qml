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

// A menu panel that drops beside the item it was opened from. The overlay material is opaque
// here rather than refracting: a menu is read, not glanced at, and the map moving underneath a
// lensed panel makes a list of words hard to hold still.
Popup {
    id:          _root
    padding:     ScreenTools.defaultFontPixelHeight * 0.3
    modal:       false
    focus:       true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    default property alias menuItems: menuColumn.data

    readonly property real _radius: ScreenTools.defaultFontPixelHeight * 0.8
    readonly property real _gap:    ScreenTools.defaultFontPixelWidth

    // Opens to the right of the anchor, or to its left when the panel would leave the window.
    // Vertically centred on the anchor and then pushed back inside the window.
    function openFrom(item) {
        parent = item

        const wantX  = item.width + _gap
        const rightX = item.mapToItem(null, wantX, 0).x
        x = rightX + width > mainWindow.width - _gap ? -width - _gap : wantX

        const wantY   = (item.height - height) / 2
        const topY    = item.mapToItem(null, 0, wantY).y
        const minTop  = ScreenTools.defaultFontPixelHeight
        const maxTop  = mainWindow.height - height - ScreenTools.defaultFontPixelHeight
        y = wantY + Math.max(0, minTop - topY) - Math.max(0, topY - maxTop)

        open()
    }

    background: Rectangle {
        radius:        _root._radius
        color:         "transparent"
        layer.enabled: true
        layer.effect:  OverlayShadowEffect { elevated: true }

        OverlayGlass {
            anchors.fill: parent
            radius:       parent.radius
            material:     OverlayGlass.Panel
        }
    }

    contentItem: ColumnLayout {
        id:      menuColumn
        spacing: 0
    }
}
