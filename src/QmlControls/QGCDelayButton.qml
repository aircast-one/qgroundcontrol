import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls

DelayButton {
    id:             control
    hoverEnabled:   !ScreenTools.isMobile
    topPadding:     _verticalPadding
    bottomPadding:  _verticalPadding
    leftPadding:    _horizontalPadding + _chevronSize
    rightPadding:   _horizontalPadding
    focusPolicy:    Qt.ClickFocus
    font.family:    ScreenTools.normalFontFamily
    delay:          defaultDelay

    property bool   showBorder:     qgcPal.globalTheme === QGCPalette.Light
    property real   backRadius:     ScreenTools.defaultBorderRadius
    property real   heightFactor:   0.5
    property real   fontWeight:     Font.Normal
    property real   pointSize:      ScreenTools.defaultFontPointSize
    property int    defaultDelay:   500

    property alias wrapMode:            text.wrapMode
    property alias horizontalAlignment: text.horizontalAlignment
    property alias backgroundColor:     backRect.color
    property alias textColor:           text.color

    property bool   _showHighlight:     enabled && pressed
    property int    _horizontalPadding: ScreenTools.defaultFontPixelWidth * 2
    property real   _chevronSize:       ScreenTools.defaultFontPixelHeight
    property int    _verticalPadding:   Math.round(ScreenTools.defaultFontPixelHeight * heightFactor)
    property bool   _showHelp:          false
    property bool   _activated:         false

    QGCPalette { id: qgcPal; colorGroupEnabled: enabled }

    Timer {
        id:         helpTimeout
        interval:   3000
        repeat:     false
        onTriggered: control._showHelp = false
    }

    onActivated: {
        _activated = true
        _showHelp = false
    }
    onPressed: {
        _activated = false
    }
    onReleased: {
        _showHelp = !_activated
        _activated = false
        if (_showHelp) {
            helpTimeout.start()
        } else {
            helpTimeout.stop()
        }
    }

    background: Rectangle {
        id:             backRect
        radius:         backRadius
        implicitWidth:  Math.max(control._showHelp ? helpText.contentWidth : 0, ScreenTools.implicitButtonWidth)
        implicitHeight: ScreenTools.implicitButtonHeight
        border.width:   showBorder ? 1 : 0
        border.color:   qgcPal.buttonBorder
        color:          qgcPal.button

        Rectangle {
            anchors.left:   parent.left
            anchors.top:    parent.top
            anchors.bottom: parent.bottom
            width:          parent.width * control.progress
            radius:         parent.radius
            color:          qgcPal.buttonHighlight
            visible:        control.progress > 0
        }

        QGCColoredImage {
            anchors.left:           parent.left
            anchors.leftMargin:     control._horizontalPadding / 2
            anchors.verticalCenter: parent.verticalCenter
            width:                  control._chevronSize
            height:                 control._chevronSize
            source:                 "qrc:/res/chevron-double-right.svg"
            sourceSize.height:      height
            fillMode:               Image.PreserveAspectFit
            color:                  control._showHighlight ? qgcPal.buttonHighlightText : qgcPal.buttonText
            opacity:                control._showHighlight ? 1 : 0.6
        }

        QGCLabel {
            id:                         helpText
            text:                       qsTr("Hold to Confirm")
            anchors.bottom:             parent.bottom
            anchors.horizontalCenter:   parent.horizontalCenter
            font.pointSize:             ScreenTools.smallFontPointSize
            color:                      control._showHighlight ? qgcPal.buttonHighlightText : qgcPal.buttonText
            visible:                    control._showHelp
        }
    }

    contentItem: QGCLabel {
        id:                     text
        horizontalAlignment:    Text.AlignHCenter
        text:                   control.text
        font.pointSize:         control.pointSize
        font.family:            control.font.family
        font.weight:            control.fontWeight
        color:                  control._showHighlight ? qgcPal.buttonHighlightText : qgcPal.buttonText
    }
}
