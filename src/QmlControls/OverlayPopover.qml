import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls

Popup {
    id:          _root
    padding:     ScreenTools.defaultFontPixelHeight * 0.3
    modal:       false
    focus:       true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    default property alias menuItems: menuColumn.data

    readonly property real _radius: ScreenTools.defaultFontPixelHeight * 0.8
    readonly property real _gap:    ScreenTools.defaultFontPixelWidth

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

        OverlaySurface {
            anchors.fill: parent
            radius:       parent.radius
            material:     OverlaySurface.Panel
        }
    }

    contentItem: ColumnLayout {
        id:      menuColumn
        spacing: 0
    }
}
