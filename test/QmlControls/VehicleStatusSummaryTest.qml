import QtQuick

import QGroundControl.Controls

Item {
    id:     root
    width:  100
    height: 100

    property alias sensorNames:           summary.sensorNames
    property alias sensorHealthy:         summary.sensorHealthy
    property alias sensorEnabled:         summary.sensorEnabled
    property alias healthChecksSupported: summary.healthChecksSupported
    property alias canArm:                summary.canArm
    property alias hasWarningsOrErrors:   summary.hasWarningsOrErrors

    readonly property alias faultList:    summary.faultList
    readonly property alias disabledList: summary.disabledList
    readonly property alias normalCount:  summary.normalCount
    readonly property alias fault:        summary.fault
    readonly property alias caution:      summary.caution
    readonly property alias nominal:      summary.nominal

    VehicleStatusSummary { id: summary }
}
