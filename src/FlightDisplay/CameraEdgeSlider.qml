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

    property bool vertical: true

    readonly property real _shortSide: Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * _widthInFontHeights)
    readonly property real _longSide:  ScreenTools.defaultFontPixelHeight * _heightInFontHeights

    width:  vertical ? _shortSide : _longSide
    height: vertical ? _longSide  : _shortSide
    layer.enabled: true
    layer.effect:  OverlayShadowEffect { elevated: _root.lifted }

    property string icon:       ""
    property string readout:    ""
    property bool   valueReadout: false
    property real   from:   0
    property real   to:     1
    property real   value:  0
    property bool   centered: false

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

    QGCPalette { id: qgcPal; colorGroupEnabled: _root.enabled }

    OverlayGlass {
        anchors.fill: parent
        radius:       Math.min(_root.width, _root.height) / 2
    }

    // Icon and readout stay upright in both orientations - only the track rotates.
    QGCColoredImage {
        id:                         iconImage
        anchors.top:                _root.vertical ? parent.top : undefined
        anchors.topMargin:          _root.vertical ? ScreenTools.defaultFontPixelHeight / 2 : 0
        anchors.left:               _root.vertical ? undefined : parent.left
        anchors.leftMargin:         _root.vertical ? 0 : ScreenTools.defaultFontPixelHeight / 2
        anchors.horizontalCenter:   _root.vertical ? parent.horizontalCenter : undefined
        anchors.verticalCenter:     _root.vertical ? undefined : parent.verticalCenter
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
        anchors.top:                _root.vertical ? iconImage.bottom : undefined
        anchors.topMargin:          _root.vertical ? ScreenTools.defaultFontPixelHeight / 4 : 0
        anchors.horizontalCenter:   _root.vertical ? parent.horizontalCenter : undefined
        anchors.left:               _root.vertical ? undefined : iconImage.right
        anchors.leftMargin:         _root.vertical ? 0 : ScreenTools.defaultFontPixelHeight / 4
        anchors.verticalCenter:     _root.vertical ? undefined : parent.verticalCenter
        text:                       slider.pressed && valueReadout ? Math.round(slider.value) : readout
        visible:                    text !== ""
        color:                      qgcPal.text
        font.pointSize:             ScreenTools.smallFontPointSize
    }

    Slider {
        id:                         slider
        enabled:                    !_root.editing && _root.actionsEnabled
        orientation:                _root.vertical ? Qt.Vertical : Qt.Horizontal
        anchors.top:                _root.vertical ? (readoutLabel.visible ? readoutLabel.bottom : iconImage.bottom) : parent.top
        anchors.bottom:             parent.bottom
        anchors.left:               _root.vertical ? undefined : (readoutLabel.visible ? readoutLabel.right : iconImage.right)
        anchors.right:              _root.vertical ? undefined : parent.right
        anchors.topMargin:          _root.vertical ? ScreenTools.defaultFontPixelHeight / 2 : 0
        anchors.bottomMargin:       ScreenTools.defaultFontPixelHeight / 2
        anchors.leftMargin:         _root.vertical ? 0 : ScreenTools.defaultFontPixelHeight / 4
        anchors.rightMargin:        _root.vertical ? 0 : ScreenTools.defaultFontPixelHeight / 2
        anchors.horizontalCenter:   _root.vertical ? parent.horizontalCenter : undefined
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
            x:              _root.vertical ? slider.leftPadding + slider.availableWidth / 2 - width / 2 : slider.leftPadding
            y:              _root.vertical ? slider.topPadding : slider.topPadding + slider.availableHeight / 2 - height / 2
            implicitWidth:  _root.vertical ? Math.max(2, ScreenTools.defaultFontPixelWidth / 4) : slider.availableWidth
            implicitHeight: _root.vertical ? slider.availableHeight : Math.max(2, ScreenTools.defaultFontPixelWidth / 4)
            width:          implicitWidth
            height:         implicitHeight
            radius:         Math.min(width, height) / 2
            color:          qgcPal.text
            opacity:        _trackOpacity

            Rectangle {
                width:      _root.vertical ? parent.width : (_root.centered ? Math.abs(parent.width / 2 - parent.width * slider.visualPosition) : parent.width * slider.visualPosition)
                height:     _root.vertical ? (_root.centered ? Math.abs(parent.height / 2 - parent.height * slider.visualPosition) : parent.height * (1 - slider.visualPosition)) : parent.height
                x:          _root.vertical ? 0 : (_root.centered ? Math.min(parent.width / 2, parent.width * slider.visualPosition) : 0)
                y:          _root.vertical ? (_root.centered ? Math.min(parent.height / 2, parent.height * slider.visualPosition) : parent.height * slider.visualPosition) : 0
                radius:     parent.radius
                color:      qgcPal.text
                opacity:    _fillOpacity
            }

            Rectangle {
                x:          _root.vertical ? (parent.width - width) / 2 : parent.width / 2 - width / 2
                y:          _root.vertical ? parent.height / 2 - height / 2 : (parent.height - height) / 2
                width:      _root.vertical ? ScreenTools.defaultFontPixelHeight * 0.6 : Math.max(2, ScreenTools.defaultFontPixelWidth / 4)
                height:     _root.vertical ? Math.max(2, ScreenTools.defaultFontPixelWidth / 4) : ScreenTools.defaultFontPixelHeight * 0.6
                radius:     Math.min(width, height) / 2
                color:      qgcPal.text
                opacity:    0.9
                visible:    _root.centered
            }
        }

        handle: Rectangle {
            x:              _root.vertical ? slider.leftPadding + slider.availableWidth / 2 - width / 2 : slider.leftPadding + slider.visualPosition * (slider.availableWidth - width)
            y:              _root.vertical ? slider.topPadding + slider.visualPosition * (slider.availableHeight - height) : slider.topPadding + slider.availableHeight / 2 - height / 2
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
