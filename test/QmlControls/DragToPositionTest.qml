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
import QtQuick.Layouts

import QGroundControl.Controls

Item {
    id: root
    width: 800
    height: 600

    Rectangle {
        id: panel
        objectName: "panel"
        width: panelSize
        height: width / 2
        color: "green"

        property real panelSize: 200
        property int childClicks: 0
        property int buttonClicks: 0

        DragToPosition {
            id: dragPosition
            objectName: "dragPosition"
            target: panel
            settingsKeyPrefix: "TestPanel"
            defaultX: root.width - panel.width - 8
            defaultY: root.height - panel.height - 8
        }

        DragHandler {
            onActiveChanged: if (!active) dragPosition.commit()
        }

        // Mirrors the production panels: the drag surface is covered by interactive children,
        // so drags must steal the grab from a child MouseArea and clicks must still reach it.
        MouseArea {
            anchors.fill: parent
            onClicked: panel.childClicks++
        }

        Button {
            objectName: "panelButton"
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            width: 60
            height: 30
            onClicked: panel.buttonClicks++
        }

    }

    // Mirrors FlyViewInstrumentPanel/SelectableControl: a Control with the selection combo
    // in its background, a content MouseArea, and a DragHandler on the whole panel.
    Control {
        id: selectablePanel
        objectName: "selectablePanel"
        topPadding: showSelectionUI ? selRow.height : 0

        property bool showSelectionUI: false

        background: Item {
            RowLayout {
                id: selRow
                anchors.right: parent.right
                visible: selectablePanel.showSelectionUI

                Button { text: "lock" }
                ComboBox {
                    objectName: "combo"
                    model: ["Attitude", "Compass"]
                }
            }
        }

        contentItem: Item {
            implicitWidth: 300
            implicitHeight: 150

            MouseArea {
                anchors.fill: parent
                acceptedButtons: Qt.LeftButton | Qt.RightButton
            }
        }

        DragToPosition {
            id: selectableDragPosition
            target: selectablePanel
            settingsKeyPrefix: "TestSelectable"
            defaultX: 10
            defaultY: 380
        }

        // Scoped to contentItem: a handler on the Control itself blocks presses from
        // reaching controls in its background, even when the handler is disabled.
        DragHandler {
            parent: selectablePanel.contentItem
            target: selectablePanel
            onActiveChanged: if (!active) selectableDragPosition.commit()
        }
    }
}
