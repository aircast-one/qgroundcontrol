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
import QtQuick.Dialogs
import QtQuick.Layouts

import QtLocation
import QtPositioning
import QtQuick.Window
import QtQml.Models

import QGroundControl
import QGroundControl.Controls
import QGroundControl.Controllers
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.FlightDisplay
import QGroundControl.FlightMap
import QGroundControl.Palette
import QGroundControl.ScreenTools
import QGroundControl.Vehicle

// This is the ui overlay layer for the widgets/tools for Fly View
Item {
    id: _root

    required property var overlayRig

    property var    parentToolInsets
    property var    totalToolInsets:        _totalToolInsets
    property var    mapControl
    property bool   isViewer3DOpen:         false

    property var    _activeVehicle:         QGroundControl.multiVehicleManager.activeVehicle
    property var    _planMasterController:  globals.planMasterControllerFlyView
    property var    _missionController:     _planMasterController.missionController
    property var    _geoFenceController:    _planMasterController.geoFenceController
    property var    _rallyPointController:  _planMasterController.rallyPointController
    property var    _guidedController:      globals.guidedControllerFlyView
    property real   _margins:               ScreenTools.defaultFontPixelWidth / 2
    property real   _toolsMargin:           ScreenTools.defaultFontPixelWidth * 0.75
    property rect   _centerViewport:        Qt.rect(0, 0, width, height)
    property real   _rightPanelWidth:       ScreenTools.defaultFontPixelWidth * 30
    property alias  _gripperMenu:           gripperOptions
    property real   _layoutMargin:          ScreenTools.defaultFontPixelWidth * 0.75
    property real   _layoutSpacing:         ScreenTools.defaultFontPixelWidth
    property bool   _showSingleVehicleUI:   true

    property bool utmspActTrigger

    property real _bottomRightPanelsBottomInset: instrumentPanel.bottomEdgeRightInset

    readonly property real instrumentPanelReservedWidth: instrumentPanel.visible ? instrumentPanel.width + _layoutMargin * 2 : 0

    QGCToolInsets {
        id:                     _totalToolInsets
        leftEdgeTopInset:       toolStrip.leftEdgeTopInset
        leftEdgeCenterInset:    toolStrip.leftEdgeCenterInset
        leftEdgeBottomInset:    virtualJoystickMultiTouch.visible ? virtualJoystickMultiTouch.leftEdgeBottomInset : parentToolInsets.leftEdgeBottomInset
        rightEdgeTopInset:      topRightPanel.rightEdgeTopInset
        rightEdgeCenterInset:   topRightPanel.rightEdgeCenterInset
        rightEdgeBottomInset:   instrumentPanel.rightEdgeBottomInset
        topEdgeLeftInset:       toolStrip.topEdgeLeftInset
        topEdgeCenterInset:     mapScale.topEdgeCenterInset
        topEdgeRightInset:      topRightPanel.topEdgeRightInset
        bottomEdgeLeftInset:    virtualJoystickMultiTouch.visible ? virtualJoystickMultiTouch.bottomEdgeLeftInset : parentToolInsets.bottomEdgeLeftInset
        bottomEdgeCenterInset:  _bottomRightPanelsBottomInset
        bottomEdgeRightInset:   virtualJoystickMultiTouch.visible ? virtualJoystickMultiTouch.bottomEdgeRightInset : _bottomRightPanelsBottomInset
    }

    FlyViewTopRightPanel {
        id:                     topRightPanel
        anchors.top:            parent.top
        anchors.right:          parent.right
        anchors.topMargin:      _layoutMargin
        anchors.rightMargin:    _layoutMargin
        maximumHeight:          parent.height - (_bottomRightPanelsBottomInset + _margins * 5)

        property real topEdgeRightInset:    height + _layoutMargin
        property real rightEdgeTopInset:    width + _layoutMargin
        property real rightEdgeCenterInset: rightEdgeTopInset
    }

    FlyViewTopRightColumnLayout {
        id:                 topRightColumnLayout
        anchors.margins:    _layoutMargin
        anchors.top:        parent.top
        anchors.bottom:     parent.bottom
        anchors.bottomMargin: _layoutMargin + _bottomRightPanelsBottomInset
        anchors.right:      parent.right
        spacing:            _layoutSpacing
        visible:           !topRightPanel.visible

        property real topEdgeRightInset:    childrenRect.height + _layoutMargin
        property real rightEdgeTopInset:    width + _layoutMargin
        property real rightEdgeCenterInset: rightEdgeTopInset
    }

    // Was anchored inside the top-right column: no jiggle, no badge, no collision, and no way
    // to move the record button off whatever it happened to land on. It is a floating overlay
    // panel like every other one, so it belongs in the rig.
    ArrangeableOverlayItem {
        id:                 photoVideoSlot
        overlayRig:         _root.overlayRig
        control:            photoVideoControlLoader
        editKey:            "photoVideoControl"
        settingsKeyPrefix:  "PhotoVideoControl"
        available:          photoVideoControlLoader.status === Loader.Ready
        z:                  QGroundControl.zOrderWidgets
        defaultX:           _root.width - photoVideoSlot.width - _layoutMargin
        defaultY:           _layoutMargin + parentToolInsets.topEdgeRightInset +
                                topRightColumnLayout.topEdgeRightInset

        // A Loader so PhotoVideoControl is built only with an active vehicle, and does not
        // have to null-check the mavlink camera everywhere.
        Loader {
            id:              photoVideoControlLoader
            objectName:      "photoVideoControl"
            sourceComponent: (globals.activeVehicle &&
                                 QGroundControl.settingsManager.flyViewSettings.showPhotoVideoControl.rawValue &&
                                 !cameraControlLayer.hasShutter) ? photoVideoControlComponent : undefined

            Component {
                id: photoVideoControlComponent

                PhotoVideoControl {
                    showCloseButton: false
                }
            }
        }
    }

    FlyViewInstrumentPanel {
        id:         instrumentPanel
        overlayRig: _root.overlayRig
        visible:    QGroundControl.corePlugin.options.flyView.showInstrumentPanel && _showSingleVehicleUI &&
                        (overlayRig.editMode || !overlayRig.isHidden("instrumentPanel"))
        opacity:    overlayRig.isHidden("instrumentPanel") ? 0.35 : 1

        // DJI corner: the compass lives bottom-left with the minimap, so it reports no
        // right-edge insets any more.
        property real bottomEdgeRightInset: 0
        property real rightEdgeBottomInset: 0

        OverlayEditBadge {
            rig:     overlayRig
            editKey: "instrumentPanel"
        }

        Component.onCompleted:   overlayRig.registerMovable(instrumentPanel, instrumentPanelDragPosition)
        Component.onDestruction: overlayRig.unregisterMovable(instrumentPanel)

        Behavior on x {
            enabled: !instrumentPanelDragHandler.active && instrumentPanelDragPosition.settling
            NumberAnimation { duration: 350; easing.type: Easing.OutBack; easing.overshoot: 2 }
        }

        Behavior on y {
            enabled: !instrumentPanelDragHandler.active && instrumentPanelDragPosition.settling
            NumberAnimation { duration: 350; easing.type: Easing.OutBack; easing.overshoot: 2 }
        }

        DragToPosition {
            id:                 instrumentPanelDragPosition
            target:             instrumentPanel
            settingsKeyPrefix:  "InstrumentPanel"
            defaultX:           _root.width - instrumentPanel.width - _layoutMargin
            defaultY:           _root.height - instrumentPanel.height - _layoutMargin
        }

        JiggleAnimation {
            target:  instrumentPanel
            running: overlayRig.editMode && instrumentPanel.visible
            lifted:  instrumentPanelDragHandler.active
        }

        DragHandler {
            id:      instrumentPanelDragHandler
            parent:  instrumentPanel.contentItem
            target:  instrumentPanel
            enabled: overlayRig.editMode
            onActiveChanged: {
                if (!active) {
                    instrumentPanelDragPosition.commit()
                    overlayRig.requestReflow()
                }
            }
        }
    }

    FlyViewMissionCompleteDialog {
        missionController:      _missionController
        geoFenceController:     _geoFenceController
        rallyPointController:   _rallyPointController
    }

    readonly property real _bottomCenterWidgetClearance: ScreenTools.defaultFontPixelHeight * 8

    GuidedActionConfirm {
        objectName:                 "guidedActionConfirm"
        anchors.bottomMargin:       _toolsMargin + parentToolInsets.bottomEdgeCenterInset + _bottomCenterWidgetClearance
        anchors.bottom:             parent.bottom
        anchors.horizontalCenter:   parent.horizontalCenter
        z:                          QGroundControl.zOrderTopMost
        guidedController:           _guidedController
        guidedValueSlider:          _guidedValueSlider
        utmspSliderTrigger:         utmspActTrigger
    }

    //-- Virtual Joystick
    Loader {
        id:                         virtualJoystickMultiTouch
        z:                          QGroundControl.zOrderTopMost + 1
        anchors.right:              parent.right
        anchors.rightMargin:        anchors.leftMargin
        height:                     Math.min(parent.height * 0.25, ScreenTools.defaultFontPixelWidth * 16)
        visible:                    _virtualJoystickEnabled && !QGroundControl.videoManager.fullScreen && !(_activeVehicle ? _activeVehicle.usingHighLatencyLink : false)
        anchors.bottom:             parent.bottom
        anchors.bottomMargin:       bottomLoaderMargin
        anchors.left:               parent.left   
        anchors.leftMargin:         ( y > toolStrip.y + toolStrip.height ? toolStrip.width / 2 : toolStrip.width * 1.05 + toolStrip.x) 
        source:                     "qrc:/qml/QGroundControl/FlightDisplay/VirtualJoystick.qml"
        active:                     _virtualJoystickEnabled && !(_activeVehicle ? _activeVehicle.usingHighLatencyLink : false)

        property real bottomEdgeLeftInset:     parent.height-y
        property bool autoCenterThrottle:      QGroundControl.settingsManager.appSettings.virtualJoystickAutoCenterThrottle.rawValue
        property bool leftHandedMode:          QGroundControl.settingsManager.appSettings.virtualJoystickLeftHandedMode.rawValue
        property bool _virtualJoystickEnabled: QGroundControl.settingsManager.appSettings.virtualJoystick.rawValue
        property real bottomEdgeRightInset:    parent.height-y
        property var  _pipViewMargin:          _pipView.visible ? parentToolInsets.bottomEdgeLeftInset + ScreenTools.defaultFontPixelHeight * 2 :
                                               _bottomRightPanelsBottomInset + ScreenTools.defaultFontPixelHeight * 1.5

        property var  bottomLoaderMargin:      _pipViewMargin >= parent.height / 2 ? parent.height / 2 : _pipViewMargin

        // Width is difficult to access directly hence this hack which may not work in all circumstances
        property real leftEdgeBottomInset:  visible ? bottomEdgeLeftInset + width/18 - ScreenTools.defaultFontPixelHeight*2 : 0
        property real rightEdgeBottomInset: visible ? bottomEdgeRightInset + width/18 - ScreenTools.defaultFontPixelHeight*2 : 0
        property real rootWidth:            _root.width
        property var  itemX:                virtualJoystickMultiTouch.x   // real X on screen

        onRootWidthChanged: virtualJoystickMultiTouch.status == Loader.Ready && visible ? virtualJoystickMultiTouch.item.uiTotalWidth = rootWidth : undefined
        onItemXChanged:     virtualJoystickMultiTouch.status == Loader.Ready && visible ? virtualJoystickMultiTouch.item.uiRealX = itemX : undefined

        //Loader status logic
        onLoaded: {
            if (virtualJoystickMultiTouch.visible) {
                virtualJoystickMultiTouch.item.calibration = true 
                virtualJoystickMultiTouch.item.uiTotalWidth = rootWidth
                virtualJoystickMultiTouch.item.uiRealX = itemX
            } else {
                virtualJoystickMultiTouch.item.calibration = false
            }
        }
    }

    FlyViewToolStrip {
        id:                     toolStrip
        objectName:             "flyViewToolStrip"
        Component.onCompleted:   overlayRig.registerMovable(toolStrip, toolStripDragPosition)
        Component.onDestruction: overlayRig.unregisterMovable(toolStrip)
        z:                      QGroundControl.zOrderWidgets
        // Height of the column the strip may occupy. Deriving this from y instead would close
        // a loop, because the dragged y is clamped against the strip's own height.
        maxHeight:              parent.height - (_toolsMargin + parentToolInsets.topEdgeLeftInset) -
                                    parentToolInsets.bottomEdgeLeftInset - _toolsMargin
        visible:                !QGroundControl.videoManager.fullScreen
        editing:                overlayRig.editMode
        onHeld:                 overlayRig.editMode = true

        onDisplayPreFlightChecklist: {
            if (!preFlightChecklistLoader.active) {
                preFlightChecklistLoader.active = true
            }
            preFlightChecklistLoader.item.open()
        }

        property real topEdgeLeftInset:     visible ? y + height : 0
        property real leftEdgeTopInset:     visible ? x + width : 0
        property real leftEdgeCenterInset:  leftEdgeTopInset

        Behavior on x {
            enabled: !toolStripDragHandler.active && toolStripDragPosition.settling
            NumberAnimation { duration: 350; easing.type: Easing.OutBack; easing.overshoot: 2 }
        }

        Behavior on y {
            enabled: !toolStripDragHandler.active && toolStripDragPosition.settling
            NumberAnimation { duration: 350; easing.type: Easing.OutBack; easing.overshoot: 2 }
        }

        DragToPosition {
            id:                 toolStripDragPosition
            target:             toolStrip
            settingsKeyPrefix:  "FlyViewToolStrip"
            defaultX:           _toolsMargin + parentToolInsets.leftEdgeCenterInset
            defaultY:           _toolsMargin + parentToolInsets.topEdgeLeftInset
        }

        JiggleAnimation {
            target:  toolStrip
            running: overlayRig.editMode && toolStrip.visible
            lifted:  toolStripDragHandler.active
        }

        DragHandler {
            id:      toolStripDragHandler
            enabled: overlayRig.editMode

            onActiveChanged: {
                if (!active) {
                    toolStripDragPosition.commit()
                    overlayRig.requestReflow()
                }
            }
        }
    }

    // Takeoff and land are their own arrangeable buttons rather than two rows in the tool
    // strip: they are the two actions a pilot reaches for under time pressure, and where the
    // thumb already is differs per airframe and per operator.
    component GuidedOverlayButton: ArrangeableOverlayItem {
        id: guidedSlot

        required property string action
        required property string title
        required property string glyph
        required property bool   actionable
        required property string buttonName

        // Visible in edit mode whatever the vehicle is doing. Gating on `actionable` alone
        // meant the takeoff button could only be arranged while the vehicle was ready to take
        // off and the land button only while it was flying - never on the bench, which is
        // where people arrange their layout.
        overlayRig: _root.overlayRig
        control:    guidedButton
        available:  actionable || _root.overlayRig.editMode
        z:          QGroundControl.zOrderWidgets

        Column {
            id:      guidedButton
            spacing: ScreenTools.defaultFontPixelHeight * 0.15

            OverlayRoundButton {
                id:                       guidedGlyph
                objectName:               guidedSlot.buttonName
                anchors.horizontalCenter: parent.horizontalCenter
                icon:                     guidedSlot.glyph
                editing:                  _root.overlayRig.editMode
                lifted:                   guidedSlot.dragging
                actionsEnabled:           guidedSlot.actionable
                opacity:                  guidedSlot.actionable ? 1 : _unavailableOpacity
                onClicked: {
                    _guidedController.closeAll()
                    _guidedController.confirmAction(_guidedController[guidedSlot.action])
                }
                onHeld: _root.overlayRig.editMode = true
            }

            // The tool strip labelled these; an unlabelled glyph for the two most consequential
            // guided actions on the screen is not an improvement.
            QGCLabel {
                anchors.horizontalCenter: parent.horizontalCenter
                text:                     guidedSlot.title
                font.pointSize:           ScreenTools.smallFontPointSize
                opacity:                  guidedSlot.actionable ? 1 : _unavailableOpacity
                style:                    Text.Outline
                styleColor:               "black"
            }
        }
    }

    readonly property real _unavailableOpacity: 0.5
    readonly property real _guidedButtonX:      _toolsMargin + parentToolInsets.leftEdgeCenterInset
    readonly property real _guidedButtonY:      _root.height - _toolsMargin -
                                                    parentToolInsets.bottomEdgeLeftInset

    GuidedOverlayButton {
        id:                 takeoffSlot
        action:             "actionTakeoff"
        title:              _guidedController.takeoffTitle
        glyph:              "/res/takeoff.svg"
        buttonName:         "guidedTakeoffButton"
        actionable:         _guidedController.showTakeoff
        editKey:            "takeoffButton"
        settingsKeyPrefix:  "GuidedTakeoffButton"
        defaultX:           _guidedButtonX
        defaultY:           _guidedButtonY - takeoffSlot.height
    }

    // Offset by its own height: the two are mutually exclusive today, so the rig never sees
    // them as a pair to separate, and identical defaults would stack them the day both show.
    GuidedOverlayButton {
        id:                 landSlot
        action:             "actionLand"
        title:              _guidedController.landTitle
        glyph:              "/res/land.svg"
        buttonName:         "guidedLandButton"
        actionable:         _guidedController.showLand
        editKey:            "landButton"
        settingsKeyPrefix:  "GuidedLandButton"
        defaultX:           _guidedButtonX
        defaultY:           _guidedButtonY - takeoffSlot.height - _toolsMargin - landSlot.height
    }

    GripperMenu {
        id: gripperOptions
    }

    VehicleWarnings {
        anchors.top:                parent.top
        anchors.topMargin:          parentToolInsets.topEdgeCenterInset + _toolsMargin
        anchors.horizontalCenter:   parent.horizontalCenter
        z:                          QGroundControl.zOrderTopMost
    }

    MapScale {
        id:                 mapScale
        objectName:         "flyViewMapScale"
        anchors.margins:    _toolsMargin
        anchors.left:       toolStrip.right
        anchors.top:        toolStrip.top
        anchors.topMargin:  0
        mapControl:         _mapControl
        buttonsOnLeft:      true
        zoomButtonsVisible: false
        visible:            !ScreenTools.isTinyScreen && QGroundControl.corePlugin.options.flyView.showMapScale && !isViewer3DOpen && mapControl.pipState.state === mapControl.pipState.fullState

        property real topEdgeCenterInset: visible ? y + height : 0
    }

    Loader {
        id: preFlightChecklistLoader
        sourceComponent: preFlightChecklistPopup
        active: false
    }

    Component {
        id: preFlightChecklistPopup
        FlyViewPreFlightChecklistPopup {
        }
    }
}
