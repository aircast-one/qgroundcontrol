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
import QtQuick.Dialogs

import QGroundControl
import QGroundControl.Controls
import QGroundControl.Palette
import QGroundControl.ScreenTools

Popup {
    id:                 root
    width:              mainWindow.width
    height:             mainWindow.height
    parent:             Overlay.overlay
    modal:              true
    focus:              true
    margins:            0

    property string title
    property var    buttons:                Dialog.Ok
    property alias  acceptButtonEnabled:    acceptButton.enabled
    property alias  rejectButtonEnabled:    rejectButton.enabled
    property var    dialogProperties
    property bool   destroyOnClose:         true
    property bool   preventClose:           false

    readonly property real headerMinWidth: titleLabel.implicitWidth + rejectButton.width + acceptButton.width + buttonRow.spacing * 2

    signal accepted
    signal rejected

    property var    _qgcPal:            QGroundControl.globalPalette
    property real   _contentMargin:     ScreenTools.defaultFontPixelHeight
    property bool   _acceptAllowed:     acceptButton.visible
    property bool   _rejectAllowed:     rejectButton.visible
    property int    _previousValidationErrorCount: 0

    background: QGCMouseArea {
        width:  mainWindow.width
        height: mainWindow.height

        Rectangle {
            anchors.fill:   parent
            color:          "black"
            opacity:        0.25
        }

        onClicked: {
            if (closePolicy & Popup.CloseOnPressOutside) {
                _reject()
            }
        }
    }

    Component.onCompleted: {
        contentChildren[contentChildren.length - 1].parent = dialogContentParent
    }

    onAboutToShow: {
        _previousValidationErrorCount = globals.validationErrorCount
        setupDialogButtons(buttons)
    }

    onClosed: {
        globals.validationErrorCount = _previousValidationErrorCount
        Qt.inputMethod.hide()
        if (destroyOnClose) {
            root.destroy()
        }
    }

    function _accept() {
        if (_acceptAllowed && acceptButton.enabled && mainWindow.allowViewSwitch(_previousValidationErrorCount)) {
            accepted()
            if (preventClose) {
                preventClose = false
            } else {
                close()
            }
        }
    }

    function _reject() {
        if (_rejectAllowed && ((buttons & Dialog.Cancel) || mainWindow.allowViewSwitch(_previousValidationErrorCount))) {
            rejected()
            if (preventClose) {
                preventClose = false
            } else {
                close()
            }
        }
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: parent.enabled }

    readonly property var _acceptLabels: [
        [ Dialog.Ok,                qsTr("OK") ],
        [ Dialog.Open,              qsTr("Open") ],
        [ Dialog.Save,              qsTr("Save") ],
        [ Dialog.Apply,             qsTr("Apply") ],
        [ Dialog.SaveAll,           qsTr("Save All") ],
        [ Dialog.Yes,               qsTr("Yes") ],
        [ Dialog.YesToAll,          qsTr("Yes to All") ],
        [ Dialog.Retry,             qsTr("Retry") ],
        [ Dialog.Reset,             qsTr("Reset") ],
        [ Dialog.RestoreToDefaults, qsTr("Restore to Defaults") ],
        [ Dialog.Ignore,            qsTr("Ignore") ],
    ]

    readonly property var _rejectLabels: [
        [ Dialog.Cancel,    qsTr("Cancel") ],
        [ Dialog.Close,     qsTr("Close") ],
        [ Dialog.No,        qsTr("No") ],
        [ Dialog.NoToAll,   qsTr("No to All") ],
        [ Dialog.Abort,     qsTr("Abort") ],
    ]

    function setupDialogButtons(buttons) {
        const accept = _acceptLabels.find(e => buttons & e[0])
        const reject = _rejectLabels.find(e => buttons & e[0])
        acceptButton.visible = accept !== undefined
        acceptButton.text    = accept ? accept[1] : ""
        rejectButton.visible = reject !== undefined
        rejectButton.text    = reject ? reject[1] : ""
        closePolicy = (buttons & Dialog.Cancel) ? (Popup.NoAutoClose | Popup.CloseOnEscape) : Popup.NoAutoClose
    }

    function disableAcceptButton() {
        acceptButton.enabled = false
    }

    Rectangle {
        id:             dialogSurface
        anchors.fill:   mainLayout
        anchors.margins: -_contentMargin
        color:          "transparent"
        radius:         Math.round(ScreenTools.defaultFontPixelHeight * 1.1)
        layer.enabled:  true
        layer.effect:   OverlayShadowEffect { elevated: true }

        OverlayGlass {
            anchors.fill:   parent
            radius:         dialogSurface.radius
            frosted:        mainWindow.glassBackdropVisible
            material:       OverlayGlass.Panel
        }
    }

    ColumnLayout {
        id:                 mainLayout
        anchors.centerIn:   parent
        spacing:            _contentMargin

        QGCLabel {
            id:                 titleLabel
            Layout.fillWidth:   true
            text:               root.title
            font.pointSize:     ScreenTools.mediumFontPointSize
            font.bold:          true
        }

        Item {
            Layout.fillWidth:       true
            Layout.preferredWidth:  Math.min(maxAvailableWidth, totalContentWidth)
            Layout.preferredHeight: Math.min(maxAvailableHeight, totalContentHeight)

            property real maxAvailableWidth:    mainWindow.width - _contentMargin * 6
            property real maxAvailableHeight:   mainWindow.height - titleLabel.height - buttonRow.height - _contentMargin * 6
            property real totalContentWidth:    dialogContentParent.childrenRect.width
            property real totalContentHeight:   dialogContentParent.childrenRect.height

            QGCFlickable {
                anchors.fill:   parent
                contentWidth:   dialogContentParent.childrenRect.width
                contentHeight:  dialogContentParent.childrenRect.height

                Item {
                    id:     dialogContentParent
                    focus:  true

                    Keys.onPressed: (event) => {
                        if (event.key === Qt.Key_Escape && _rejectAllowed) {
                            _reject()
                            event.accepted = true
                        } else if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && _acceptAllowed) {
                            _accept()
                            event.accepted = true
                        }
                    }
                }
            }
        }

        RowLayout {
            id:                 buttonRow
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelWidth

            Item { Layout.fillWidth: true }

            QGCButton {
                id:                     rejectButton
                Layout.minimumWidth:    ScreenTools.defaultFontPixelWidth * 10
                onClicked:              _reject()
            }

            QGCButton {
                id:                     acceptButton
                Layout.minimumWidth:    ScreenTools.defaultFontPixelWidth * 10
                primary:                true
                onClicked:              _accept()
            }
        }
    }
}
