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

    property string defaultBorderColor  : QGroundControl.globalPalette.groupBorder
    property string outerBorderColor    : defaultBorderColor

    property string heading
    property string headingDescription
    property bool   showDividers:       true
    property bool   showBorder:         true

    property real _margins: ScreenTools.defaultFontPixelHeight / 2

    ColumnLayout {
        Layout.leftMargin:  _margins
        Layout.fillWidth:   true
        spacing:            0
        visible:            heading !== ""

        QGCLabel { 
            text:           heading
            font.pointSize: ScreenTools.defaultFontPointSize + 1
            font.bold:      true
        }

        QGCLabel { 
            Layout.fillWidth:   true
            text:               headingDescription
            wrapMode:           Text.WordWrap
            font.pointSize:     ScreenTools.smallFontPointSize
            visible:            headingDescription !== ""
        }
    }

    Rectangle {
        id:                 outerRect
        Layout.fillWidth:   true
        implicitWidth:      _contentLayout.implicitWidth + (showBorder ? _margins * 2 : 0)
        implicitHeight:     _contentLayout.implicitHeight + (showBorder ? _margins * 2: 0)
        color:              showBorder ? QGroundControl.globalPalette.windowShade : "transparent"
        border.color:       outerBorderColor
        border.width:       showBorder ? 1 : 0
        radius:             Math.round(ScreenTools.defaultFontPixelHeight * 0.6)

        Repeater {
            model: showDividers? _contentLayout.children.length : 0

            Rectangle {
                x:                  showBorder ? _margins : 0
                // Half of the actual row gap. Callers that pack rows tight (contentSpacing 0)
                // used to get the hairline drawn a full margin down, striking through the
                // next row's text.
                y:                  _contentItem.y + _contentItem.height + (_contentLayout.spacing / 2) + (showBorder ? _margins : 0)
                width:              parent.width - (showBorder ? _margins * 2 : 0)
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
            x:                  showBorder ? _margins : 0
            y:                  showBorder ? _margins : 0
            width:              parent.width - (showBorder ? _margins * 2 : 0)
            spacing:            _margins * 2
        }
    }
}
