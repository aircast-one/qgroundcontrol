/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl.ScreenTools

// Corner resize handle for a free-floating item. Must be declared as a child of `target`:
// it anchors itself to the target's top-right corner. Pulling up or right enlarges along
// the dominant axis while the bottom edge stays pinned. The caller owns the size — apply
// the width reported by resized() (typically through a clamped binding) and persist it
// from committed(), which fires when the gesture ends. Deltas are tracked in scene
// coordinates so pinning the bottom edge (which moves this handle mid-gesture) cannot
// feed back into the growth math.
Item {
    id: root

    anchors.right:  parent.right
    anchors.top:    parent.top
    height:         ScreenTools.defaultFontPixelHeight * 2.5
    width:          height

    required property Item target

    property alias          iconVisible:    icon.visible
    readonly property alias pressed:        area.pressed

    signal resized(real newWidth)
    signal committed()

    Component.onCompleted: {
        if (parent !== target) {
            console.warn("ResizeHandle must be a child of its target; anchoring and bottom-pinning are wrong otherwise")
        }
    }

    Image {
        id:                 icon
        objectName:         "resizeHandleIcon"
        source:             "/qmlimages/pipResize.svg"
        anchors.fill:       parent
        fillMode:           Image.PreserveAspectFit
        mipmap:             true
        sourceSize.height:  height
    }

    MouseArea {
        id:                 area
        anchors.fill:       parent
        preventStealing:    true
        cursorShape:        Qt.PointingHandCursor

        property point  initialScenePos
        property real   initialWidth
        property real   initialBottom

        onPressed: (mouse) => {
            area.initialScenePos = area.mapToItem(null, mouse.x, mouse.y)
            area.initialWidth = root.target.width
            area.initialBottom = root.target.y + root.target.height
        }

        onPositionChanged: (mouse) => {
            if (area.pressed) {
                var scenePos = area.mapToItem(null, mouse.x, mouse.y)
                var growth = Math.max(scenePos.x - area.initialScenePos.x,
                                      area.initialScenePos.y - scenePos.y)
                root.resized(area.initialWidth + growth)
                root.target.y = area.initialBottom - root.target.height
            }
        }

        onReleased: root.committed()
        onCanceled: {
            root.resized(area.initialWidth)
            root.target.y = area.initialBottom - root.target.height
            root.committed()
        }
    }
}
