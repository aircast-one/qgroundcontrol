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
import QGroundControl.Palette
import QGroundControl.ScreenTools

Button {
    id:             control
    padding:        ScreenTools.defaultFontPixelWidth * 0.75
    hoverEnabled:   !ScreenTools.isMobile
    autoExclusive:  true
    icon.color:     textColor

    property color textColor:    checked || pressed ? qgcPal.buttonHighlightText : qgcPal.buttonText
    property color tileColor:    "transparent"
    property bool  badgeVisible: false

    readonly property bool _hasTile: tileColor.a > 0
    readonly property real _tileSize: Math.round(ScreenTools.defaultFontPixelHeight * 1.35)

    QGCPalette {
        id:                 qgcPal
        colorGroupEnabled:  control.enabled
    }

    background: Rectangle {
        color:      qgcPal.buttonHighlight
        opacity:    checked || pressed ? 1 : enabled && hovered ? .12 : 0
        radius:     ScreenTools.buttonBorderRadius
    }

    contentItem: RowLayout {
        spacing: ScreenTools.defaultFontPixelWidth

        Rectangle {
            Layout.preferredWidth:  _hasTile ? _tileSize : ScreenTools.defaultFontPixelHeight
            Layout.preferredHeight: Layout.preferredWidth
            radius:                 Math.round(_tileSize * 0.28)
            color:                  control.tileColor

            QGCColoredImage {
                anchors.centerIn:   parent
                source:             control.icon.source
                color:              _hasTile ? "white" : control.icon.color
                width:              _hasTile ? Math.round(_tileSize * 0.68) : parent.width
                height:             width
                sourceSize.height:  height
                fillMode:           Image.PreserveAspectFit
            }
        }

        QGCLabel {
            id:                     displayText
            Layout.fillWidth:       true
            text:                   control.text
            color:                  control.textColor
            horizontalAlignment:    QGCLabel.AlignLeft
        }

        Rectangle {
            Layout.preferredWidth:  Math.round(ScreenTools.defaultFontPixelHeight * 0.55)
            Layout.preferredHeight: Layout.preferredWidth
            radius:                 width / 2
            color:                  qgcPal.colorRed
            visible:                control.badgeVisible
        }
    }
}
