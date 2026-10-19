import QtQuick
import QtQuick.Controls

import QGroundControl
import QGroundControl.Controls

Item {
    id:         _root
    visible:    false

    signal          clicked()
    property real   radius:             ScreenTools.isMobile ? ScreenTools.defaultFontPixelHeight * 1.75 : ScreenTools.defaultFontPixelHeight * 1.25
    property real   viewportMargins:    0
    property var    toolStrip

    readonly property int dropLeft:     1
    readonly property int dropRight:    2
    readonly property int dropUp:       3
    readonly property int dropDown:     4

    readonly property real _dropMargin:     ScreenTools.defaultFontPixelHeight * 0.4
    readonly property real _panelRadius:    ScreenTools.defaultFontPixelHeight * 0.8

    property var    _dropEdgeTopPoint
    property alias  _dropDownComponent: panelLoader.sourceComponent
    property real   _viewportMaxTop:    0
    property real   _viewportMaxBottom: parent.parent.height - parent.y
    property real   _viewportMaxHeight: _viewportMaxBottom - _viewportMaxTop
    property var    _dropPanelCancel
    property var    _parentButton

    function show(panelEdgeTopPoint, panelComponent, parentButton) {
        _parentButton = parentButton
        _dropEdgeTopPoint = panelEdgeTopPoint
        _dropDownComponent = panelComponent
        _calcPositions()
        visible = true
        _dropPanelCancel = dropPanelCancelComponent.createObject(toolStrip.parent)
    }

    function hide() {
        if (_dropPanelCancel) {
            _dropPanelCancel.destroy()
            _parentButton.checked = false
            visible = false
            _dropDownComponent = undefined
        }
    }

    function _calcPositions() {
        var panelComponentWidth  = panelLoader.item.width
        var panelComponentHeight = panelLoader.item.height

        dropDownItem.width  = panelComponentWidth  + (_dropMargin * 2)
        dropDownItem.height = panelComponentHeight + (_dropMargin * 2)

        dropDownItem.x = _dropEdgeTopPoint.x + _dropMargin
        dropDownItem.y = _dropEdgeTopPoint.y -(dropDownItem.height / 2) + radius

        dropDownItem.y = Math.min(dropDownItem.y + dropDownItem.height, _viewportMaxBottom) - dropDownItem.height
        dropDownItem.y = Math.max(dropDownItem.y, _viewportMaxTop)

        dropDownItem.height = Math.min(dropDownItem.height, _viewportMaxHeight - dropDownItem.y)
    }

    Component {
        id: dropPanelCancelComponent

        MouseArea {
            anchors.fill:   parent
            z:              toolStrip.z - 1
            onClicked:      dropPanel.hide()
        }
    }

    Item {
        id: dropDownItem

        DeadMouseArea {
            anchors.fill: parent
        }

        Rectangle {
            anchors.fill:  parent
            color:         "transparent"
            radius:        _panelRadius
            layer.enabled: true
            layer.effect:  OverlayShadowEffect { elevated: true }

            OverlaySurface {
                anchors.fill: parent
                radius:       parent.radius
                material:     OverlaySurface.Panel
            }
        }

        QGCFlickable {
            id:                 panelItemFlickable
            anchors.margins:    _dropMargin
            anchors.fill:       parent
            flickableDirection: Flickable.VerticalFlick
            contentWidth:       panelLoader.width
            contentHeight:      panelLoader.height

            Loader {
                id: panelLoader

                onHeightChanged:    _calcPositions()
                onWidthChanged:     _calcPositions()

                property var dropPanel: _root
            }
        }
    }
}
