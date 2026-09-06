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

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FactControls
import QGroundControl.ScreenTools
import QGroundControl.Palette

SettingsPage {
    property var _linkManager:          QGroundControl.linkManager
    property var _autoConnectSettings:  QGroundControl.settingsManager.autoConnectSettings

    SettingsGroupLayout {
        heading:        qsTr("Auto Connect")
        visible:        _autoConnectSettings.visible

        Repeater {
            id: autoConnectRepeater

            model: [
                _autoConnectSettings.autoConnectPixhawk,
                _autoConnectSettings.autoConnectSiKRadio,
                _autoConnectSettings.autoConnectLibrePilot,
                _autoConnectSettings.autoConnectUDP,
                _autoConnectSettings.autoConnectZeroConf,
                _autoConnectSettings.autoConnectRTKGPS,
            ]

            property var names: [ qsTr("Pixhawk"), qsTr("SiK Radio"), qsTr("LibrePilot"), qsTr("UDP"), qsTr("Zero-Conf"), qsTr("RTK") ]

            property var descriptions: [
                qsTr("Flight controllers plugged in over USB."),
                qsTr("Telemetry radios plugged in over USB."),
                qsTr("LibrePilot boards plugged in over USB."),
                qsTr("Vehicles broadcasting to this computer over the network."),
                qsTr("Vehicles that announce themselves on the local network."),
                qsTr("RTK base stations plugged in over USB.")
            ]

            RowLayout {
                Layout.fillWidth:   true
                spacing:            ScreenTools.defaultFontPixelWidth * 2
                visible:            modelData.visible

                ColumnLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelHeight * 0.1

                    QGCLabel { text: autoConnectRepeater.names[index] }

                    QGCLabel {
                        Layout.fillWidth:   true
                        wrapMode:           Text.WordWrap
                        font.pointSize:     ScreenTools.smallFontPointSize
                        color:              QGroundControl.globalPalette.colorGrey
                        text:               autoConnectRepeater.descriptions[index]
                    }
                }

                FactCheckBoxSlider { fact: modelData }
            }
        }
    }

    SettingsGroupLayout {
        heading: qsTr("NMEA GPS")
        visible: QGroundControl.settingsManager.autoConnectSettings.autoConnectNmeaPort.visible && QGroundControl.settingsManager.autoConnectSettings.autoConnectNmeaBaud.visible

        LabelledComboBox {
            id: nmeaPortCombo
            label: qsTr("Device")

            model: ListModel {}

            onActivated: (index) => {
                if (index !== -1) {
                    QGroundControl.settingsManager.autoConnectSettings.autoConnectNmeaPort.value = comboBox.textAt(index);
                }
            }

            Component.onCompleted: {
                var model = []

                model.push(qsTr("Disabled"))
                model.push(qsTr("UDP Port"))

                if (QGroundControl.linkManager.serialPorts.length === 0) {
                    model.push(qsTr("Serial <none available>"))
                } else {
                    for (var i in QGroundControl.linkManager.serialPorts) {
                        model.push(QGroundControl.linkManager.serialPorts[i])
                    }
                }
                nmeaPortCombo.model = model

                const index = nmeaPortCombo.comboBox.find(QGroundControl.settingsManager.autoConnectSettings.autoConnectNmeaPort.valueString);
                nmeaPortCombo.currentIndex = index;
            }
        }

        LabelledComboBox {
            id: nmeaBaudCombo
            visible: (nmeaPortCombo.currentText !== "UDP Port") && (nmeaPortCombo.currentText !== "Disabled")
            label: qsTr("Baudrate")
            model: QGroundControl.linkManager.serialBaudRates

            onActivated: (index) => {
                if (index !== -1) {
                    QGroundControl.settingsManager.autoConnectSettings.autoConnectNmeaBaud.value = parseInt(comboBox.textAt(index));
                }
            }

            Component.onCompleted: {
                const index = nmeaBaudCombo.comboBox.find(QGroundControl.settingsManager.autoConnectSettings.autoConnectNmeaBaud.valueString);
                nmeaBaudCombo.currentIndex = index;
            }
        }

        LabelledFactTextField {
            visible: nmeaPortCombo.currentText === "UDP Port"
            label: qsTr("NMEA stream UDP port")
            fact: QGroundControl.settingsManager.autoConnectSettings.nmeaUdpPort
        }
    }

    SettingsGroupLayout {
        heading: qsTr("Links")

        Repeater {
            model: _linkManager.linkConfigurations

            RowLayout {
                id:                 linkRow
                Layout.fillWidth:   true
                visible:            !object.dynamic

                readonly property bool _actionsVisible: ScreenTools.isMobile || linkRowHover.hovered

                HoverHandler { id: linkRowHover }

                QGCLabel {
                    Layout.fillWidth:   true
                    elide:              Text.ElideRight
                    text:               object.name
                }
                QGCColoredImage {
                    height:                 ScreenTools.minTouchPixels
                    width:                  height
                    sourceSize.height:      height
                    fillMode:               Image.PreserveAspectFit
                    mipmap:                 true
                    smooth:                 true
                    color:                  qgcPalEdit.text
                    source:                 "/res/pencil.svg"
                    enabled:                linkRow._actionsVisible && !object.link
                    opacity:                linkRow._actionsVisible ? 1 : 0

                    Behavior on opacity { NumberAnimation { duration: 120 } }

                    QGCPalette {
                        id: qgcPalEdit
                        colorGroupEnabled: parent.enabled
                    }

                    QGCMouseArea {
                        fillItem: parent
                        onClicked: {
                            var editingConfig = _linkManager.startConfigurationEditing(object)
                            linkDialogComponent.createObject(mainWindow, { editingConfig: editingConfig, originalConfig: object }).open()
                        }
                    }
                }
                QGCColoredImage {
                    height:                 ScreenTools.minTouchPixels
                    width:                  height
                    sourceSize.height:      height
                    fillMode:               Image.PreserveAspectFit
                    mipmap:                 true
                    smooth:                 true
                    color:                  qgcPalDelete.text
                    source:                 "/res/TrashDelete.svg"
                    enabled:                linkRow._actionsVisible
                    opacity:                linkRow._actionsVisible ? 1 : 0

                    Behavior on opacity { NumberAnimation { duration: 120 } }

                    QGCPalette {
                        id: qgcPalDelete
                        colorGroupEnabled: parent.enabled
                    }

                    QGCMouseArea {
                        fillItem:   parent
                        onClicked:  mainWindow.showMessageDialog(
                                        qsTr("Delete Link"),
                                        qsTr("Are you sure you want to delete '%1'?").arg(object.name),
                                        Dialog.Ok | Dialog.Cancel,
                                        function () {
                                            _linkManager.removeConfiguration(object)
                                        })
                    }
                }
                QGCButton {
                    text:       object.link ? qsTr("Disconnect") : qsTr("Connect")
                    onClicked: {
                        if (object.link) {
                            object.link.disconnect()
                        } else {
                            _linkManager.createConnectedLink(object)
                        }
                    }
                }
            }
        }

        LabelledButton {
            objectName: "addLinkButton"
            label:      ""
            buttonText: qsTr("Add Link…")

            onClicked: {
                var editingConfig = _linkManager.createConfiguration(ScreenTools.isSerialAvailable ? LinkConfiguration.TypeSerial : LinkConfiguration.TypeUdp, "")
                linkDialogComponent.createObject(mainWindow, { editingConfig: editingConfig, originalConfig: null }).open()
            }
        }
    }

    Component {
        id: linkDialogComponent

        QGCPopupDialog {
            id:                     linkDialog
            title:                  originalConfig ? qsTr("Edit Link") : qsTr("Add Link")
            buttons:                Dialog.Save | Dialog.Cancel
            acceptButtonTitle:      originalConfig && originalConfig.link ? qsTr("Save") : qsTr("Save & Connect")

            property var originalConfig
            property var editingConfig

            readonly property real _labelWidth:   ScreenTools.defaultFontPixelWidth * 14
            readonly property real _rowSpacing:   ScreenTools.defaultFontPixelWidth * 2

            function _uniqueName(base) {
                const taken = Array.from({ length: _linkManager.linkConfigurations.count },
                                         (_, i) => _linkManager.linkConfigurations.get(i))
                    .filter(config => config && config !== originalConfig)
                    .map(config => config.name)
                const candidates = [ base ].concat(
                    Array.from({ length: taken.length }, (_, i) => qsTr("%1 (%2)").arg(base).arg(i + 2)))
                return candidates.find(candidate => taken.indexOf(candidate) === -1)
            }

            function _suggestedName() {
                const item = linkSettingsLoader.item
                const suggested = item ? item.suggestedName() : ""
                return _uniqueName(suggested !== "" ? suggested : _linkManager.linkTypeStrings[editingConfig.linkType])
            }

            function _resolvedName() {
                const typed = nameField.text.trim()
                return typed !== "" ? typed : _suggestedName()
            }

            onAccepted: {
                if (linkSettingsLoader.item.validate && !linkSettingsLoader.item.validate()) {
                    preventClose = true
                    return
                }
                linkSettingsLoader.item.saveSettings()
                editingConfig.name = _resolvedName()
                if (originalConfig) {
                    _linkManager.endConfigurationEditing(originalConfig, editingConfig)
                    if (!originalConfig.link) {
                        _linkManager.createConnectedLink(originalConfig)
                    }
                } else {
                    editingConfig.dynamic = false
                    _linkManager.endCreateConfiguration(editingConfig)
                    _linkManager.createConnectedLink(editingConfig)
                }
            }

            onRejected: _linkManager.cancelConfigurationEditing(editingConfig)

            component OptionRow: RowLayout {
                property alias title:       titleLabel.text
                property alias description: descriptionLabel.text
                property alias checked:     toggle.checked

                signal toggled(bool checked)

                Layout.fillWidth:   true
                spacing:            linkDialog._rowSpacing

                ColumnLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelHeight * 0.1

                    QGCLabel {
                        id:                 titleLabel
                        Layout.fillWidth:   true
                    }

                    QGCLabel {
                        id:                 descriptionLabel
                        Layout.fillWidth:   true
                        wrapMode:           Text.WordWrap
                        font.pointSize:     ScreenTools.smallFontPointSize
                        color:              QGroundControl.globalPalette.colorGrey
                        visible:            text !== ""
                    }
                }

                QGCCheckBoxSlider {
                    id:         toggle
                    onClicked:  parent.toggled(checked)
                }
            }

            ColumnLayout {
                width:      ScreenTools.defaultFontPixelWidth * 60
                spacing:    ScreenTools.defaultFontPixelHeight * 0.8

                Component.onCompleted: nameField.forceActiveFocus()

                SettingsGroupLayout {
                    Layout.fillWidth:   true
                    heading:            qsTr("Link")
                    popoverStyle:       true
                    cardStyle:          true

                    RowLayout {
                        Layout.fillWidth:   true
                        spacing:            linkDialog._rowSpacing

                        QGCLabel {
                            Layout.preferredWidth:  linkDialog._labelWidth
                            text:                   qsTr("Name (optional)")
                        }

                        QGCTextField {
                            id:                 nameField
                            Layout.fillWidth:   true
                            text:               editingConfig.name
                            placeholderText:    linkDialog._suggestedName()
                        }
                    }

                    RowLayout {
                        Layout.fillWidth:   true
                        spacing:            linkDialog._rowSpacing

                        QGCLabel {
                            Layout.preferredWidth:  linkDialog._labelWidth
                            text:                   qsTr("Type")
                        }

                        QGCComboBox {
                            Layout.fillWidth:       true
                            enabled:                originalConfig == null
                            model:                  _linkManager.linkTypeStrings
                            Component.onCompleted:  currentIndex = editingConfig.linkType

                            onActivated: (index) => {
                                if (index !== editingConfig.linkType) {
                                    editingConfig = _linkManager.createConfiguration(index, nameField.text)
                                }
                            }
                        }
                    }

                    Loader {
                        id:                 linkSettingsLoader
                        Layout.fillWidth:   true
                        source:             subEditConfig.settingsURL

                        property var subEditConfig:         editingConfig
                        property int _firstColumnWidth:     linkDialog._labelWidth
                        property int _secondColumnWidth:    width - linkDialog._labelWidth - linkDialog._rowSpacing
                        property int _rowSpacing:           ScreenTools.defaultFontPixelHeight / 2
                        property int _colSpacing:           linkDialog._rowSpacing
                    }
                }

                SettingsGroupLayout {
                    Layout.fillWidth:   true
                    heading:            qsTr("Options")
                    popoverStyle:       true
                    cardStyle:          true

                    OptionRow {
                        title:          qsTr("Connect on Start")
                        description:    qsTr("Open this link when the app launches.")
                        checked:        editingConfig.autoConnect
                        onToggled:      (checked) => editingConfig.autoConnect = checked
                    }

                    OptionRow {
                        title:          qsTr("High Latency")
                        description:    qsTr("Tune the link for satellite or cellular round trips.")
                        checked:        editingConfig.highLatency
                        onToggled:      (checked) => editingConfig.highLatency = checked
                    }
                }
            }
        }
    }
}
