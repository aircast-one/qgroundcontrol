/****************************************************************************
 *
 * (c) 2009-2022 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.MultiVehicleManager
import QGroundControl.ScreenTools
import QGroundControl.Palette
import QGroundControl.FactSystem
import QGroundControl.FactControls

ToolIndicatorPage {
    id:         control
    showExpand: true

    property var    linkConfigs:            QGroundControl.linkManager.linkConfigurations
    property bool   noLinks:                true
    property var    editingConfig:          null
    property var    autoConnectSettings:    QGroundControl.settingsManager.autoConnectSettings

    Component.onCompleted: {
        for (var i = 0; i < linkConfigs.count; i++) {
            var linkConfig = linkConfigs.get(i)
            if (!linkConfig.dynamic && !linkConfig.isAutoConnect) {
                noLinks = false
                break
            }
        }
    }

    // What auto-connect is currently listening on, in the user's words. Without it the pill
    // says "Not connected" and nothing on screen says whether anything is being attempted -
    // so there is no way to tell whether to wait, replug, or go and configure a link.
    readonly property var _watching: [
        autoConnectSettings.autoConnectPixhawk.rawValue    ? qsTr("USB")        : "",
        autoConnectSettings.autoConnectSiKRadio.rawValue   ? qsTr("SiK radio")  : "",
        autoConnectSettings.autoConnectUDP.rawValue        ? qsTr("UDP %1").arg(autoConnectSettings.udpListenPort.rawValue) : "",
        autoConnectSettings.autoConnectZeroConf.rawValue   ? qsTr("Zero-Conf")  : ""
    ].filter((entry) => entry !== "")

    contentComponent: Component {
        ColumnLayout {
            spacing: ScreenTools.defaultFontPixelHeight / 2

            SettingsGroupLayout {
                popoverStyle: true
                heading: qsTr("Select Link to Connect")

                QGCLabel {
                    Layout.fillWidth:   true
                    wrapMode:           Text.WordWrap
                    text:               qsTr("No links are configured yet.")
                    visible:            noLinks
                }

                // The way forward used to live behind the expand chevron, so a first run
                // followed the only instruction on screen and arrived at a sentence with
                // nothing to press.
                QGCButton {
                    Layout.fillWidth:   true
                    text:               qsTr("Configure a Link")
                    visible:            noLinks

                    onClicked: {
                        mainWindow.showSettingsTool(qsTr("Comm Links"))
                        mainWindow.closeIndicatorDrawer()
                    }
                }

                Repeater {
                    model: linkConfigs

                    delegate: QGCButton {
                        Layout.fillWidth:   true
                        text:               object.name + (object.link ? " (" + qsTr("Connected") + ")" : "")
                        visible:            !object.dynamic
                        enabled:            !object.link
                        autoExclusive:      true

                        onClicked: {
                            QGroundControl.linkManager.createConnectedLink(object)
                            mainWindow.closeIndicatorDrawer()
                        }
                    }
                }
            }

            QGCLabel {
                Layout.fillWidth:   true
                wrapMode:           Text.WordWrap
                font.pointSize:     ScreenTools.smallFontPointSize
                color:              QGroundControl.globalPalette.colorGrey
                visible:            control._watching.length > 0
                text:               qsTr("Watching for a vehicle on %1.").arg(control._watching.join(", "))
            }
        }
    }

    expandedComponent: Component {
        ColumnLayout {
            spacing: ScreenTools.defaultFontPixelHeight / 2

            SettingsGroupLayout {
                popoverStyle: true
                LabelledButton {
                    label:      qsTr("Communication Links")
                    buttonText: qsTr("Configure")

                    onClicked: {
                        mainWindow.showSettingsTool(qsTr("Comm Links"))
                        mainWindow.closeIndicatorDrawer()
                    }
                }
            }

            SettingsGroupLayout {
                popoverStyle: true
                heading:        qsTr("AutoConnect")
                visible:        autoConnectSettings.visible

                Repeater {
                    id: autoConnectRepeater

                    model: [
                        autoConnectSettings.autoConnectPixhawk,
                        autoConnectSettings.autoConnectSiKRadio,
                        autoConnectSettings.autoConnectLibrePilot,
                        autoConnectSettings.autoConnectUDP,
                        autoConnectSettings.autoConnectZeroConf,
                        autoConnectSettings.autoConnectRTKGPS,
                    ]

                    property var names: [ qsTr("Pixhawk"), qsTr("SiK Radio"), qsTr("LibrePilot"), qsTr("UDP"), qsTr("Zero-Conf"), qsTr("RTK") ]

                    FactCheckBoxSlider {
                        Layout.fillWidth:   true
                        text:               autoConnectRepeater.names[index]
                        fact:               modelData
                        visible:            modelData.visible
                    }
                }
            }
        }
    }
}
