/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl.Controls

// One draggable slot in an OverlayRig: sizes itself to `control`, carries the drag, the
// persisted position, the edit-mode jiggle and the hide badge. Declare the control as a child
// and point `control` at it - the slot has to own x/y, so the Behaviors cannot live on the
// control itself.
Item {
    id: root

    required property var    overlayRig
    required property Item   control
    required property string editKey
    required property string settingsKeyPrefix

    property bool available:    true
    property string hint:       ""
    property real defaultX:     0
    property real defaultY:     0

    readonly property bool dragging: _dragHandler.active
    readonly property bool held:     overlayRig.heldItem === root

    readonly property int  _animationDuration:  350
    readonly property real _animationOvershoot: 2
    readonly property real _hiddenOpacity:      0.35

    readonly property bool displaced: _position.displaced

    width:      control ? control.width : 0
    height:     control ? control.height : 0
    visible:    available && (overlayRig.editMode || !overlayRig.isHidden(editKey))
    opacity:    overlayRig.isHidden(editKey) ? _hiddenOpacity : 1

    Behavior on x {
        enabled: !root.dragging && _position.settling
        NumberAnimation {
            duration:           root._animationDuration
            easing.type:        Easing.OutBack
            easing.overshoot:   root._animationOvershoot
        }
    }

    Behavior on y {
        enabled: !root.dragging && _position.settling
        NumberAnimation {
            duration:           root._animationDuration
            easing.type:        Easing.OutBack
            easing.overshoot:   root._animationOvershoot
        }
    }

    DragToPosition {
        id:                 _position
        target:             root
        settingsKeyPrefix:  root.settingsKeyPrefix
        defaultX:           root.defaultX
        defaultY:           root.defaultY
    }

    DragHandler {
        id:            _dragHandler
        dragThreshold: root.overlayRig.dragThreshold

        onActiveChanged: {
            if (!active) {
                _position.commit()
                root.overlayRig.requestReflow()
            }
        }
    }

    JiggleAnimation {
        target:     root
        lifted:     root.dragging || root.held
        running:    root.overlayRig.editMode && root.visible
    }

    OverlayEditBadge {
        rig:     root.overlayRig
        editKey: root.editKey
    }

    HoverHandler { id: _hover }

    Loader {
        anchors.fill:    parent
        active:          _hover.hovered && root.hint !== ""
        sourceComponent: QGCToolTip { visible: true; text: root.hint }
    }

    Component.onCompleted:   overlayRig.registerMovable(root, _position)
    Component.onDestruction: overlayRig.unregisterMovable(root)
}
