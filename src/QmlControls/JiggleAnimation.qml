/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

ParallelAnimation {
    id: root

    property Item target
    property real amplitude: 1.1
    property real shimmy:    0.7
    property bool lifted:    false
    property int  beat:      55 + Math.round(Math.random() * 15)

    readonly property Translate _translate: Translate {}

    readonly property NumberAnimation _liftAnimation: NumberAnimation {
        target:      root.target
        property:    "scale"
        duration:    150
        easing.type: Easing.OutQuad
    }

    function _settleLift() {
        if (target && running) {
            _liftAnimation.to = lifted ? 1.07 : 1.0
            _liftAnimation.restart()
        }
    }

    onLiftedChanged: _settleLift()
    onStarted:       _settleLift()

    onTargetChanged: {
        if (target && !target.transform.includes(_translate)) {
            target.transform = [...target.transform, _translate]
        }
    }

    onStopped: {
        if (target) {
            target.rotation = 0
            target.scale = 1
        }
        _translate.x = 0
        _translate.y = 0
    }

    Component.onDestruction: {
        if (target) {
            target.transform = target.transform.filter((entry) => entry !== _translate)
        }
    }

    SequentialAnimation {
        loops: Animation.Infinite

        NumberAnimation { target: root.target; property: "rotation"; to:  root.amplitude; duration: root.beat;     easing.type: Easing.InOutSine }
        NumberAnimation { target: root.target; property: "rotation"; to: -root.amplitude; duration: root.beat * 2; easing.type: Easing.InOutSine }
        NumberAnimation { target: root.target; property: "rotation"; to:  0;              duration: root.beat;     easing.type: Easing.InOutSine }
    }

    SequentialAnimation {
        loops: Animation.Infinite

        NumberAnimation { target: root._translate; property: "y"; to:  root.shimmy; duration: root.beat * 1.4; easing.type: Easing.InOutSine }
        NumberAnimation { target: root._translate; property: "y"; to: -root.shimmy; duration: root.beat * 2.8; easing.type: Easing.InOutSine }
        NumberAnimation { target: root._translate; property: "y"; to:  0;           duration: root.beat * 1.4; easing.type: Easing.InOutSine }
    }

    SequentialAnimation {
        loops: Animation.Infinite

        NumberAnimation { target: root._translate; property: "x"; to: -root.shimmy; duration: root.beat * 1.7; easing.type: Easing.InOutSine }
        NumberAnimation { target: root._translate; property: "x"; to:  root.shimmy; duration: root.beat * 3.4; easing.type: Easing.InOutSine }
        NumberAnimation { target: root._translate; property: "x"; to:  0;           duration: root.beat * 1.7; easing.type: Easing.InOutSine }
    }
}
