import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls

Item {
    id:         control
    width:      mainLayout.width
    visible:    false

    property var    guidedController
    property var    guidedValueSlider
    property var    messageDisplay
    property string title
    property string message
    property int    action
    property var    actionData
    property bool   hideTrigger:        false
    property var    mapIndicator
    property alias  optionText:         optionCheckBox.text
    property alias  optionChecked:      optionCheckBox.checked

    property real _margins:         2
    property bool _emergencyAction: action === guidedController.actionEmergencyStop

    Component.onCompleted: guidedController.confirmDialog = this

    onHideTriggerChanged: {
        if (hideTrigger) {
            confirmCancelled()
        }
    }

    function show(immediate) {
        if (immediate) {
            _reallyShow()
        } else {
            visibleTimer.restart()
        }
    }

    function reset() {
        visible = false
        guidedValueSlider.visible = false
        hideTrigger = false
        visibleTimer.stop()
        messageDisplay.opacity = 1.0
        messageFadeTimer.stop()
        messageOpacityAnimation.stop()
    }

    function confirmCancelled(incomingIndicator) {
        reset()
        if (mapIndicator && mapIndicator !== incomingIndicator) {
            mapIndicator.actionCancelled()
        }
        mapIndicator = undefined
    }

    function _reallyShow() {
        visible = true
        messageDisplay.opacity = 1.0
        messageFadeTimer.start()
    }

    Timer {
        id:             visibleTimer
        interval:       1000
        repeat:         false
        onTriggered:    _reallyShow()
    }

    QGCPalette { id: qgcPal }

    RowLayout {
        id:         mainLayout
        y:          2
        height:     parent.height - 4
        spacing:    ScreenTools.defaultFontPixelWidth

        QGCDelayButton {
            objectName:         "guidedActionConfirmButton"
            text:               control.title
            enabled:            true

            onActivated: {
                control.visible = false
                var sliderOutputValue = 0
                if (guidedValueSlider.visible) {
                    sliderOutputValue = guidedValueSlider.getOutputValue()
                    guidedValueSlider.visible = false
                }
                hideTrigger = false
                let success = guidedController.executeAction(control.action, control.actionData, sliderOutputValue, control.optionChecked)
                if (mapIndicator) {
                    if (success) {
                        mapIndicator.actionConfirmed()
                    } else {
                        mapIndicator.actionCancelled()
                    }
                    mapIndicator = undefined
                }
            }
        }

        QGCLabel {
            text:               qsTr("Hold to confirm")
            font.pointSize:     ScreenTools.smallFontPointSize
        }

        QGCCheckBox {
            id:                 optionCheckBox
            visible:            text !== ""
        }

        QGCColoredImage {
            id:                 closeButton
            Layout.alignment:   Qt.AlignTop
            width:              height
            height:             ScreenTools.defaultFontPixelHeight * 0.5
            source:             "/res/XDelete.svg"
            fillMode:           Image.PreserveAspectFit
            color:              qgcPal.text

            QGCMouseArea {
                fillItem:   parent
                onClicked:  confirmCancelled()
            }
        }
    }
}
