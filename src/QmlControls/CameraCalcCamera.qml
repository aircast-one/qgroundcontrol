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

    property var cameraCalc

    Component.onCompleted: {
        cameraBrandCombo.selectCurrentBrand()
        cameraModelCombo.selectCurrentModel()
    }

    PlanGroupCard {
        width: parent.width

        PlanGroupRow {
            QGCComboBox {
                id:                     cameraBrandCombo
                anchors.verticalCenter: parent.verticalCenter
                width:                  parent.parent.width - ScreenTools.defaultFontPixelWidth * 3
                model:                  cameraCalc.cameraBrandList
                onModelChanged:         selectCurrentBrand()
                onActivated:            (index) => { cameraCalc.cameraBrand = currentText }

                Connections {
                    target:                 cameraCalc
                    onCameraBrandChanged:   cameraBrandCombo.selectCurrentBrand()
                }

                function selectCurrentBrand() {
                    currentIndex = cameraBrandCombo.find(cameraCalc.cameraBrand)
                }
            }
        }

        PlanGroupRow {
            visible: !cameraCalc.isManualCamera && !cameraCalc.isCustomCamera

            QGCComboBox {
                id:                     cameraModelCombo
                anchors.verticalCenter: parent.verticalCenter
                width:                  parent.parent.width - ScreenTools.defaultFontPixelWidth * 3
                model:                  cameraCalc.cameraModelList
                onModelChanged:         selectCurrentModel()
                onActivated:            (index) => { cameraCalc.cameraModel = currentText }

                Connections {
                    target:                 cameraCalc
                    onCameraModelChanged:   cameraModelCombo.selectCurrentModel()
                }

                function selectCurrentModel() {
                    currentIndex = cameraModelCombo.find(cameraCalc.cameraModel)
                }
            }
        }

        PlanGroupRow {
            text:    qsTr("Orientation")
            visible: !cameraCalc.isManualCamera && !cameraCalc.fixedOrientation.value

            OverlaySegmentedControl {
                anchors.verticalCenter: parent.verticalCenter
                width:                  ScreenTools.defaultFontPixelWidth * 20
                height:                 ScreenTools.defaultFontPixelHeight * 1.8
                segments:               [ qsTr("Landscape"), qsTr("Portrait") ]
                currentIndex:           cameraCalc.landscape.value ? 0 : 1
                onActivated:            (index) => cameraCalc.landscape.value = index === 0 ? 1 : 0
            }
        }
    }

    PlanSectionLabel {
        text:    qsTr("SENSOR")
        visible: !cameraCalc.isManualCamera
    }

    PlanGroupCard {
        width:   parent.width
        visible: !cameraCalc.isManualCamera
        enabled: cameraCalc.isCustomCamera

        PlanFactRow { text: qsTr("Sensor width");  fact: cameraCalc.sensorWidth }
        PlanFactRow { text: qsTr("Sensor height"); fact: cameraCalc.sensorHeight }
        PlanFactRow { text: qsTr("Image width");   fact: cameraCalc.imageWidth }
        PlanFactRow { text: qsTr("Image height");  fact: cameraCalc.imageHeight }
        PlanFactRow { text: qsTr("Focal length");  fact: cameraCalc.focalLength }
    }
}
