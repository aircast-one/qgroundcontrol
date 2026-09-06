/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Window
import QtQuick.Controls
import QtQuick.Templates as T

import QGroundControl.ScreenTools
import QGroundControl.Palette
import QGroundControl.Controls

T.ComboBox {
    id:             control
    padding:        ScreenTools.comboBoxPadding
    spacing:        ScreenTools.defaultFontPixelWidth
    font.pointSize: ScreenTools.defaultFontPointSize
    font.family:    ScreenTools.normalFontFamily
    implicitWidth:  Math.max(background ? background.implicitWidth : 0,
                             contentItem.implicitWidth + leftPadding + rightPadding + padding)
    implicitHeight: Math.max(background ? background.implicitHeight : 0,
                             Math.max(contentItem.implicitHeight, indicator ? indicator.implicitHeight : 0) + topPadding + bottomPadding)
    baselineOffset: contentItem.y + text.baselineOffset
    leftPadding:    padding + (!control.mirrored || !indicator || !indicator.visible ? 0 : indicator.width + spacing)
    rightPadding:   padding + (control.mirrored || !indicator || !indicator.visible ? 0 : indicator.width)

    property bool   centeredLabel:  false
    property bool   sizeToContents: false
    property string alternateText:  ""

    property real   _popupWidth:    width
    property bool   _onCompleted:   false

    QGCPalette { id: qgcPal; colorGroupEnabled: enabled }

    TextMetrics {
        id:                 textMetrics
        font.family:        control.font.family
        font.pointSize:     control.font.pointSize
    }

    TextMetrics {
        id:     rowMetrics
        font:   control.font
        text:   "Xg"
    }

    readonly property real _rowHeight:    Math.round(rowMetrics.height * 1.75)
    readonly property real _popupPadding: Math.round(ScreenTools.defaultFontPixelHeight * 0.25)

    ItemDelegate {
        id:             itemDelegateMetrics
        visible:        false
        font.family:    control.font.family
        font.pointSize: control.font.pointSize
    }

    function _calcPopupWidth() {
        if (_onCompleted && sizeToContents && model) {
            var largestTextWidth = 0
            for (var i = 0; i < model.length; i++){
                textMetrics.text = control.textRole ? model[i][control.textRole] : model[i]
                largestTextWidth = Math.max(textMetrics.width, largestTextWidth)
            }
            _popupWidth = largestTextWidth + itemDelegateMetrics.leftPadding + itemDelegateMetrics.rightPadding + _popupPadding * 2
        }
    }

    onModelChanged: _calcPopupWidth()

    Component.onCompleted: {
        _onCompleted = true
        _calcPopupWidth()
    }
    delegate: ItemDelegate {
        width:  _popupWidth - _popupPadding * 2
        height: _rowHeight

        property string _text: control.textRole ? 
                                    (model.hasOwnProperty(control.textRole) ? model[control.textRole] : modelData[control.textRole]) :
                                    modelData

        contentItem: Text {
            text:                   _text
            font:                   control.font
            color:                  control.highlightedIndex === index ? qgcPal.buttonHighlightText : qgcPal.buttonText
            verticalAlignment:      Text.AlignVCenter
        }

        background: Rectangle {
            radius:                 ScreenTools.buttonBorderRadius
            color:                  control.highlightedIndex === index ? qgcPal.buttonHighlight : "transparent"
        }

        highlighted:                control.highlightedIndex === index
    }

    indicator: Column {
        anchors.rightMargin:    Math.round(control.padding / 2)
        anchors.right:          parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing:                -Math.round(ScreenTools.defaultFontPixelHeight * 0.2)

        Repeater {
            model: [ "/InstrumentValueIcons/cheveron-up.svg", "/InstrumentValueIcons/cheveron-down.svg" ]

            QGCColoredImage {
                source:             modelData
                color:              Qt.alpha(qgcPal.text, control.enabled ? 0.55 : 0.3)
                height:             Math.round(ScreenTools.defaultFontPixelHeight * 0.5)
                width:              height
                sourceSize.height:  height
                fillMode:           Image.PreserveAspectFit
            }
        }
    }
    contentItem: QGCLabel {
        id:                         text
        anchors.verticalCenter:     parent.verticalCenter
        anchors.horizontalCenter:   centeredLabel ? parent.horizontalCenter : undefined
        text:                       control.alternateText === "" ? control.currentText : control.alternateText
        font:                       control.font
        color:                      qgcPal.buttonText
    }

    background: Rectangle {
        color:          qgcPal.button
        border.color:   qgcPal.buttonBorder
        border.width:   1
        radius:         ScreenTools.buttonBorderRadius
    }

    popup: T.Popup {
        x:              control.width - _popupWidth
        y:              control.height + _popupPadding
        width:          _popupWidth
        height:         Math.min(control.count * _rowHeight + _popupPadding * 2, control.Window.height - topMargin - bottomMargin)
        padding:        _popupPadding
        topMargin:      6
        bottomMargin:   6
        transformOrigin: Item.TopRight

        enter: Transition {
            NumberAnimation { property: "opacity"; from: 0;    to: 1; duration: 140; easing.type: Easing.OutCubic }
            NumberAnimation { property: "scale";   from: 0.94; to: 1; duration: 160; easing.type: Easing.OutCubic }
        }

        exit: Transition {
            NumberAnimation { property: "opacity"; from: 1; to: 0;    duration: 90; easing.type: Easing.InCubic }
            NumberAnimation { property: "scale";   from: 1; to: 0.98; duration: 90; easing.type: Easing.InCubic }
        }

        contentItem: ListView {
            clip:                   true
            model:                  control.delegateModel
            currentIndex:           control.highlightedIndex
            highlightMoveDuration:  0

            T.ScrollIndicator.vertical: ScrollIndicator { }
        }

        background: Rectangle {
            color:          qgcPal.windowShade
            border.color:   qgcPal.groupBorder
            border.width:   1
            radius:         Math.round(ScreenTools.defaultFontPixelHeight * 0.75)
        }
    }
}
