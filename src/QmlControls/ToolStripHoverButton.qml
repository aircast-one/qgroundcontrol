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

import QGroundControl.ScreenTools
import QGroundControl.Palette

Button {
    id:             control
    width:          contentLayoutItem.contentWidth + (contentMargins * 2)
    height:         width
    hoverEnabled:   !ScreenTools.isMobile
    enabled:        toolStripAction.enabled

    visible:        toolStripAction.visible
    imageSource:    toolStripAction.showAlternateIcon ? modelData.alternateIconSource : modelData.iconSource
    text:           toolStripAction.text
    checked:        toolStripAction.checked
    checkable:      toolStripAction.dropPanelComponent || modelData.checkable

    // Elevation only in the round fly-view theme; the plan-view strip is a flat panel.
    layer.enabled:  iconOnly
    layer.effect:   OverlayShadowEffect { }

    property var    toolStripAction:    undefined
    property var    dropPanel:          undefined
    property alias  radius:             buttonBkRect.radius
    property alias  fontPointSize:      innerText.font.pointSize
    property alias  imageSource:        innerImage.source
    property alias  contentWidth:       innerText.contentWidth
    // Plain properties, not aliases: buttonBkRect lives inside `background:` and border.color
    // is a grouped sub-property, which QML refuses to alias ("Invalid alias target location").
    property color  borderColor:        "transparent"
    property real   borderWidth:        0

    property color  bkColor:             qgcPal.toolbarBackground
    property color  bkHoverColor:        qgcPal.toolStripHoverColor
    property color  bkCheckedColor:      qgcPal.buttonHighlight
    property color  contentColor:        qgcPal.buttonText
    property color  contentCheckedColor: qgcPal.buttonHighlightText

    property bool forceImageScale11: false
    property bool iconOnly:          false
    property real imageScale:        iconOnly ? 0.5 : (forceImageScale11 && (text == "") ? 0.8 : 0.6)
    property real contentMargins:    iconOnly ? 0 : innerText.height * 0.1

    property color _currentContentColor:  (checked || pressed) ? contentCheckedColor : contentColor
    property color _currentContentColorSecondary:  (checked || pressed) ? qgcPal.buttonText : qgcPal.buttonHighlight

    signal dropped(int index)

    onCheckedChanged: toolStripAction.checked = checked

    onClicked: {
        if (mainWindow.allowViewSwitch()) {
            dropPanel.hide()
            if (!toolStripAction.dropPanelComponent) {
                toolStripAction.triggered(this)
            } else if (checked) {
                var panelEdgeTopPoint = mapToItem(_root, width, 0)
                dropPanel.show(panelEdgeTopPoint, toolStripAction.dropPanelComponent, this)
                checked = true
                control.dropped(index)
            }
        } else if (checkable) {
            checked = !checked
        }
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: control.enabled }

    QGCToolTip {
        visible:    control.iconOnly && control.hovered && control.text !== ""
        text:       control.text
        delay:      300
    }

    contentItem: Item {
        id:                 contentLayoutItem
        anchors.fill:       parent
        anchors.margins:    contentMargins

        Column {
            anchors.centerIn:   parent
            spacing:        contentMargins * 2

            Image {
                id:                         innerImageColorful
                height:                     contentLayoutItem.height * imageScale
                width:                      contentLayoutItem.width  * imageScale
                smooth:                     true
                mipmap:                     true
                fillMode:                   Image.PreserveAspectFit
                antialiasing:               true
                sourceSize.height:          height
                sourceSize.width:           width
                anchors.horizontalCenter:   parent.horizontalCenter
                source:                     control.imageSource
                visible:                    source != "" && modelData.fullColorIcon
            }

            QGCColoredImage {
                id:                         innerImage
                height:                     contentLayoutItem.height * imageScale
                width:                      contentLayoutItem.width  * imageScale
                smooth:                     true
                mipmap:                     true
                color:                      _currentContentColor
                fillMode:                   Image.PreserveAspectFit
                antialiasing:               true
                sourceSize.height:          height
                sourceSize.width:           width
                anchors.horizontalCenter:   parent.horizontalCenter
                visible:                    source != "" && !modelData.fullColorIcon
                
                QGCColoredImage {
                    id:                         innerImageSecondColor
                    source:                     modelData.alternateIconSource
                    height:                     contentLayoutItem.height * imageScale
                    width:                      contentLayoutItem.width  * imageScale
                    smooth:                     true
                    mipmap:                     true
                    color:                      _currentContentColorSecondary
                    fillMode:                   Image.PreserveAspectFit
                    antialiasing:               true
                    sourceSize.height:          height
                    sourceSize.width:           width
                    anchors.horizontalCenter:   parent.horizontalCenter
                    visible:                    source != "" && modelData.biColorIcon
                }
            }

            QGCLabel {
                id:                         innerText
                text:                       control.text
                color:                      _currentContentColor
                anchors.horizontalCenter:   parent.horizontalCenter
                font.bold:                  !innerImage.visible && !innerImageColorful.visible
                opacity:                    !innerImage.visible ? 0.8 : 1.0
                visible:                    !control.iconOnly || (!innerImage.visible && !innerImageColorful.visible)
            }
        }
    }

    background: Rectangle {
        id:             buttonBkRect
        color:          (control.checked || control.pressed) ?
                            bkCheckedColor :
                            ((control.enabled && control.hovered) ? bkHoverColor : bkColor)
        border.color:   control.borderColor
        border.width:   control.borderWidth
        anchors.fill:   parent
    }
}
