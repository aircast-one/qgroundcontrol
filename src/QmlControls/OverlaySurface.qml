import QtQuick

import QGroundControl

Rectangle {
    id: _root

    enum Material { Control, Panel }

    property int   material:  OverlaySurface.Control
    property bool  highlight: false
    property color tint:      defaultTint

    readonly property color defaultTint:   _panel ? _qgcPal.window : Qt.alpha(_qgcPal.windowShade, 0.94)
    readonly property color contentColor:  _qgcPal.text
    readonly property color invertedColor: _qgcPal.window

    readonly property bool _panel:  material === OverlaySurface.Panel
    readonly property var  _qgcPal: QGroundControl.globalPalette

    radius:       height / 2
    color:        tint
    border.width: 1
    border.color: highlight ? _qgcPal.colorBlue : _qgcPal.groupBorder
}
