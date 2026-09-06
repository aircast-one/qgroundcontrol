import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.FactControls
import QGroundControl.Palette
import QGroundControl.UTMSP

// Plan view toolbar contents: what the plan is, what it adds up to, and the one action that
// matters. This was nine loose chips in three rows plus a button - every number the controller
// could produce, laid out flat, so nothing was easier to find than anything else.
RowLayout {
    id:      _root
    spacing: ScreenTools.defaultFontPixelWidth

    property var    planMasterController

    property var    _planMasterController:      planMasterController
    property var    _missionController:         _controllerValid ? _planMasterController.missionController : null
    property var    _currentMissionItem:        _controllerValid ? _missionController.currentPlanViewItem : null

    property bool   _controllerValid:           _planMasterController !== undefined && _planMasterController !== null
    property bool   _controllerOffline:         _controllerValid ? _planMasterController.offline : true
    property bool   _controllerDirty:           _controllerValid ? _planMasterController.dirty : false
    property bool   _controllerSyncInProgress:  _controllerValid ? _planMasterController.syncInProgress : false
    property bool   _containsItems:             _controllerValid ? _planMasterController.containsItems : false
    property real   _progressPct:               _controllerValid ? _missionController.progressPct : 0

    property var    missionItems:               _controllerValid ? _missionController.visualItems : undefined
    property bool   _missionValid:              missionItems !== undefined
    property int    _itemCount:                 _missionValid ? Math.max(missionItems.count - 1, 0) : 0

    property bool   _currentMissionItemValid:   _currentMissionItem && _currentMissionItem !== undefined && _currentMissionItem !== null
    property bool   _currentItemIsVTOLTakeoff:  _currentMissionItemValid && _currentMissionItem.command == 84

    property real   _distance:                  _currentMissionItemValid ? _currentMissionItem.distance : NaN
    property real   _altDifference:             _currentMissionItemValid ? _currentMissionItem.altDifference : NaN
    property real   _azimuth:                   _currentMissionItemValid ? _currentMissionItem.azimuth : NaN
    property real   _heading:                   _currentMissionItemValid ? _currentMissionItem.missionVehicleYaw : NaN
    property real   _missionPlannedDistance:    _missionValid ? _missionController.missionPlannedDistance : NaN
    property real   _missionMaxTelemetry:       _missionValid ? _missionController.missionMaxTelemetry : NaN
    property real   _missionTime:               _missionValid ? _missionController.missionTime : 0
    property int    _batteryChangePoint:        _controllerValid ? _missionController.batteryChangePoint : -1
    property int    _batteriesRequired:         _controllerValid ? _missionController.batteriesRequired : -1
    property bool   _batteryInfoAvailable:      _batteryChangePoint >= 0 || _batteriesRequired >= 0
    property real   _gradient:                  _currentMissionItemValid && _currentMissionItem.distance > 0 ?
                                                    (_currentItemIsVTOLTakeoff ?
                                                         0 :
                                                         (Math.atan(_currentMissionItem.altDifference / _currentMissionItem.distance) * (180.0/Math.PI)))
                                                  : NaN

    property string _distanceText:                  isNaN(_distance) ?                  _noValue : QGroundControl.unitsConversion.metersToAppSettingsHorizontalDistanceUnits(_distance).toFixed(1) + " " + QGroundControl.unitsConversion.appSettingsHorizontalDistanceUnitsString
    property string _altDifferenceText:             isNaN(_altDifference) ?             _noValue : QGroundControl.unitsConversion.metersToAppSettingsVerticalDistanceUnits(_altDifference).toFixed(1) + " " + QGroundControl.unitsConversion.appSettingsVerticalDistanceUnitsString
    property string _gradientText:                  isNaN(_gradient) ?                  _noValue : _gradient.toFixed(0) + qsTr(" deg")
    property string _azimuthText:                   isNaN(_azimuth) ?                   _noValue : Math.round(_azimuth) % 360
    property string _headingText:                   isNaN(_heading) ?                   _noValue : Math.round(_heading) % 360
    property string _missionPlannedDistanceText:    isNaN(_missionPlannedDistance) ?    _noValue : QGroundControl.unitsConversion.metersToAppSettingsHorizontalDistanceUnits(_missionPlannedDistance).toFixed(0) + " " + QGroundControl.unitsConversion.appSettingsHorizontalDistanceUnitsString
    property string _missionMaxTelemetryText:       isNaN(_missionMaxTelemetry) ?       _noValue : QGroundControl.unitsConversion.metersToAppSettingsHorizontalDistanceUnits(_missionMaxTelemetry).toFixed(0) + " " + QGroundControl.unitsConversion.appSettingsHorizontalDistanceUnitsString
    property string _batteriesRequiredText:         _batteriesRequired < 0 ?            _noValue : _batteriesRequired

    // An em dash, the way every readout in the system shows "there is no number here".
    readonly property string _noValue: "—"

    readonly property var  _qgcPal:      QGroundControl.globalPalette
    readonly property real _capsuleHeight: Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * 2.4)

    property bool   _utmspEnabled:      QGroundControl.utmspSupported

    readonly property var  _activeVehicle:     QGroundControl.multiVehicleManager.activeVehicle
    readonly property bool _communicationLost: _activeVehicle ? _activeVehicle.vehicleLinkManager.communicationLost : false
    property int    _statsPage:         0
    readonly property real _compactNameWidth: ScreenTools.defaultFontPixelWidth * 10
    readonly property real _fullRowWidth:     viewSwitch.Layout.preferredWidth + statusIndicator.implicitWidth + _nameWidth + spacing * 2 +
                                              (_containsItems ? statsRow.width + spacing : 0) +
                                              (uploadButton.visible ? uploadButton.Layout.preferredWidth + spacing : 0)
    readonly property bool _compact:          width < _fullRowWidth

    readonly property real _namePadding:  ScreenTools.defaultFontPixelWidth * 4
    readonly property real _nameMaxWidth: ScreenTools.defaultFontPixelWidth * 28
    readonly property real _nameWidth:    Math.min(nameMetrics.width + _namePadding, _nameMaxWidth)

    readonly property string _planName: {
        if (!_controllerValid) {
            return qsTr("Plan")
        }
        const file = _planMasterController.currentPlanFile
        if (file === "") {
            return qsTr("Untitled Plan")
        }
        const base = file.substring(file.lastIndexOf("/") + 1)
        const dot  = base.lastIndexOf(".")
        return dot > 0 ? base.substring(0, dot) : base
    }

    // One line for the state of the plan against the vehicle, in the place a document's title
    // sits in every other editor. The upload button says the same thing in the same words.
    readonly property string _planState: {
        if (_controllerSyncInProgress) {
            return qsTr("Uploading…")
        }
        const items = _itemCount === 1 ? qsTr("1 item") : qsTr("%1 items").arg(_itemCount)
        if (!_containsItems) {
            return qsTr("Empty plan")
        }
        const state  = _controllerDirty ? qsTr("Edited") : qsTr("Uploaded")
        const figure = _compact && !isNaN(_missionPlannedDistance) ? " · " + _missionPlannedDistanceText : ""
        return state + figure + " · " + items
    }

    function getMissionTime() {
        if (!_missionTime) {
            return "00:00:00"
        }
        var t = new Date(2021, 0, 0, 0, 0, Number(_missionTime))
        var days = Qt.formatDateTime(t, 'dd')
        var complete

        if (days == 31) {
            days = '0'
            complete = Qt.formatTime(t, 'hh:mm:ss')
        } else {
            complete = days + " days " + Qt.formatTime(t, 'hh:mm:ss')
        }
        return complete
    }

    component Stat: Item {
        id: stat

        required property string label
        required property string value

        implicitWidth:  Math.max(labelText.implicitWidth, valueText.implicitWidth) + ScreenTools.defaultFontPixelWidth * 3
        implicitHeight: _root._capsuleHeight
        visible:        value !== _root._noValue

        QGCLabel {
            id:                       labelText
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.top:              parent.top
            anchors.topMargin:        ScreenTools.defaultFontPixelHeight * 0.3
            text:                     stat.label.toUpperCase()
            font.pointSize:           ScreenTools.smallFontPointSize
            font.bold:                true
            color:                    Qt.alpha(_root._qgcPal.overlayInk, 0.45)
        }

        QGCLabel {
            id:                       valueText
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.top:              labelText.bottom
            text:                     stat.value
            font.family:              ScreenTools.fixedFontFamily
        }

        Rectangle {
            anchors.left:           parent.left
            anchors.verticalCenter: parent.verticalCenter
            width:                  1
            height:                 parent.height * 0.55
            color:                  Qt.alpha(_root._qgcPal.overlayInk, 0.12)
            visible:                stat.x > 0
        }
    }

    // The same switch the fly view carries, in the same place, so it stays put while the views
    // dissolve under it.
    OverlayCapsule {
        id:                     viewSwitch
        visible:                !mainWindow.hostProvidesNavigation
        Layout.preferredHeight: _capsuleHeight
        Layout.preferredWidth:  viewSwitchControl.implicitWidth
        Layout.minimumWidth:    Layout.preferredWidth
        radius:                 height / 2

        OverlayViewSwitch {
            id:           viewSwitchControl
            objectName:   "planViewSwitch"
            contentColor: viewSwitch.contentColor
            anchors.fill: parent
            options:      [qsTr("Fly"), qsTr("Plan")]
            currentIndex: mainWindow.flyViewActive ? 0 : 1
            onActivated:  (index) => {
                if (mainWindow.allowViewSwitch()) {
                    index === 0 ? mainWindow.showFlyView() : mainWindow.showPlanView()
                }
            }
        }
    }

    // The one piece of furniture that stays put across both modes, in the same place it sits in
    // the fly view. Planning is planning for a vehicle, so whether that vehicle is there and ready
    // is context the plan needs - and something persisting across the switch is what makes this
    // read as one app changing mode rather than two that happen to share a map.
    MainStatusIndicator {
        id:                     statusIndicator
        Layout.preferredHeight: _capsuleHeight
    }

    // The plan, named, with its state against the vehicle underneath it. Width comes from
    // measured text rather than from the labels themselves - a label that elides to its own
    // container's width cannot also be what sets that width.
    OverlayCapsule {
        objectName:             "planNameCapsule"
        Layout.preferredHeight: _capsuleHeight
        Layout.preferredWidth:  Math.min(Math.max(nameMetrics.width, stateMetrics.width) + _namePadding, _nameMaxWidth)
        Layout.minimumWidth:    _compact ? _compactNameWidth : Layout.preferredWidth
        visible:                _controllerValid

        TextMetrics { id: nameMetrics;  font: nameLabel.font;  text: _root._planName }
        TextMetrics { id: stateMetrics; font: stateLabel.font; text: _root._planState }

        Column {
            id:                     docColumn
            anchors.left:           parent.left
            anchors.right:          parent.right
            anchors.leftMargin:     ScreenTools.defaultFontPixelWidth * 1.5
            anchors.rightMargin:    ScreenTools.defaultFontPixelWidth * 1.5
            anchors.verticalCenter: parent.verticalCenter
            spacing:                0

            QGCLabel {
                id:         nameLabel
                width:      parent.width
                text:       _root._planName
                font.bold:  true
                elide:      Text.ElideMiddle
            }

            QGCLabel {
                id:             stateLabel
                width:          parent.width
                text:           _root._planState
                font.pointSize: ScreenTools.smallFontPointSize
                elide:          Text.ElideRight
                color:          !_controllerDirty && _containsItems && !_controllerSyncInProgress
                                    ? _qgcPal.colorGreen
                                    : Qt.alpha(_qgcPal.overlayInk, 0.5)
            }
        }
    }

    Item { Layout.fillWidth: true }

    // One capsule of figures with hairline dividers instead of a scatter of pills. Tapping it
    // pages to the per-item numbers, which matter while an item is selected and are noise the
    // rest of the time.
    OverlayCapsule {
        id:                     statsCapsule
        objectName:             "planStatsCapsule"
        Layout.preferredHeight: _capsuleHeight
        Layout.preferredWidth:  statsRow.width
        Layout.minimumWidth:    Math.min(statsRow.width, _root.width * 0.45)
        visible:                _containsItems && !_compact
        highlight:              statsMouseArea.containsMouse
        clip:                   true

        Row {
            id:                 statsRow
            anchors.centerIn:   parent

            Stat { label: qsTr("Distance");  value: _missionPlannedDistanceText; visible: _statsPage === 0 }
            Stat { label: qsTr("Time");      value: getMissionTime();            visible: _statsPage === 0 }
            Stat { label: qsTr("Max telem"); value: _missionMaxTelemetryText;    visible: _statsPage === 0 }
            Stat { label: qsTr("Batteries"); value: _batteriesRequiredText;      visible: _statsPage === 0 && _batteryInfoAvailable }

            Stat { label: qsTr("Alt diff");  value: _altDifferenceText;          visible: _statsPage === 1 }
            Stat { label: qsTr("Azimuth");   value: _azimuthText;                visible: _statsPage === 1 }
            Stat { label: qsTr("Heading");   value: _headingText;                visible: _statsPage === 1 }
            Stat { label: qsTr("Gradient");  value: _gradientText;               visible: _statsPage === 1 }
            Stat { label: qsTr("Prev WP");   value: _distanceText;               visible: _statsPage === 1 }
        }

        QGCMouseArea {
            id:             statsMouseArea
            anchors.fill:   parent
            hoverEnabled:   !ScreenTools.isMobile
            onClicked:      _statsPage = _statsPage === 0 ? 1 : 0
        }
    }

    // The only filled action on screen. Progress runs inside it rather than as a separate bar,
    // so the thing you pressed is the thing that reports back.
    Rectangle {
        id:                     uploadButton
        objectName:             "planUploadButton"
        Layout.preferredHeight: _capsuleHeight
        Layout.preferredWidth:  uploadLabel.implicitWidth + ScreenTools.defaultFontPixelWidth * 5
        Layout.minimumWidth:    Layout.preferredWidth
        radius:                 height / 2
        // With no vehicle there is nothing to upload to. Dimmed-to-invisible on glass reads as
        // a rendering fault; an action that cannot apply is simply not offered.
        visible:                !QGroundControl.corePlugin.options.disableVehicleConnection && !_controllerOffline
        clip:                   true
        color:                  !_uploadEnabled  ? Qt.alpha(_qgcPal.overlayInk, 0.08)
                              : _uploadedClean   ? _qgcPal.colorGreen
                                                 : _qgcPal.primaryButton

        readonly property bool _uploadEnabled:  _controllerValid && !_controllerOffline && !_controllerSyncInProgress && _containsItems &&
                                                    (!_utmspEnabled || UTMSPStateStorage.enableMissionUploadButton)
        readonly property bool _uploadedClean:  _controllerValid && !_controllerDirty && _containsItems && !_controllerOffline && !_controllerSyncInProgress

        Behavior on color { ColorAnimation { duration: 160 } }

        Rectangle {
            anchors.top:    parent.top
            anchors.bottom: parent.bottom
            anchors.left:   parent.left
            width:          parent.width * _progressPct
            color:          Qt.alpha(_qgcPal.overlayInk, 0.28)
            visible:        _controllerSyncInProgress
        }

        QGCLabel {
            id:                 uploadLabel
            anchors.centerIn:   parent
            font.bold:          true
            color:              uploadButton._uploadEnabled || uploadButton._uploadedClean
                                    ? _qgcPal.primaryButtonText
                                    : Qt.alpha(_qgcPal.overlayInk, 0.4)
            text:               _controllerSyncInProgress ? qsTr("Uploading…")
                              : uploadButton._uploadedClean ? qsTr("✓ Uploaded")
                                                            : qsTr("Upload")
        }

        QGCMouseArea {
            anchors.fill:   parent
            enabled:        uploadButton._uploadEnabled
            onClicked: {
                if (_utmspEnabled) {
                    QGroundControl.utmspManager.utmspVehicle.triggerActivationStatusBar(true);
                    UTMSPStateStorage.removeFlightPlanState = true
                    UTMSPStateStorage.indicatorDisplayStatus = true
                }
                _planMasterController.upload();
            }
        }
    }
}
