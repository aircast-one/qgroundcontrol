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
    autoExclusive:  !listStyle
    icon.color:     textColor
    implicitHeight: Math.max(implicitContentHeight + topPadding + bottomPadding,
                             listStyle ? ScreenTools.minTouchPixels : 0)

    property color  textColor:    checked ? qgcPal.buttonHighlightText : qgcPal.buttonText
    property color  tileColor:    "transparent"
    property bool   badgeVisible: false
    property string description:  ""
    property bool   listStyle:    false

    readonly property bool _hasTile: tileColor.a > 0 && !listStyle
    readonly property real _tileSize: Math.round(ScreenTools.defaultFontPixelHeight * 1.35)

    QGCPalette {
        id:                 qgcPal
        colorGroupEnabled:  control.enabled
    }

    onPressedChanged: {
        if (pressed && listStyle) {
            rippleAnimation.restart()
        }
    }

    background: Rectangle {
        color:  control.checked ? qgcPal.buttonHighlight
                                : Qt.alpha(qgcPal.text, !control.enabled ? 0
                                                      : control.pressed ? 0.14
                                                      : control.hovered ? 0.07 : 0)
        radius: control.listStyle ? 0 : ScreenTools.buttonBorderRadius * 1.6
        clip:   control.listStyle

        PointHandler {
            id:         pressPoint
            enabled:    control.listStyle
        }

        Rectangle {
            id:         ripple
            x:          pressPoint.point.position.x - width / 2
            y:          pressPoint.point.position.y - height / 2
            width:      0
            height:     width
            radius:     width / 2
            color:      Qt.alpha(qgcPal.text, 0.16)
            opacity:    0
            visible:    control.listStyle
        }

        ParallelAnimation {
            id: rippleAnimation

            NumberAnimation {
                target:     ripple
                property:   "width"
                from:       0
                to:         control.width * 2
                duration:   450
                easing.type: Easing.OutCubic
            }

            NumberAnimation {
                target:     ripple
                property:   "opacity"
                from:       1
                to:         0
                duration:   450
            }
        }
    }

    contentItem: RowLayout {
        spacing: ScreenTools.defaultFontPixelWidth

        Rectangle {
            Layout.preferredWidth:  _hasTile ? _tileSize : ScreenTools.defaultFontPixelHeight
            Layout.preferredHeight: Layout.preferredWidth
            radius:                 Math.round(_tileSize * 0.28)
            color:                  _hasTile ? control.tileColor : "transparent"

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

        ColumnLayout {
            Layout.fillWidth:   true
            spacing:            0

            QGCLabel {
                Layout.fillWidth:       true
                text:                   control.text
                color:                  control.textColor
                elide:                  Text.ElideRight
                horizontalAlignment:    QGCLabel.AlignLeft
            }

            QGCLabel {
                Layout.fillWidth:       true
                visible:                control.description !== ""
                text:                   control.description
                color:                  Qt.alpha(control.textColor, 0.6)
                font.pointSize:         ScreenTools.smallFontPointSize
                elide:                  Text.ElideRight
                horizontalAlignment:    QGCLabel.AlignLeft
            }
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
