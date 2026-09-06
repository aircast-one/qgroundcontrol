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
import QGroundControl.ScreenTools


RowLayout {
    id:         control
    spacing:    ScreenTools.defaultFontPixelWidth

    property bool       showExpand:         false
    property string     expandText:         qsTr("Details")
    property bool       waitForParameters:  false
    property Component  contentComponent
    property Component  expandedComponent
    property var        pageProperties

    // These properties are bound by the MainRoowWindow loader
    property bool expanded: false
    property var  drawer

    property var    activeVehicle:      QGroundControl.multiVehicleManager.vehicle
    property bool   parametersReady:    QGroundControl.multiVehicleManager.parameterReadyVehicleAvailable

    property bool _loadPages: !waitForParameters || parametersReady

    QGCLabel {
        text:       qsTr("Waiting for parameters...")
        visible:    waitForParameters && !parametersReady
    }

    Loader {
        id:                 contentItemLoader
        Layout.alignment:   Qt.AlignTop
        Layout.fillWidth:   true
        sourceComponent:    _loadPages ? contentComponent : undefined

        property var pageProperties: control.pageProperties
    }

    Rectangle {
        id:                     divider
        Layout.preferredWidth:  visible ? 1 : -1
        Layout.fillHeight:      true
        color:                  QGroundControl.globalPalette.groupBorder
        visible:                expanded
    }
    
    Loader {
        id:                     expandedItemLoader
        Layout.alignment:       Qt.AlignTop
        Layout.preferredWidth:  visible ? -1 : 0
        visible:                expanded
        sourceComponent:        expanded ? expandedComponent : undefined

        property var pageProperties: control.pageProperties
    }
}
