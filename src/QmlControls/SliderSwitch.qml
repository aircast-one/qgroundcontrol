import QtQuick
import QtQuick.Controls

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Palette

/// The SliderSwitch control implements a sliding switch control similar to the power off
/// control on an iPhone. It supports holding the space bar to slide the switch.
Rectangle {
    id:             _root
    implicitWidth:  label.contentWidth + (_diameter * 2.5) + (_border * 4)
    implicitHeight: Math.max(label.height * 2.6, ScreenTools.minTouchPixels)
    radius:         height / 2
    color:          Qt.tint(qgcPal.window, Qt.alpha(qgcPal.text, 0.30))
    border.width:   0

    signal accept   ///< Action confirmed

    property string confirmText                         ///< Text for slider
    property bool   destructive:   false                ///< Confirms something that cannot be undone
    property alias  fontPointSize: label.font.pointSize ///< Point size for text

    property real _border:                      Math.round(ScreenTools.defaultFontPixelHeight * 0.2)
    property real _diameter:                    height - (_border * 2)
    property real _dragStartX:                  _border
    property real _dragStopX:                   _root.width - (_diameter + _border)
    property real _travel:                      Math.max(0, Math.min(1, (slider.x - _dragStartX) / Math.max(1, _dragStopX - _dragStartX)))

    Keys.onSpacePressed: (event) => {
        if (visible && event.modifiers === Qt.NoModifier && !sliderDragArea.drag.active) {
            event.accepted = true
            sliderAnimation.start()
        }
    }

    Keys.onReleased: (event) => {
        if (visible && event.key === Qt.Key_Space && !event.isAutoRepeat) {
            event.accepted = true
            resetSpaceBarSliding()
        }
    }

    function resetSpaceBarSliding() {
        slider.reset()
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

    QGCLabel {
        id:                         label
        x:                          0
        width:                      parent.width
        anchors.verticalCenter:     parent.verticalCenter
        horizontalAlignment:        Text.AlignHCenter
        text:                       confirmText
        color:                      qgcPal.text
        opacity:                    0.75 * (1 - _root._travel)
    }

    Rectangle {
        id:            slider
        x:             _border
        y:             _border
        height:        _diameter
        width:         _diameter
        radius:        _diameter / 2
        color:         _root.destructive ? qgcPal.colorRed : qgcPal.window
        Behavior on x {
            enabled: !sliderDragArea.drag.active && !sliderAnimation.running
            NumberAnimation { duration: 220; easing.type: Easing.OutBack; easing.overshoot: 1.5 }
        }

        QGCColoredImage {
            anchors.centerIn:       parent
            width:                  parent.width  * 0.6
            height:                 parent.height * 0.6
            sourceSize.height:      height
            fillMode:               Image.PreserveAspectFit
            smooth:                 true
            mipmap:                 true
            color:                  _root.destructive ? "white" : qgcPal.text
            cache:                  false
            source:                 "/res/ArrowRight.svg"
        }

        PropertyAnimation on x {
            id:         sliderAnimation
            duration:   1500
            from:       _dragStartX
            to:         _dragStopX
            running:    false

            onFinished: {
                slider.reset()
                _root.accept()
            }
        }

        function reset() {
            sliderAnimation.stop()
            slider.x = _border
        }
    }

    QGCMouseArea {
        id:                 sliderDragArea
        anchors.leftMargin: -ScreenTools.defaultFontPixelWidth * 15
        fillItem:           slider
        drag.target:        slider
        drag.axis:          Drag.XAxis
        drag.minimumX:      _dragStartX
        drag.maximumX:      _dragStopX
        preventStealing:    true

        property bool dragActive: drag.active

        onDragActiveChanged: {
            if (!sliderDragArea.drag.active) {
                if (slider.x > _dragStopX - _border) {
                    _root.accept()
                }
                slider.reset()
            }
        }
    }
}
