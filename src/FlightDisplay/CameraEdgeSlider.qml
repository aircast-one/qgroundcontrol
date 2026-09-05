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
    id: _root

    property bool vertical: true

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

    readonly property real _widthInFontHeights:  3.4
    readonly property real _heightInFontHeights: 11.5
    readonly property real _fillOpacity:         0.9

    readonly property real _shortSide: Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * _widthInFontHeights)
    readonly property real _longSide:  ScreenTools.defaultFontPixelHeight * _heightInFontHeights

    width:  vertical ? _shortSide : _longSide
    height: vertical ? _longSide  : _shortSide

    layer.enabled: true
    layer.effect:  OverlayShadowEffect { elevated: _root.lifted }

    scale: slider.pressed ? 1.03 : 1.0
    Behavior on scale { NumberAnimation { duration: 120; easing.type: Easing.OutQuint } }

    readonly property real _radius: _shortSide / 2

    QGCPalette { id: qgcPal; colorGroupEnabled: _root.enabled }

    readonly property real _valueFrac: (slider.to === slider.from)
                                           ? 0
                                           : Math.max(0, Math.min(1, (slider.value - slider.from) / (slider.to - slider.from)))
    readonly property real _fillLo: centered ? Math.min(0.5, _valueFrac) : 0
    readonly property real _fillHi: centered ? Math.max(0.5, _valueFrac) : _valueFrac

    readonly property real _axisLen:   vertical ? height : width
    readonly property real _fillLen:   _axisLen * (_fillHi - _fillLo)
    readonly property real _fillStart: vertical ? height * (1 - _fillHi) : width * _fillLo

    OverlayGlass {
        objectName:   "edgeSliderGlass"
        anchors.fill: parent
        radius:       _root._radius
        minTint:      0.3
        maxTint:      0.6
    }

    readonly property real _fillThick: centered ? _shortSide * 0.2 : _shortSide

    Rectangle {
        id:      fill
        x:       _root.vertical ? (_root.width - _root._fillThick) / 2 : _root._fillStart
        y:       _root.vertical ? _root._fillStart : (_root.height - _root._fillThick) / 2
        width:   _root.vertical ? _root._fillThick : _root._fillLen
        height:  _root.vertical ? _root._fillLen : _root._fillThick
        radius:  _root.centered ? _root._fillThick / 2 : _root._radius
        color:   _root.centered ? qgcPal.colorBlue : qgcPal.text
        opacity: slider.pressed ? 1.0 : _root._fillOpacity
        visible: _root._fillLen > 0.5

        Behavior on opacity { NumberAnimation { duration: 120 } }

        Rectangle {
            readonly property real _patch: _root.vertical
                                               ? Math.min(_root._radius, (_root.height - _root._radius) - _root._fillStart)
                                               : Math.min(_root._radius, _root._fillLen - _root._radius)
            visible: !_root.centered && _patch > 0 &&
                     (_root.vertical ? _root._fillStart >= _root._radius
                                     : _root._fillLen <= _root.width - _root._radius)
            x:       _root.vertical ? 0 : parent.width - _patch
            y:       0
            width:   _root.vertical ? parent.width : _patch
            height:  _root.vertical ? _patch : parent.height
            color:   parent.color
        }
    }

    Rectangle {
        anchors.centerIn: parent
        width:   _root.vertical ? _root.width * 0.42 : Math.max(2, ScreenTools.defaultFontPixelWidth / 4)
        height:  _root.vertical ? Math.max(2, ScreenTools.defaultFontPixelWidth / 4) : _root.height * 0.42
        radius:  Math.min(width, height) / 2
        color:   qgcPal.text
        opacity: 0.55
        visible: _root.centered
    }

    readonly property real _labelZone: ScreenTools.defaultFontPixelHeight * 2.2
    readonly property real _labelFrac: (_labelZone / 2) / Math.max(1, _axisLen)
    readonly property bool _labelOnFill: !centered && _fillLen > 0.5 && _fillLo <= _labelFrac && _labelFrac <= _fillHi
    readonly property color _labelColor: _labelOnFill ? qgcPal.window : qgcPal.text

    Item {
        id:         labelZone
        objectName: "edgeSliderLabelZone"
        width:  _root.vertical ? _root.width : _root._labelZone
        height: _root.vertical ? _root._labelZone : _root.height
        anchors.bottom: _root.vertical ? parent.bottom : undefined
        anchors.left:   _root.vertical ? undefined : parent.left
        anchors.horizontalCenter: _root.vertical ? parent.horizontalCenter : undefined
        anchors.verticalCenter:   _root.vertical ? undefined : parent.verticalCenter

        QGCColoredImage {
            id:                       iconImage
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom:           readoutLabel.visible ? readoutLabel.top : parent.bottom
            anchors.bottomMargin:     readoutLabel.visible ? ScreenTools.defaultFontPixelHeight * 0.1
                                                           : ScreenTools.defaultFontPixelHeight * 0.4
            source:                   _root.icon
            visible:                  _root.icon !== ""
            color:                    _root._labelColor
            height:                   _root.icon !== "" ? ScreenTools.defaultFontPixelHeight * 0.85 : 0
            width:                    height
            sourceSize.height:        height
            fillMode:                 Image.PreserveAspectFit
            mipmap:                   true
            smooth:                   true
        }

        QGCLabel {
            id:                       readoutLabel
            objectName:               "edgeSliderReadout"
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom:           parent.bottom
            anchors.bottomMargin:     ScreenTools.defaultFontPixelHeight * 0.35
            text:                     slider.pressed && _root.valueReadout ? Math.round(slider.value) : _root.readout
            visible:                  text !== ""
            color:                    _root._labelColor
            opacity:                  slider.pressed ? 1 : 0.7
            font.pointSize:           ScreenTools.smallFontPointSize
        }
    }

    Slider {
        id:          slider
        anchors.fill: parent
        padding:     0
        enabled:     !_root.editing && _root.actionsEnabled
        orientation: _root.vertical ? Qt.Vertical : Qt.Horizontal
        from:        _root.from
        to:          _root.to
        onMoved:     _root.moved(value)

        background: Item {}
        handle:     Item { implicitWidth: 0; implicitHeight: 0 }

        Binding {
            target:         slider
            property:       "value"
            value:          _root.value
            restoreMode:    Binding.RestoreBindingOrValue
        }
    }

    TapHandler {
        onDoubleTapped: _root.recenterRequested()
        onLongPressed:  _root.held()
    }
}
