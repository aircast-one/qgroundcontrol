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

import QGroundControl.Controls
import QGroundControl.ScreenTools

QGCPopupDialog {
    property alias  text:           label.text
    property var    acceptFunction: null        // Mainly used by MainRootWindow.showMessage to specify accept function in call
    property var    closeFunction:  null

    onAccepted: {
        if (acceptFunction) {
            acceptFunction()
        }
    }

    onClosed: {
        if (closeFunction) {
            closeFunction()
        }
    }

    ColumnLayout {
        QGCLabel {
            id:                     label
            Layout.preferredWidth:  Math.max(Math.min(mainWindow.width / (ScreenTools.isMobile ? 2 : 4), ScreenTools.defaultFontPixelWidth * 56), headerMinWidth)
            wrapMode:               Text.WordWrap
        }
    }
}
