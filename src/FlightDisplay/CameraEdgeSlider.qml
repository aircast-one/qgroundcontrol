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

import QGroundControl.Controls
import QGroundControl.Palette
import QGroundControl.ScreenTools

Item {
    id:     _root
    width:  Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * _widthInFontHeights)
    height: ScreenTools.defaultFontPixelHeight * _heightInFontHeights
    layer.enabled: true
    layer.effect:  OverlayShadowEffect { elevated: _root.lifted }

    property string icon:       ""
    property string readout:    ""
    property bool   valueReadout: false
    property real   from:   0
    property real   to:     1
    property real   value:  0

    signal moved(real value)
    signal recenterRequested()
    signal held()

    property bool editing:        false
    property bool lifted:         false
    property bool actionsEnabled: true

    readonly property real  _widthInFontHeights:  2.2
    readonly property real  _heightInFontHeights: 13
    readonly property real  _handleInFontHeights: 1.15
    readonly property real  _trackOpacity:        0.35
    readonly property real  _fillOpacity:         0.9
    readonly property real  _borderOpacity:       0.25
    readonly property real  _panelOpacity:        0.9
    readonly property color _panelColor:          Qt.alpha(qgcPal.overlayBackground, 1)

    QGCPalette { id: qgcPal; colorGroupEnabled: enabled }

    Rectangle {
        anchors.fill:   parent
        radius:         width / 2
        color:          Qt.alpha(_panelColor, _panelOpacity)
        border.color:   Qt.alpha(qgcPal.text, _borderOpacity)
        border.width:   1
    }

    QGCColoredImage {
        id:                         iconImage
        anchors.top:                parent.top
        anchors.topMargin:          ScreenTools.defaultFontPixelHeight / 2
        anchors.horizontalCenter:   parent.horizontalCenter
        source:                     icon
        visible:                    icon !== ""
        color:                      qgcPal.text
        height:                     icon !== "" ? ScreenTools.defaultFontPixelHeight : 0
        width:                      height
        sourceSize.height:          height
        fillMode:                   Image.PreserveAspectFit
        mipmap:                     true
        smooth:                     true
    }

    QGCLabel {
        id:                         readoutLabel
        anchors.top:                iconImage.bottom
        anchors.topMargin:          ScreenTools.defaultFontPixelHeight / 4
        anchors.horizontalCenter:   parent.horizontalCenter
        text:                       slider.pressed && valueReadout ? Math.round(slider.value) : readout
        visible:                    text !== ""
        color:                      qgcPal.text
        font.pointSize:             ScreenTools.smallFontPointSize
    }

    Slider {
        id:                         slider
        enabled:                    !_root.editing && _root.actionsEnabled
        orientation:                Qt.Vertical
        anchors.top:                readoutLabel.visible ? readoutLabel.bottom : iconImage.bottom
        anchors.bottom:             parent.bottom
        anchors.topMargin:          ScreenTools.defaultFontPixelHeight / 2
        anchors.bottomMargin:       ScreenTools.defaultFontPixelHeight / 2
        anchors.horizontalCenter:   parent.horizontalCenter
        from:                       _root.from
        to:                         _root.to
        onMoved:                    _root.moved(value)

        Binding {
            target:         slider
            property:       "value"
            value:          _root.value
            restoreMode:    Binding.RestoreBindingOrValue
        }

        background: Rectangle {
            x:              slider.leftPadding + slider.availableWidth / 2 - width / 2
            y:              slider.topPadding
            implicitWidth:  Math.max(2, ScreenTools.defaultFontPixelWidth / 4)
            width:          implicitWidth
            height:         slider.availableHeight
            radius:         width / 2
            color:          qgcPal.text
            opacity:        _trackOpacity

            Rectangle {
                width:      parent.width
                height:     parent.height * (1 - slider.visualPosition)
                anchors.bottom: parent.bottom
                radius:     parent.radius
                color:      qgcPal.text
                opacity:    _fillOpacity
            }
        }

        handle: Rectangle {
            x:              slider.leftPadding + slider.availableWidth / 2 - width / 2
            y:              slider.topPadding + slider.visualPosition * (slider.availableHeight - height)
            implicitWidth:  ScreenTools.defaultFontPixelHeight * _handleInFontHeights
            implicitHeight: implicitWidth
            radius:         width / 2
            color:          slider.pressed ? qgcPal.colorOrange : qgcPal.text
        }
    }

    TapHandler {
        onDoubleTapped: _root.recenterRequested()
        onLongPressed:  _root.held()
    }
}
