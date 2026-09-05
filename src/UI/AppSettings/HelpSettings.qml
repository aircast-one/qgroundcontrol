/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.ScreenTools

SettingsPage {
    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("About")

        RowLayout {
            Layout.fillWidth:       true
            Layout.preferredHeight: ScreenTools.settingsRowHeight
            spacing:                ScreenTools.defaultFontPixelWidth * 2

            QGCLabel {
                Layout.fillWidth:   true
                text:               qsTr("%1 Version").arg(QGroundControl.appName)
            }

            QGCLabel {
                id:         versionLabel
                text:       QGroundControl.qgcVersion
                color:      QGroundControl.globalPalette.colorGrey

                QGCMouseArea {
                    anchors.fill:   parent
                    onClicked: (mouse) => {
                        if (mouse.modifiers & Qt.ControlModifier) {
                            QGroundControl.corePlugin.showTouchAreas = !QGroundControl.corePlugin.showTouchAreas
                        } else if (ScreenTools.isMobile || mouse.modifiers & Qt.ShiftModifier) {
                            QGroundControl.corePlugin.showAdvancedUI = !QGroundControl.corePlugin.showAdvancedUI
                        }
                    }
                    onPressAndHold: QGroundControl.corePlugin.showTouchAreas = !QGroundControl.corePlugin.showTouchAreas
                }
            }
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth: true

        Repeater {
            model: [
                { name: qsTr("QGroundControl User Guide"),        url: "https://docs.qgroundcontrol.com" },
                { name: qsTr("PX4 Users Discussion Forum"),       url: "http://discuss.px4.io/c/qgroundcontrol" },
                { name: qsTr("ArduPilot Users Discussion Forum"), url: "https://discuss.ardupilot.org/c/ground-control-software/qgroundcontrol" },
                { name: qsTr("QGroundControl Discord Channel"),   url: "https://discord.com/channels/1022170275984457759/1022185820683255908" },
            ]

            RowLayout {
                Layout.fillWidth:       true
                Layout.preferredHeight: ScreenTools.settingsRowHeight
                spacing:                ScreenTools.defaultFontPixelWidth * 2

                QGCLabel {
                    Layout.fillWidth:   true
                    text:               modelData.name
                }

                QGCLabel {
                    linkColor:          QGroundControl.globalPalette.colorBlue
                    text:               "<a href=\"" + modelData.url + "\">" + modelData.url.replace(/^https?:\/\//, "").split("/")[0] + "</a>"
                    onLinkActivated:    (link) => Qt.openUrlExternally(link)
                }
            }
        }
    }
}
