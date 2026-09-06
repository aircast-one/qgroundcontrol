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

    // Whether this is the first row drawn in its card. Keyed off y before, which only worked
    // because the hidden mission-settings row happened to collapse to zero height above it.
    property bool   firstRow: false

    signal clicked
    signal remove

    property var    _masterController:          masterController
    property var    _missionController:         _masterController.missionController
    property bool   _currentItem:               missionItem.isCurrentItem
    property bool   _readyForSave:              missionItem.readyForSaveState === VisualMissionItem.ReadyForSave
    property bool   _waypointsOnlyMode:         QGroundControl.corePlugin.options.missionWaypointsOnly

    // A survey or scan opens as its own page, so its editor never expands under the row.
    readonly property bool _isComplexItem:  !missionItem.isSimpleItem && missionItem.sequenceNumber !== 0
    readonly property bool _expandInline:   _currentItem && !_isComplexItem
    readonly property bool _isLaunchItem:   missionItem.sequenceNumber === 0 || missionItem.isTakeoffItem

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
        visible:            !_root.firstRow && !_currentItem
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
                height:                 ScreenTools.defaultFontPixelHeight / 2
                width:                  height
                sourceSize.height:      height
                fillMode:               Image.PreserveAspectFit
                mipmap:                 true
                color:                  Qt.alpha(qgcPal.text, 0.5)
                source:                 "/res/DropArrow.svg"
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
            // An incomplete item had only a "?" seal to say so, which says that something is
            // wrong but not what. The figures slot is empty on an item with no position, so the
            // reason goes where the numbers would have been.
            QGCLabel {
                anchors.verticalCenter: parent.verticalCenter
                visible:                !_readyForSave
                color:                  qgcPal.warningText
                font.pointSize:         ScreenTools.smallFontPointSize
                text:                   missionItem.readyForSaveMessage
            }

            QGCLabel {
                anchors.verticalCenter: parent.verticalCenter
                visible:                _readyForSave && text !== ""
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

            // One button, not two. A trash can and a hamburger side by side put a destructive
            // action a few pixels from a routine one, in a row that already carries a number, a
            // name, a picker and two figures. Everything the row can do now lives behind this,
            // with the destructive entry set apart at the bottom where it is reached deliberately.
            QGCColoredImage {
                id:                     moreButton
                anchors.verticalCenter: parent.verticalCenter
                width:                  _glyphSize
                height:                 _glyphSize
                sourceSize.height:      _glyphSize
                fillMode:               Image.PreserveAspectFit
                mipmap:                 true
                source:                 "/InstrumentValueIcons/navigation-more.svg"
                visible:                _currentItem && missionItem.sequenceNumber !== 0 && !_isComplexItem
                color:                  Qt.alpha(qgcPal.text, 0.55)

                QGCMouseArea {
                    fillItem:   moreButton
                    onClicked:  itemMenu.openFrom(moreButton)
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

    OverlayPopover {
        id: itemMenu

        readonly property var _activeVehicle: QGroundControl.multiVehicleManager.activeVehicle

        OverlayMenuItem {
            text:      qsTr("Move to Vehicle Position")
            reserveGutter: true
            enabled:   itemMenu._activeVehicle && missionItem.specifiesCoordinate
            opacity:   enabled ? 1 : 0.45
            onClicked: {
                itemMenu.close()
                missionItem.coordinate = itemMenu._activeVehicle.coordinate
            }
        }

        OverlayMenuItem {
            text:      qsTr("Move to Previous Item")
            reserveGutter: true
            enabled:   _missionController.previousCoordinate.isValid
            opacity:   enabled ? 1 : 0.45
            onClicked: {
                itemMenu.close()
                missionItem.coordinate = _missionController.previousCoordinate
            }
        }

        OverlayMenuItem {
            text:      qsTr("Edit Position…")
            reserveGutter: true
            enabled:   missionItem.specifiesCoordinate
            opacity:   enabled ? 1 : 0.45
            onClicked: {
                itemMenu.close()
                editPositionDialog.createObject(mainWindow).open()
            }
        }

        OverlayMenuSeparator { visible: showAllValues.visible }

        OverlayMenuItem {
            id:        showAllValues
            text:      qsTr("Show All Values")
            checkable: true
            checked:   missionItem.isSimpleItem ? missionItem.rawEdit : false
            visible:   QGroundControl.corePlugin.showAdvancedUI
            enabled:   missionItem.isSimpleItem && !_waypointsOnlyMode
            opacity:   enabled ? 1 : 0.45
            onClicked: {
                itemMenu.close()
                missionItem.rawEdit = !missionItem.rawEdit
                if (missionItem.rawEdit && !missionItem.friendlyEditAllowed) {
                    missionItem.rawEdit = false
                    mainWindow.showMessageDialog(qsTr("Mission Edit"), qsTr("You have made changes to the mission item which cannot be shown in Simple Mode"))
                }
            }
        }

        OverlayMenuSeparator { }

        OverlayMenuItem {
            text:      qsTr("Delete Waypoint")
            reserveGutter: true
            textColor: qgcPal.colorRed
            onClicked: {
                itemMenu.close()
                remove()
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
