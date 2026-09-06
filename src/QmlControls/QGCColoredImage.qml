import QtQuick
import QtQuick.Controls
import Qt5Compat.GraphicalEffects

import QGroundControl.Palette

Item {
    property color color: "white"   // Image color

    property alias asynchronous:        image.asynchronous
    property alias cache:               image.cache
    property alias fillMode:            image.fillMode
    property alias horizontalAlignment: image.horizontalAlignment
    property alias mirror:              image.mirror
    property alias paintedHeight:       image.paintedHeight
    property alias paintedWidth:        image.paintedWidth
    property alias progress:            image.progress
    property alias mipmap:              image.mipmap
    property alias source:              image.source
    property alias sourceSize:          image.sourceSize
    property alias status:              image.status
    property alias verticalAlignment:   image.verticalAlignment

    width:  image.width
    height: image.height

    readonly property bool _tinted: color.a > 0

    Image {
        id:                 image
        smooth:             true
        mipmap:             true
        antialiasing:       true
        visible:            !_tinted
        fillMode:           Image.PreserveAspectFit
        anchors.fill:       parent
        sourceSize.height:  height
    }

    ColorOverlay {
        anchors.fill:       image
        source:             image
        visible:            _tinted
        color:              Qt.rgba(parent.color.r, parent.color.g, parent.color.b, 1)
        opacity:            parent.color.a
    }
}
