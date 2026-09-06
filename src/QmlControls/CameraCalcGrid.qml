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
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.FactControls

Column {
    spacing: ScreenTools.defaultFontPixelHeight * 0.7

    property var    cameraCalc
    property string distanceToSurfaceLabel
    property string frontalDistanceLabel
    property string sideDistanceLabel

    PlanGroupCard {
        width:   parent.width
        visible: !cameraCalc.isManualCamera

        PlanFactRow {
            text: qsTr("Front overlap")
            fact: cameraCalc.frontalOverlap
        }

        PlanFactRow {
            text: qsTr("Side overlap")
            fact: cameraCalc.sideOverlap
        }

        PlanGroupRow {
            text: qsTr("Set by")

            OverlaySegmentedControl {
                anchors.verticalCenter: parent.verticalCenter
                width:                  ScreenTools.defaultFontPixelWidth * 20
                height:                 ScreenTools.defaultFontPixelHeight * 1.8
                segments:               [ distanceToSurfaceLabel, qsTr("Ground res") ]
                currentIndex:           cameraCalc.valueSetIsDistance.value ? 0 : 1
                onActivated:            (index) => cameraCalc.valueSetIsDistance.value = index === 0 ? 1 : 0
            }
        }

        PlanFactRow {
            text:         distanceToSurfaceLabel
            fact:         cameraCalc.distanceToSurface
            altitudeMode: cameraCalc.distanceMode
            visible:      !!cameraCalc.valueSetIsDistance.value
        }

        PlanFactRow {
            text:    qsTr("Ground resolution")
            fact:    cameraCalc.imageDensity
            visible: !cameraCalc.valueSetIsDistance.value
        }

        PlanGroupRow {
            text:  frontalDistanceLabel
            value: cameraCalc.adjustedFootprintFrontal.valueString + " " + cameraCalc.adjustedFootprintFrontal.units
        }

        PlanGroupRow {
            text:  sideDistanceLabel
            value: cameraCalc.adjustedFootprintSide.valueString + " " + cameraCalc.adjustedFootprintSide.units
        }
    }

    PlanGroupCard {
        width:   parent.width
        visible: cameraCalc.isManualCamera

        PlanFactRow {
            text:         distanceToSurfaceLabel
            fact:         cameraCalc.distanceToSurface
            altitudeMode: cameraCalc.distanceMode
        }

        PlanFactRow {
            text: frontalDistanceLabel
            fact: cameraCalc.adjustedFootprintFrontal
        }

        PlanFactRow {
            text: sideDistanceLabel
            fact: cameraCalc.adjustedFootprintSide
        }
    }
}
