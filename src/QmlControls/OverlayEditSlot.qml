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

Item {
    id: root

    required property var    rig
    required property string editKey

    property bool available:    true
    property bool lifted:       false
    property bool swallowsTaps: false

    readonly property bool editing: rig ? rig.editMode : false
    readonly property bool hidden:  rig ? rig.isHidden(editKey) : false

    visible: available && (editing || !hidden)
    opacity: hidden ? rig.hiddenOpacity : 1

    JiggleAnimation {
        target:  root
        lifted:  root.lifted
        running: root.editing && root.visible
    }

    OverlayEditBadge {
        id:      editBadge
        rig:     root.rig
        editKey: root.editKey
    }

    QGCMouseArea {
        anchors.fill: parent
        z:            editBadge.z - 1
        enabled:      root.editing && root.swallowsTaps
        visible:      enabled
    }
}
