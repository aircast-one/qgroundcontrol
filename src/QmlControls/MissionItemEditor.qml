import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQml
import QtQuick.Layouts

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Vehicle
import QGroundControl.Controls
import QGroundControl.FactControls
import QGroundControl.Palette


/// One row of the mission item list.
///
/// The row used to be a free-standing card: its own fill, its own radius, its own gap to the next
/// one. A list of framed boxes reads as a list of unrelated things, and the number that ties a row
/// to its marker on the map was buried in the middle of it. Now rows sit flush inside one grouped
/// surface, separated by hairlines, each led by the same numbered seal the map draws.
Rectangle {
    id:             _root
    height:         headerRow.height + (_expandInline ? editorLoader.height + _innerMargin * 2 : 0)
    color:          "transparent"

    property var    map                 ///< Map control
    property var    masterController
    property var    missionItem         ///< MissionItem associated with this editor
    property bool   readOnly            ///< true: read only view, false: full editing view

    signal clicked
    signal remove
    signal selectNextNotReadyItem

    property var    _masterController:          masterController
    property var    _missionController:         _masterController.missionController
    property bool   _currentItem:               missionItem.isCurrentItem
    property bool   _readyForSave:              missionItem.readyForSaveState === VisualMissionItem.ReadyForSave
    property bool   _waypointsOnlyMode:         QGroundControl.corePlugin.options.missionWaypointsOnly
    // Read by MissionSettingsEditor from this scope when it loads below the Mission Start row.
    property bool   _noMissionItemsAdded:       ListView.view ? ListView.view.model.count === 1 : false

    // A survey or scan opens as its own page, so its editor never expands under the row.
    readonly property bool _isComplexItem:  !missionItem.isSimpleItem && missionItem.sequenceNumber !== 0
    readonly property bool _expandInline:   _currentItem && !_isComplexItem
    readonly property bool _isLaunchItem:   missionItem.sequenceNumber === 0 || missionItem.isTakeoffItem

    readonly property real  _editFieldWidth:    Math.min(width - _innerMargin * 2, ScreenTools.defaultFontPixelWidth * 12)
    readonly property real  _margin:            ScreenTools.defaultFontPixelWidth / 2
    readonly property real  _innerMargin:       ScreenTools.defaultFontPixelHeight * 0.4
    readonly property real  _hPad:              ScreenTools.defaultFontPixelWidth * 1.5
    readonly property real  _sealSize:          ScreenTools.defaultFontPixelHeight * 1.5
    readonly property real  _glyphSize:         ScreenTools.defaultFontPixelHeight

    QGCPalette {
        id: qgcPal
        colorGroupEnabled: enabled
    }

    // Selection tints the row, not the form that opens under it. Filling the whole expanded
    // height turned a selected item into a slab the size of a page.
    Rectangle {
        anchors.top:    parent.top
        anchors.left:   parent.left
        anchors.right:  parent.right
        height:         headerRow.height
        color:          _currentItem                ? Qt.alpha(qgcPal.primaryButton, 0.14)
                      : rowMouseArea.containsMouse  ? Qt.alpha(qgcPal.text, 0.05)
                                                    : "transparent"

        Behavior on color { ColorAnimation { duration: 120 } }
    }

    Rectangle {
        anchors.top:        parent.top
        anchors.left:       parent.left
        anchors.right:      parent.right
        anchors.leftMargin: _hPad
        height:             1
        color:              Qt.alpha(qgcPal.text, 0.09)
        visible:            _root.y > 0 && !_currentItem
    }

    QGCMouseArea {
        id:             rowMouseArea
        anchors.top:    parent.top
        anchors.left:   parent.left
        anchors.right:  parent.right
        height:         headerRow.height
        hoverEnabled:   !ScreenTools.isMobile
        onClicked: {
            if (mainWindow.allowViewSwitch()) {
                _root.clicked()
            }
        }
    }

    Component {
        id: editPositionDialog

        EditPositionDialog {
            coordinate:             missionItem.isSurveyItem ?  missionItem.centerCoordinate : missionItem.coordinate
            onCoordinateChanged:    missionItem.isSurveyItem ?  missionItem.centerCoordinate = coordinate : missionItem.coordinate = coordinate
        }
    }

    Item {
        id:             headerRow
        anchors.top:    parent.top
        anchors.left:   parent.left
        anchors.right:  parent.right
        height:         Math.max(ScreenTools.defaultFontPixelHeight * 2.6, _sealSize + _innerMargin * 2)

        // The number the map draws on this item's marker, and the only thing that lets a row be
        // matched to a point on the plan. Doubles as the not-ready indicator so one thing sits
        // in this position rather than two competing for it.
        Rectangle {
            id:                     sequenceSeal
            anchors.left:           parent.left
            anchors.leftMargin:     _hPad
            anchors.verticalCenter: parent.verticalCenter
            width:                  _sealSize
            height:                 _sealSize
            radius:                 _isComplexItem ? width * 0.3 : width / 2
            color:                  !_readyForSave  ? "transparent"
                                  : _isLaunchItem   ? qgcPal.colorGreen
                                                    : qgcPal.primaryButton
            border.width:           _readyForSave ? 0 : 1
            border.color:           qgcPal.warningText

            QGCLabel {
                anchors.centerIn:   parent
                //: Indicator in Plan view to show mission item is not ready for save/send
                text:               _readyForSave ? missionItem.sequenceNumber : qsTr("?")
                color:              _readyForSave ? "white" : qgcPal.warningText
                font.pointSize:     ScreenTools.smallFontPointSize
                font.bold:          true
            }
        }

        // The command name is a menu only while this row is the current simple item; every other
        // row shows the name flat, so a list at rest has one affordance on it, not one per row.
        Item {
            id:                     commandPicker
            anchors.left:           sequenceSeal.right
            anchors.leftMargin:     ScreenTools.defaultFontPixelWidth
            anchors.right:          trailingRow.left
            anchors.rightMargin:    ScreenTools.defaultFontPixelWidth * 0.5
            anchors.verticalCenter: parent.verticalCenter
            height:                 commandLabel.height

            readonly property bool _editable: missionItem.isCurrentItem && missionItem.isSimpleItem &&
                                                  !_waypointsOnlyMode && !missionItem.isTakeoffItem

            QGCLabel {
                id:                     commandLabel
                anchors.left:           parent.left
                anchors.verticalCenter: parent.verticalCenter
                text:                   missionItem.commandName
                color:                  qgcPal.text
                font.bold:              _currentItem
                elide:                  Text.ElideRight
                width:                  Math.min(implicitWidth, parent.width - (commandPicker._editable ? ScreenTools.defaultFontPixelWidth * 1.5 : 0))
            }

            QGCColoredImage {
                anchors.left:           commandLabel.right
                anchors.leftMargin:     ScreenTools.defaultFontPixelWidth * 0.4
                anchors.verticalCenter: commandLabel.verticalCenter
                height:                 ScreenTools.defaultFontPixelWidth
                width:                  height
                fillMode:               Image.PreserveAspectFit
                smooth:                 true
                antialiasing:           true
                color:                  Qt.alpha(qgcPal.text, 0.5)
                source:                 "/qmlimages/arrow-down.png"
                visible:                commandPicker._editable
            }

            QGCMouseArea {
                fillItem:   parent
                enabled:    commandPicker._editable
                onClicked:  commandDialog.createObject(mainWindow).open()
            }

            Component {
                id: commandDialog

                MissionCommandDialog {
                    vehicle:                    masterController.controllerVehicle
                    missionItem:                _root.missionItem
                    map:                        _root.map
                    // FIXME: Disabling fly through commands doesn't work since you may need to change from an RTL to something else
                    flyThroughCommandsAllowed:  true //_missionController.flyThroughCommandsAllowed
                }
            }
        }

        Row {
            id:                     trailingRow
            anchors.right:          parent.right
            anchors.rightMargin:    _hPad
            anchors.verticalCenter: parent.verticalCenter
            spacing:                ScreenTools.defaultFontPixelWidth

            // Altitude and leg length are the two numbers a planner scans a list for. In tabular
            // figures they line up down the column, so two rows can be compared without reading.
            QGCLabel {
                anchors.verticalCenter: parent.verticalCenter
                visible:                text !== ""
                color:                  Qt.alpha(qgcPal.text, _currentItem ? 0.85 : 0.6)
                font.family:            ScreenTools.fixedFontFamily
                font.pointSize:         ScreenTools.smallFontPointSize
                text: {
                    const parts = []
                    if (missionItem.specifiesCoordinate && !isNaN(missionItem.amslEntryAlt)) {
                        parts.push(QGroundControl.unitsConversion.metersToAppSettingsVerticalDistanceUnits(missionItem.amslEntryAlt).toFixed(0) +
                                   " " + QGroundControl.unitsConversion.appSettingsVerticalDistanceUnitsString)
                    }
                    if (!isNaN(missionItem.distance) && missionItem.distance > 0) {
                        parts.push(QGroundControl.unitsConversion.metersToAppSettingsHorizontalDistanceUnits(missionItem.distance).toFixed(0) +
                                   " " + QGroundControl.unitsConversion.appSettingsHorizontalDistanceUnitsString)
                    }
                    return parts.join(" · ")
                }
            }

            QGCColoredImage {
                anchors.verticalCenter: parent.verticalCenter
                height:                 _glyphSize
                width:                  height
                sourceSize.height:      height
                fillMode:               Image.PreserveAspectFit
                mipmap:                 true
                smooth:                 true
                color:                  qgcPal.text
                visible:                _currentItem && missionItem.sequenceNumber !== 0 && !_isComplexItem
                source:                 "/res/TrashDelete.svg"

                QGCMouseArea {
                    fillItem:   parent
                    onClicked:  remove()
                }
            }

            QGCColoredImage {
                id:                     hamburger
                anchors.verticalCenter: parent.verticalCenter
                width:                  _glyphSize
                height:                 _glyphSize
                sourceSize.height:      _glyphSize
                source:                 "qrc:/qmlimages/Hamburger.svg"
                visible:                _currentItem && missionItem.sequenceNumber !== 0 && !_isComplexItem
                color:                  qgcPal.text

                QGCMouseArea {
                    fillItem:   hamburger
                    onClicked: (position) => {
                        position = Qt.point(position.x, position.y)
                        // For some strange reason using mainWindow in mapToItem doesn't work, so we use globals.parent instead which also gets us mainWindow
                        position = mapToItem(globals.parent, position)
                        var dropPanel = hamburgerMenuDropPanelComponent.createObject(mainWindow, { clickRect: Qt.rect(position.x, position.y, 0, 0) })
                        dropPanel.open()
                    }
                }
            }

            QGCLabel {
                anchors.verticalCenter: parent.verticalCenter
                text:                   "›"
                color:                  Qt.alpha(qgcPal.text, 0.4)
                font.pointSize:         ScreenTools.mediumFontPointSize
                visible:                _isComplexItem
            }
        }
    }

    Component {
        id: hamburgerMenuDropPanelComponent

        DropPanel {
            id: hamburgerMenuDropPanel

            sourceComponent: Component {
                ColumnLayout {
                    spacing: ScreenTools.defaultFontPixelHeight / 2

                    QGCButton {
                        Layout.fillWidth:   true
                        text:               qsTr("Move to vehicle position")
                        enabled:            _activeVehicle && missionItem.specifiesCoordinate

                        onClicked: {
                            missionItem.coordinate = _activeVehicle.coordinate
                            hamburgerMenuDropPanel.close()
                        }

                        property var _activeVehicle: QGroundControl.multiVehicleManager.activeVehicle
                    }

                    QGCButton {
                        Layout.fillWidth:   true
                        text:               qsTr("Move to previous item position")
                        enabled:            _missionController.previousCoordinate.isValid
                        onClicked: {
                            missionItem.coordinate = _missionController.previousCoordinate
                            hamburgerMenuDropPanel.close()
                        }
                    }

                    QGCButton {
                        Layout.fillWidth:   true
                        text:               qsTr("Edit position...")
                        enabled:            missionItem.specifiesCoordinate
                        onClicked: {
                            editPositionDialog.createObject(mainWindow).open()
                            hamburgerMenuDropPanel.close()
                        }
                    }

                    Rectangle {
                        Layout.fillWidth:       true
                        Layout.preferredHeight: 1
                        color:                  qgcPal.groupBorder
                    }

                    QGCCheckBoxSlider {
                        Layout.fillWidth:   true
                        text:               qsTr("Show all values")
                        visible:            QGroundControl.corePlugin.showAdvancedUI
                        checked:            missionItem.isSimpleItem ? missionItem.rawEdit : false
                        enabled:            missionItem.isSimpleItem && !_waypointsOnlyMode

                        onClicked: {
                            missionItem.rawEdit = checked
                            if (missionItem.rawEdit && !missionItem.friendlyEditAllowed) {
                                missionItem.rawEdit = false
                                checked = false
                                mainWindow.showMessageDialog(qsTr("Mission Edit"), qsTr("You have made changes to the mission item which cannot be shown in Simple Mode"))
                            }
                            hamburgerMenuDropPanel.close()
                        }
                    }
                }
            }
        }
    }

    Loader {
        id:                 editorLoader
        anchors.top:        headerRow.bottom
        anchors.topMargin:  _innerMargin
        anchors.left:       parent.left
        anchors.leftMargin: _hPad
        width:              _root.width - _hPad * 2
        source:             _expandInline ? missionItem.editorQml : ""
        asynchronous:       true

        property var    masterController:   _masterController
        property real   availableWidth:     editorLoader.width ///< How wide the editor should be
        property var    editorRoot:         _root
    }
}
