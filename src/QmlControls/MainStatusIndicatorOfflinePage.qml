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
    expandText: qsTr("Connection Settings")

    property var    linkConfigs:            QGroundControl.linkManager.linkConfigurations
    property bool   noLinks:                true
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

    readonly property bool _watching: autoConnectSettings.autoConnectPixhawk.rawValue
                                   || autoConnectSettings.autoConnectSiKRadio.rawValue
                                   || autoConnectSettings.autoConnectUDP.rawValue
                                   || autoConnectSettings.autoConnectZeroConf.rawValue

    function openLinkSettings() {
        mainWindow.showSettingsTool(qsTr("Comm Links"))
        mainWindow.closeIndicatorDrawer()
    }

    contentComponent: Component {
        ColumnLayout {
            spacing: ScreenTools.defaultFontPixelHeight * 0.75

            readonly property real _margin: ScreenTools.defaultFontPixelWidth * 1.5

            ColumnLayout {
                Layout.fillWidth:   true
                Layout.minimumWidth: ScreenTools.defaultFontPixelWidth * 28
                Layout.leftMargin:  parent._margin
                Layout.rightMargin: parent._margin
                spacing:            ScreenTools.defaultFontPixelHeight / 6

                QGCLabel {
                    text:           qsTr("Not Connected")
                    font.pointSize: ScreenTools.mediumFontPointSize
                    font.bold:      true
                }

                RowLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelWidth

                    OverlayActivityRing {
                        width:          ScreenTools.defaultFontPixelHeight * 0.8
                        visible:        control._watching
                        contentColor:   QGroundControl.globalPalette.colorGrey
                    }

                    QGCLabel {
                        objectName:         "connectionFootnote"
                        Layout.fillWidth:   true
                        wrapMode:           Text.WordWrap
                        font.pointSize:     ScreenTools.smallFontPointSize
                        color:              QGroundControl.globalPalette.colorGrey
                        text:               control._watching ? qsTr("Looking for a vehicle…")
                                                              : qsTr("Automatic connection is off.")
                    }
                }
            }

            QGCButton {
                objectName:         "addLinkButton"
                Layout.fillWidth:   true
                Layout.leftMargin:  parent._margin
                Layout.rightMargin: parent._margin
                text:               qsTr("Add Link…")
                primary:            true
                visible:            noLinks
                onClicked:          control.openLinkSettings()
            }

            ColumnLayout {
                Layout.fillWidth:   true
                spacing:            0
                visible:            !noLinks

                Repeater {
                    model: linkConfigs

                    delegate: OverlayMenuItem {
                        text:       object.name
                        checkable:  true
                        checked:    object.link ? true : false
                        visible:    !object.dynamic
                        onClicked: {
                            if (object.link) {
                                return
                            }
                            QGroundControl.linkManager.createConnectedLink(object)
                            mainWindow.closeIndicatorDrawer()
                        }
                    }
                }
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
                    onClicked:  control.openLinkSettings()
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
