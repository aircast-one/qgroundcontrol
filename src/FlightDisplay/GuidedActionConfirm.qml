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
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.Palette
import QGroundControl.UTMSP

Rectangle {
    id:         _root
    width:      ScreenTools.defaultFontPixelWidth * 34
    height:     mainLayout.implicitHeight + (_padding * 2)
    radius:     ScreenTools.defaultFontPixelHeight * 0.9
    color:      "transparent"
    layer.enabled: true
    layer.effect:  OverlayShadowEffect { elevated: true }
    visible:    _utmspEnabled && utmspSliderTrigger
    scale:      visible ? 1 : 0.92

    Behavior on scale { NumberAnimation { duration: 200; easing.type: Easing.OutBack; easing.overshoot: 1.4 } }

    OverlayGlass {
        anchors.fill: parent
        radius:       _root.radius
        material:     OverlayGlass.Panel
    }

    property var    guidedController
    property var    guidedValueSlider
    property alias  title:              titleText.text
    property alias  message:            messageText.text
    property int    action
    property var    actionData
    property bool   hideTrigger:        false
    property var    mapIndicator
    property alias  optionText:         optionCheckBox.text
    property alias  optionChecked:      optionCheckBox.checked

    property real _margins:         ScreenTools.defaultFontPixelWidth / 2
    property real _padding:         ScreenTools.defaultFontPixelHeight * 0.85
    property bool _emergencyAction: action === guidedController.actionEmergencyStop

    // Properties of UTM adapter
    property bool   utmspSliderTrigger
    property bool   _utmspEnabled:                       QGroundControl.utmspSupported

    Component.onCompleted: guidedController.confirmDialog = this

    onVisibleChanged: {
        if (visible) {
            slider.focus = true
        }
    }

    onHideTriggerChanged: {
        if (hideTrigger) {
            confirmCancelled()
        }
    }

    function show(immediate) {
        if (immediate) {
            visible = true
        } else {
            // We delay showing the confirmation for a small amount in order for any other state
            // changes to propogate through the system. This way only the final state shows up.
            visibleTimer.restart()
        }
    }

    function confirmCancelled() {
        guidedValueSlider.visible = false
        visible = false
        hideTrigger = false
        visibleTimer.stop()
        if (mapIndicator) {
            mapIndicator.actionCancelled()
            mapIndicator = undefined
        }
    }

    Timer {
        id:             visibleTimer
        interval:       1000
        repeat:         false
        onTriggered:    visible = true
    }

    QGCPalette { id: qgcPal }

    ColumnLayout {
        id:                 mainLayout
        anchors.centerIn:   parent
        width:              parent.width - (_padding * 2)
        spacing:            _margins

        QGCLabel {
            id:                     titleText
            Layout.fillWidth:       true
            Layout.bottomMargin:    -_margins / 2
            horizontalAlignment:    Text.AlignHCenter
            wrapMode:               Text.WordWrap
            font.pointSize:         ScreenTools.mediumFontPointSize
            font.bold:              true
            color:                  _emergencyAction ? qgcPal.colorRed : qgcPal.text
            visible:                text !== ""
        }

        QGCLabel {
            id:                     messageText
            Layout.fillWidth:       true
            Layout.bottomMargin:    _margins / 2
            horizontalAlignment:    Text.AlignHCenter
            wrapMode:               Text.WordWrap
            font.pointSize:         ScreenTools.smallFontPointSize
            opacity:                0.7
        }

        RowLayout {
            Layout.alignment:    Qt.AlignHCenter
            Layout.bottomMargin: _margins / 2
            spacing:             _margins * 2
            visible:             guidedValueSlider ? guidedValueSlider.visible : false

            OverlayRoundButton {
                objectName: "guidedValueDown"
                text:       "−"
                onClicked:  guidedValueSlider.step(-1)
            }

            QGCLabel {
                objectName:          "guidedValueReadout"
                horizontalAlignment: Text.AlignHCenter
                font.pointSize:      ScreenTools.mediumFontPointSize
                font.bold:           true
                text:                guidedValueSlider ? guidedValueSlider.displayText + "  " + guidedValueSlider.valueString : ""
            }

            OverlayRoundButton {
                objectName: "guidedValueUp"
                text:       "+"
                onClicked:  guidedValueSlider.step(1)
            }
        }

        QGCCheckBox {
            id:                 optionCheckBox
            Layout.alignment:   Qt.AlignHCenter
            Layout.bottomMargin: _margins / 2
            text:               ""
            visible:            text !== ""
        }

        SliderSwitch {
            id:                 slider
            confirmText:        ScreenTools.isMobile ? qsTr("Slide to confirm") : qsTr("Slide or hold spacebar")
            destructive:        _emergencyAction
            Layout.fillWidth:   true
            enabled: _utmspEnabled === true? utmspSliderTrigger : true
            opacity: if(_utmspEnabled){utmspSliderTrigger === true ? 1 : 0.5} else{1}

            onAccept: {
                _root.visible = false
                var sliderOutputValue = 0
                if (guidedValueSlider.visible) {
                    sliderOutputValue = guidedValueSlider.getOutputValue()
                    guidedValueSlider.visible = false
                }
                hideTrigger = false
                guidedController.executeAction(_root.action, _root.actionData, sliderOutputValue, _root.optionChecked)
                if (mapIndicator) {
                    mapIndicator.actionConfirmed()
                    mapIndicator = undefined
                }

                UTMSPStateStorage.indicatorOnMissionStatus = true
                UTMSPStateStorage.currentNotificationIndex = 7
                UTMSPStateStorage.currentStateIndex = 3
            }
        }

        Rectangle {
            objectName:         "guidedActionCancel"
            Layout.fillWidth:   true
            Layout.preferredHeight: slider.height * 0.8
            radius:             height / 2
            color:              cancelMouseArea.pressed        ? Qt.alpha(qgcPal.text, 0.15)
                              : cancelMouseArea.containsMouse  ? Qt.alpha(qgcPal.text, 0.08)
                                                               : "transparent"

            QGCLabel {
                anchors.centerIn:   parent
                text:               qsTr("Cancel")
                color:              qgcPal.text
                opacity:            0.75
            }

            QGCMouseArea {
                id:           cancelMouseArea
                anchors.fill: parent
                hoverEnabled: !ScreenTools.isMobile
                onClicked:    confirmCancelled()
            }
        }
    }
}

