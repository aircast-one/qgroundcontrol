/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

QtObject {
    property var  sensorNames:           []
    property var  sensorHealthy:         []
    property var  sensorEnabled:         []
    property bool healthChecksSupported: false
    property bool canArm:                true
    property bool hasWarningsOrErrors:   false

    readonly property var    faults:       sensorNames.filter((name, i) => sensorEnabled[i] && !sensorHealthy[i])
    readonly property var    disabled:     sensorNames.filter((name, i) => !sensorEnabled[i])
    readonly property int    normalCount:  sensorNames.length - faults.length - disabled.length
    readonly property string faultList:    faults.join(", ")
    readonly property string disabledList: disabled.join(", ")

    readonly property bool fault:   healthChecksSupported ? !canArm             : faults.length > 0
    readonly property bool caution: healthChecksSupported ? hasWarningsOrErrors : disabled.length > 0
    readonly property bool nominal: !fault && !caution
}
