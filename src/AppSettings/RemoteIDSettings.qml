/****************************************************************************
 *
 * (c) 2009-2022 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.FactControls
import QGroundControl.Controls

SettingsPage {
    readonly property var  _ridSettings:        QGroundControl.settingsManager.remoteIDSettings
    readonly property bool _isEURegion:         _ridSettings.region.rawValue === RemoteIDSettings.RegionOperation.EU
    readonly property bool _isFAARegion:        _ridSettings.region.rawValue === RemoteIDSettings.RegionOperation.FAA
    readonly property bool _sendBasicID:        _ridSettings.sendBasicID.rawValue
    readonly property bool _sendSelfID:         _ridSettings.sendSelfID.rawValue
    readonly property bool _locationIsFixed:    _ridSettings.locationType.rawValue === RemoteIDSettings.LocationType.FIXED
    readonly property bool _classificationIsEU: _ridSettings.classificationType.rawValue === RemoteIDSettings.ClassificationType.EU

    component SettingCombo: LabelledFactComboBox {
        Layout.fillWidth:   true
        label:              fact.shortDescription
        indexModel:         false
        visible:            fact.userVisible
    }

    component SettingField: LabelledFactTextField {
        Layout.fillWidth:   true
        objectName:         "settingsTextField_" + fact.name
        visible:            fact.userVisible
    }

    component SettingSwitch: FactCheckBoxSlider {
        Layout.fillWidth:   true
        objectName:         "settingsCheckBox_" + fact.name
        text:               fact.shortDescription
        visible:            fact.userVisible
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Region")

        SettingCombo { fact: _ridSettings.region }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Basic ID")

        SettingSwitch { fact: _ridSettings.sendBasicID }
        SettingCombo  { fact: _ridSettings.basicIDType;   enabled: _sendBasicID }
        SettingCombo  { fact: _ridSettings.basicIDUaType; enabled: _sendBasicID }
        SettingField  { fact: _ridSettings.basicID;       enabled: _sendBasicID }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Operator ID")

        SettingSwitch { fact: _ridSettings.sendOperatorID; enabled: _isFAARegion }
        SettingCombo  { fact: _ridSettings.operatorIDType; visible: fact.userVisible && fact.enumValues.length > 1 }
        SettingField  { fact: _ridSettings.operatorIDEU;   visible: fact.userVisible && _isEURegion }
        SettingField  { fact: _ridSettings.operatorIDFAA;  visible: fact.userVisible && !_isEURegion }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Self ID")

        SettingSwitch { fact: _ridSettings.sendSelfID }
        SettingCombo  { fact: _ridSettings.selfIDType;     enabled: _sendSelfID }
        SettingField  { fact: _ridSettings.selfIDFree;     enabled: _sendSelfID }
        SettingField  { fact: _ridSettings.selfIDExtended; enabled: _sendSelfID }
        SettingField  { fact: _ridSettings.selfIDEmergency }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Ground Station Location")

        SettingCombo { fact: _ridSettings.locationType }
        SettingField { fact: _ridSettings.latitudeFixed;  enabled: _locationIsFixed }
        SettingField { fact: _ridSettings.longitudeFixed; enabled: _locationIsFixed }
        SettingField { fact: _ridSettings.altitudeFixed;  enabled: _locationIsFixed }
    }

    GcsPositionStatus {
        Layout.fillWidth: true
    }

    RemoteIDGpsLocation { }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        objectName:         "settingsGroup_EUVehicleInfo"
        heading:            qsTr("EU Vehicle Info")
        visible:            _isEURegion

        SettingCombo { fact: _ridSettings.classificationType }
        SettingCombo { fact: _ridSettings.categoryEU; enabled: _classificationIsEU }
        SettingCombo { fact: _ridSettings.classEU;    enabled: _classificationIsEU }
    }
}
