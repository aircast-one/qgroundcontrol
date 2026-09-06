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

ToolIndicatorPage {
    id:         control
    showExpand: false

    property var    linkConfigs:            QGroundControl.linkManager.linkConfigurations
    property var    autoConnectSettings:    QGroundControl.settingsManager.autoConnectSettings

    readonly property bool noLinks: !Array.from({ length: linkConfigs.count }, (_, i) => linkConfigs.get(i))
                                          .some((config) => !config.dynamic)

    readonly property bool _watching: autoConnectSettings.autoConnectPixhawk.rawValue
                                   || autoConnectSettings.autoConnectSiKRadio.rawValue
                                   || autoConnectSettings.autoConnectUDP.rawValue
                                   || autoConnectSettings.autoConnectZeroConf.rawValue

    readonly property string _watchedLinks: [
        autoConnectSettings.autoConnectPixhawk.rawValue  ? qsTr("USB")       : "",
        autoConnectSettings.autoConnectSiKRadio.rawValue ? qsTr("SiK radio") : "",
        autoConnectSettings.autoConnectUDP.rawValue      ? qsTr("Wi‑Fi")     : "",
        autoConnectSettings.autoConnectZeroConf.rawValue ? qsTr("local network")   : ""
    ].filter((name) => name !== "").join(", ")

    function _markup(name) { return name.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;") }

    readonly property string _connectingLinkName: _markup(QGroundControl.linkManager.connectingLinkName)
    readonly property var    _failedLink:         QGroundControl.linkManager.failedLink
    readonly property string _failedLinkName:     _failedLink ? _markup(_failedLink.name) : ""
    readonly property bool   _failedInSettings:   _failedLink ? _failedLink.lastErrorRemedy === LinkConfiguration.RemedyEditAddress : false
    readonly property bool   _stalled:            QGroundControl.linkManager.connectingStalled
    readonly property bool   _busy:               (_connectingLinkName !== "" && !_stalled) || (noLinks && _watching)

    readonly property string _title: _connectingLinkName !== "" ? qsTr("Connecting…")
                                   : _failedLinkName !== ""     ? qsTr("Connection Failed")
                                                                : qsTr("Not Connected")

    readonly property string _footnote: _connectingLinkName !== "" && _stalled ? qsTr("No telemetry from %1 yet. Check the port is the drone's MAVLink port.").arg(_connectingLinkName)
                                      : _connectingLinkName !== ""             ? qsTr("Connecting to %1…").arg(_connectingLinkName)
                                      : _failedLinkName !== "" && _failedInSettings ? qsTr("Couldn't connect to %1. <a href=\"settings\">Check its address in Connection Settings</a>.").arg(_failedLinkName)
                                      : _failedLinkName !== ""                 ? qsTr("Couldn't connect to %1. Check the drone is powered on and on the same network, then tap it to retry.").arg(_failedLinkName)
                                      : !_watching                             ? qsTr("Automatic connection is off.")
                                      : noLinks                                ? qsTr("Looking for a vehicle on %1…").arg(_watchedLinks)
                                                                               : qsTr("Auto-connect watches %1. Tap a link to connect.").arg(_watchedLinks)

    onActiveVehicleChanged: {
        if (activeVehicle) {
            mainWindow.closeIndicatorDrawer()
        }
    }

    function openLinkSettings() {
        mainWindow.showCommLinkSettings()
        mainWindow.closeIndicatorDrawer()
    }

    contentComponent: Component {
        ColumnLayout {
            spacing: 0

            readonly property real _margin: ScreenTools.defaultFontPixelWidth * 1.5

            ColumnLayout {
                Layout.fillWidth:   true
                Layout.minimumWidth: ScreenTools.defaultFontPixelWidth * 28
                Layout.leftMargin:  parent._margin
                Layout.rightMargin: parent._margin
                Layout.bottomMargin: ScreenTools.defaultFontPixelHeight * 0.25
                spacing:            ScreenTools.defaultFontPixelHeight / 6

                QGCLabel {
                    text:           control._title
                    font.pointSize: ScreenTools.mediumFontPointSize
                    font.bold:      true
                }

                RowLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelWidth

                    OverlayActivityRing {
                        width:          ScreenTools.defaultFontPixelHeight * 0.8
                        visible:        control._busy
                        contentColor:   QGroundControl.globalPalette.colorGrey
                    }

                    QGCLabel {
                        objectName:         "connectionFootnote"
                        Layout.fillWidth:   true
                        wrapMode:           Text.WordWrap
                        font.pointSize:     ScreenTools.smallFontPointSize
                        color:              Qt.alpha(QGroundControl.globalPalette.text, 0.65)
                        linkColor:          QGroundControl.globalPalette.text
                        textFormat:         Text.StyledText
                        text:               control._footnote
                        onLinkActivated:    control.openLinkSettings()

                        QGCMouseArea {
                            objectName:             "connectionFootnoteTap"
                            anchors.fill:           parent
                            anchors.topMargin:      -Math.max(0, (ScreenTools.minTouchPixels - parent.height) / 2)
                            anchors.bottomMargin:   anchors.topMargin
                            enabled:                control._failedInSettings
                            onClicked:              control.openLinkSettings()
                        }
                    }
                }
            }

            QGCButton {
                objectName:         "addLinkButton"
                Layout.fillWidth:   true
                Layout.leftMargin:  parent._margin
                Layout.rightMargin: parent._margin
                Layout.topMargin:   ScreenTools.defaultFontPixelHeight * 0.5
                Layout.bottomMargin: ScreenTools.defaultFontPixelHeight * 0.25
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
                        text:           object.link ? object.name : qsTr("Connect to %1").arg(object.name)
                        description:    object.lastError !== "" ? object.summary + "\n" + object.lastError : object.summary
                        detail:         object.lastError !== "" && !object.link ? qsTr("Retry") : ""
                        current:        object.lastError !== ""
                        checkable:      true
                        busy:           !!object.link
                        checked:        busy
                        visible:        !object.dynamic
                        onClicked:      object.link ? object.link.disconnect()
                                                    : QGroundControl.linkManager.createConnectedLink(object)
                    }
                }
            }

            OverlayMenuSeparator { Layout.fillWidth: true }

            OverlayMenuItem {
                objectName:         "connectionSettingsItem"
                Layout.fillWidth:   true
                reserveGutter:      true
                text:               qsTr("Connection Settings")
                onClicked:          control.openLinkSettings()
            }
        }
    }
}
