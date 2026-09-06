import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Palette

ColumnLayout {
    id:                 control    
    spacing:            _margins / 2
    implicitWidth:      _contentLayout.implicitWidth + (_margins * 2)
    implicitHeight:     _contentLayout.implicitHeight + (_margins * 2)

    default property alias contentItem: _contentLayout.data

    property alias contentSpacing: _contentLayout.spacing

    property string defaultBorderColor  : "transparent"
    property string outerBorderColor    : defaultBorderColor

    property string heading
    property string description
    property bool   showDividers:       true
    property bool   showBorder:         !popoverStyle

    property bool   popoverStyle:       false

    property bool   cardStyle:          false

    readonly property bool _inset: showBorder || cardStyle

    property real _margins: ScreenTools.defaultFontPixelHeight / 2

    QGCLabel {
        Layout.leftMargin:  _margins
        Layout.topMargin:   _margins
        text:               popoverStyle ? heading.toUpperCase() : heading
        font.pointSize:     popoverStyle ? ScreenTools.smallFontPointSize
                                         : ScreenTools.defaultFontPointSize + 1
        font.bold:          !popoverStyle
        font.letterSpacing: popoverStyle ? 0.5 : 0
        color:              popoverStyle ? QGroundControl.globalPalette.colorGrey
                                         : QGroundControl.globalPalette.text
        visible:            heading !== ""
    }

    Rectangle {
        id:                 outerRect
        Layout.fillWidth:   true
        implicitWidth:      _contentLayout.implicitWidth + (control._inset ? _margins * 2 : 0)
        implicitHeight:     _contentLayout.implicitHeight + (control._inset ? _margins * 2: 0)
        color:              cardStyle  ? QGroundControl.globalPalette.overlayCard
                          : showBorder ? QGroundControl.globalPalette.windowShade
                                       : "transparent"
        border.color:       outerBorderColor
        border.width:       showBorder && !cardStyle ? 1 : 0
        radius:             Math.round(ScreenTools.defaultFontPixelHeight * 0.85)

        Repeater {
            model: showDividers? _contentLayout.children.length : 0

            Rectangle {
                x:                  control._inset ? _margins : 0
                y:                  _contentItem.y + _contentItem.height + (_contentLayout.spacing / 2) + (control._inset ? _margins : 0)
                width:              parent.width - x
                height:             1
                color:              QGroundControl.globalPalette.groupBorder
                visible:            _contentItem.visible && 
                                        _contentItem.width !== 0 && _contentItem.height !== 0 &&
                                        index < _contentLayout.children.length - 1

                property var _contentItem: _contentLayout.children[index]
            }
        }
 
        ColumnLayout {
            id:                 _contentLayout
            x:                  control._inset ? _margins : 0
            y:                  control._inset ? _margins : 0
            width:              parent.width - (control._inset ? _margins * 2 : 0)
            spacing:            _margins
        }
    }

    QGCLabel {
        Layout.leftMargin:  _margins
        Layout.rightMargin: _margins
        Layout.topMargin:   _margins / 2
        Layout.fillWidth:   true
        text:               description
        wrapMode:           Text.WordWrap
        font.pointSize:     ScreenTools.smallFontPointSize
        color:              Qt.alpha(QGroundControl.globalPalette.text, 0.65)
        visible:            description !== ""
    }
}
