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

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Palette

Button {
    id:             control
    width:          contentLayoutItem.contentWidth + (contentMargins * 2)
    implicitHeight: iconOnly ? discSize + captionLabel.height + _captionSpacing : width
    height:         implicitHeight
    hoverEnabled:   !ScreenTools.isMobile
    enabled:        !editing

    visible:        toolStripAction.visible
    imageSource:    toolStripAction.showAlternateIcon ? modelData.alternateIconSource : modelData.iconSource
    text:           toolStripAction.text
    checked:        toolStripAction.checked
    checkable:      toolStripAction.dropPanelComponent || modelData.checkable

    layer.enabled:  iconOnly
    layer.effect:   OverlayShadowEffect { }

    property var    toolStripAction:    undefined
    property var    dropPanel:          undefined
    property alias  radius:             buttonBkRect.radius
    property alias  fontPointSize:      innerText.font.pointSize
    property alias  imageSource:        innerImage.source
    property alias  contentWidth:       innerText.contentWidth
    property color  borderColor:        "transparent"
    property bool   glass:              false
    property real   borderWidth:        0

    property color  bkColor:             qgcPal.toolbarBackground
    property color  bkHoverColor:        qgcPal.toolStripHoverColor
    property color  bkCheckedColor:      qgcPal.buttonHighlight
    property color  contentColor:        qgcPal.buttonText
    property color  contentCheckedColor: qgcPal.buttonHighlightText

    property bool forceImageScale11: false
    property bool iconOnly:          false
    property bool editing:           false
    readonly property bool actionable: toolStripAction.enabled && !editing
    readonly property real _captionSpacing: ScreenTools.defaultFontPixelHeight * 0.15
    readonly property real discSize:        iconOnly ? Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * 2.4) : width
    property real imageScale:        iconOnly ? 0.5 : (forceImageScale11 && (text == "") ? 0.8 : 0.6)
    property real contentMargins:    iconOnly ? 0 : innerText.height * 0.1
    readonly property real _imageWidth:  (iconOnly ? discSize : contentLayoutItem.width)  * imageScale
    readonly property real _imageHeight: (iconOnly ? discSize : contentLayoutItem.height) * imageScale

    property color _currentContentColor:  (checked || pressed) ? contentCheckedColor : contentColor
    property color _currentContentColorSecondary:  (checked || pressed) ? qgcPal.buttonText : qgcPal.buttonHighlight

    signal dropped(int index)

    onCheckedChanged: toolStripAction.checked = checked

    onClicked: {
        if (!actionable) {
            return
        }
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

    QGCPalette { id: qgcPal; colorGroupEnabled: toolStripAction.enabled }

    QGCLabel {
        id:                         captionLabel
        anchors.horizontalCenter:   parent.horizontalCenter
        y:                          control.discSize + control._captionSpacing
        text:                       control.text
        color:                      QGroundControl.globalPalette.text
        opacity:                    toolStripAction.enabled ? 1 : 0.5
        font.pointSize:             ScreenTools.smallFontPointSize
        style:                      Text.Outline
        styleColor:                 "black"
        visible:                    control.iconOnly && control.text !== ""
    }

    contentItem: Item {
        id:                         contentLayoutItem
        anchors.horizontalCenter:   parent.horizontalCenter
        anchors.top:                parent.top
        anchors.topMargin:          contentMargins
        width:                      (control.iconOnly ? control.discSize : control.width) - contentMargins * 2
        height:                     (control.iconOnly ? control.discSize : control.height) - contentMargins * 2

        Column {
            anchors.centerIn:   parent
            spacing:        contentMargins * 2

            Image {
                id:                         innerImageColorful
                height:                     control._imageHeight
                width:                      control._imageWidth
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
                height:                     control._imageHeight
                width:                      control._imageWidth
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
                    height:                     control._imageHeight
                    width:                      control._imageWidth
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
        color:          (control.checked || control.pressed) ? bkCheckedColor
                            : control.glass ? "transparent"
                                            : ((control.actionable && control.hovered) ? bkHoverColor : bkColor)
        border.color:   control.borderColor
        border.width:   control.borderWidth
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top:    parent.top
        width:          control.iconOnly ? control.discSize : control.width
        height:         control.iconOnly ? control.discSize : control.height

        OverlayGlass {
            anchors.fill: parent
            visible:      control.glass && !control.checked && !control.pressed
            radius:       buttonBkRect.radius
            highlight:    control.actionable && control.hovered
        }
    }
}
