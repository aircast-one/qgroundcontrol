import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls

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
            objectName:             "popupDialog_text"
            Layout.preferredWidth:  Math.max(Math.min(mainWindow.width / (ScreenTools.isMobile ? 2 : 4), ScreenTools.defaultFontPixelWidth * 56), headerMinWidth)
            wrapMode:               Text.WordWrap
        }
    }
}
