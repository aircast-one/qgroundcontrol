/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.ScreenTools

PlanGroupRow {
    id: _root

    property Fact fact

    QGCSwitch {
        id:                     toggle
        anchors.verticalCenter: parent.verticalCenter
        onClicked:              _root.fact.value = checked ? 1 : 0

        Binding on checked {
            value: _root.fact ? (_root.fact.typeIsBool ? _root.fact.value : _root.fact.value !== 0) : false
        }
    }
}
