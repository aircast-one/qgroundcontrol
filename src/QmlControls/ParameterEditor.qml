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
import QGroundControl.Palette
import QGroundControl.ScreenTools
import QGroundControl.Controllers
import QGroundControl.FactSystem
import QGroundControl.FactControls

Item {
    id:         _root

    property Fact   _editorDialogFact: Fact { }
    property bool   _searchFilter:      searchText.text.trim() != "" || controller.showModifiedOnly
    property var    _activeVehicle:     QGroundControl.multiVehicleManager.activeVehicle
    property bool   _showRCToParam:     _activeVehicle.px4Firmware
    property var    _appSettings:       QGroundControl.settingsManager.appSettings
    property var    _controller:        controller
    property string initialSearchText

    Component.onCompleted: searchText.text = initialSearchText

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

    ParameterEditorController {
        id: controller
    }

    QGCMenu {
        id:                 toolsMenu
        QGCMenuItem {
            text:           qsTr("Refresh")
            onTriggered:	controller.refresh()
        }
        QGCMenuItem {
            text:           qsTr("Reset all to firmware's defaults")
            onTriggered:    mainWindow.showMessageDialog(qsTr("Reset All"),
                                                         qsTr("Select Reset to reset all parameters to their defaults.\n\nNote that this will also completely reset everything, including UAVCAN nodes, all vehicle settings, setup and calibrations."),
                                                         Dialog.Cancel | Dialog.Reset,
                                                         function() { controller.resetAllToDefaults() })
        }
        QGCMenuItem {
            text:           qsTr("Reset to vehicle's configuration defaults")
            visible:        !_activeVehicle.apmFirmware
            onTriggered:    mainWindow.showMessageDialog(qsTr("Reset All"),
                                                         qsTr("Select Reset to reset all parameters to the vehicle's configuration defaults."),
                                                         Dialog.Cancel | Dialog.Reset,
                                                         function() { controller.resetAllToVehicleConfiguration() })
        }
        QGCMenuSeparator { }
        QGCMenuItem {
            text:           qsTr("Load from file for review...")
            onTriggered: {
                fileDialog.title =          qsTr("Load Parameters")
                fileDialog.openForLoad()
            }
        }
        QGCMenuItem {
            text:           qsTr("Save to file...")
            onTriggered: {
                fileDialog.title =          qsTr("Save Parameters")
                fileDialog.openForSave()
            }
        }
        QGCMenuSeparator { visible: _showRCToParam }
        QGCMenuItem {
            text:           qsTr("Clear all RC to Param")
            onTriggered:	_activeVehicle.clearAllParamMapRC()
            visible:        _showRCToParam
        }
        QGCMenuSeparator { }
        QGCMenuItem {
            text:           qsTr("Reboot Vehicle")
            onTriggered:    mainWindow.showMessageDialog(qsTr("Reboot Vehicle"),
                                                         qsTr("Select Ok to reboot vehicle."),
                                                         Dialog.Cancel | Dialog.Ok,
                                                         function() { _activeVehicle.rebootVehicle() })
        }
    }


    QGCFileDialog {
        id:             fileDialog
        folder:         _appSettings.parameterSavePath
        nameFilters:    [ qsTr("Parameter Files (*.%1)").arg(_appSettings.parameterFileExtension) , qsTr("All Files (*)") ]

        onAcceptedForSave: (file) => {
            controller.saveToFile(file)
            close()
        }

        onAcceptedForLoad: (file) => {
            close()
            if (controller.buildDiffFromFile(file)) {
                parameterDiffDialog.createObject(mainWindow).open()
            }
        }
    }

    Component {
        id: editorDialogComponent

        ParameterEditorDialog {
            fact:           _editorDialogFact
            showRCToParam:  _showRCToParam
        }
    }

    Component {
        id: parameterDiffDialog

        ParameterDiffDialog {
            paramController: _controller
        }
    }

    RowLayout {
        id:             header
        anchors.left:   parent.left
        anchors.right:  parent.right
        spacing:        ScreenTools.defaultFontPixelWidth

        QGCTextField {
            id:                     searchText
            objectName:             "parameterSearchField"
            Layout.fillWidth:       true
            Layout.maximumWidth:    ScreenTools.defaultFontPixelWidth * 60
            placeholderText:        qsTr("Search parameters")
            onDisplayTextChanged:   controller.searchText = displayText
        }

        QGCCheckBox {
            text:       qsTr("Modified")
            checked:    controller.showModifiedOnly
            onClicked:  controller.showModifiedOnly = checked
            visible:    QGroundControl.multiVehicleManager.activeVehicle.px4Firmware
        }

        Item { Layout.fillWidth: true }

        QGCButton {
            text:       qsTr("Tools")
            onClicked:  toolsMenu.popup()
        }
    }

    QGCFlickable {
        id:                 groupScroll
        anchors.topMargin:  ScreenTools.defaultFontPixelHeight * 0.8
        anchors.top:        header.bottom
        anchors.bottom:     parent.bottom
        width:              ScreenTools.defaultFontPixelWidth * 28
        clip:               true
        contentHeight:      groupColumn.height
        flickableDirection: Flickable.VerticalFlick
        visible:            !_searchFilter

        Column {
            id:         groupColumn
            width:      parent.width
            spacing:    ScreenTools.defaultFontPixelHeight * 0.8

            Repeater {
                model: controller.categories

                Column {
                    width:      parent.width
                    spacing:    ScreenTools.defaultFontPixelHeight * 0.35

                    property var category: object

                    QGCLabel {
                        text:               object.name.toUpperCase()
                        font.pointSize:     ScreenTools.smallFontPointSize
                        font.letterSpacing: 0.5
                        color:              qgcPal.colorGrey
                        leftPadding:        ScreenTools.defaultFontPixelWidth * 1.5
                    }

                    PlanGroupCard {
                        width: parent.width

                        Repeater {
                            model: category.groups

                            PlanGroupRow {
                                text:           object.name
                                interactive:    true
                                current:        object == controller.currentGroup
                                onClicked: {
                                    if (controller.currentCategory != category) {
                                        controller.currentCategory = category
                                    }
                                    controller.currentGroup = object
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Rectangle {
        anchors.topMargin:  ScreenTools.defaultFontPixelHeight * 0.8
        anchors.leftMargin: _searchFilter ? 0 : ScreenTools.defaultFontPixelWidth * 2
        anchors.top:        header.bottom
        anchors.bottom:     parent.bottom
        anchors.left:       _searchFilter ? parent.left : groupScroll.right
        anchors.right:      parent.right
        radius:             ScreenTools.defaultFontPixelHeight * 0.9
        color:              Qt.alpha(qgcPal.text, 0.055)
        clip:               true

        QGCListView {
            id:             parameterList
            anchors.fill:   parent
            model:          controller.parameters

            onModelChanged: positionViewAtBeginning()

            delegate: PlanGroupRow {
                text:           display
                description:    fact.shortDescription
                interactive:    true
                value:          fact.enumStrings.length === 0    ? fact.valueString + " " + fact.units
                              : fact.bitmaskStrings.length !== 0 ? fact.selectedBitmaskStrings.join(", ")
                                                                 : fact.enumStringValue
                onClicked: {
                    _editorDialogFact = fact
                    editorDialogComponent.createObject(mainWindow).open()
                }

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width:                  ScreenTools.defaultFontPixelHeight * 0.45
                    height:                 width
                    radius:                 width / 2
                    color:                  qgcPal.colorOrange
                    visible:                fact.defaultValueAvailable && !fact.valueEqualsDefault
                }
            }
        }
    }
}
