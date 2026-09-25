import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls

GridLayout {
    columns:        2
    rowSpacing:     _rowSpacing
    columnSpacing:  _colSpacing

    function saveSettings() {
        subEditConfig.apiBase = apiField.text
        subEditConfig.deviceId = deviceField.text
        AircastAccount.apiBase = apiField.text
    }

    function validate() {
        apiField.validationError = !/^https?:\/\/[^\/]+/.test(apiField.text.trim())
        deviceField.validationError = deviceField.text.trim() === ""
        return !apiField.validationError && !deviceField.validationError
    }

    function suggestedName() {
        return qsTr("Aircast cloud")
    }

    QGCLabel { text: qsTr("Account Server"); Layout.preferredWidth: _firstColumnWidth }
    QGCTextField {
        id:                     apiField
        Layout.preferredWidth:  _secondColumnWidth
        inputMethodHints:       Qt.ImhNoAutoUppercase | Qt.ImhNoPredictiveText | Qt.ImhUrlCharactersOnly
        text:                   subEditConfig.apiBase
        placeholderText:        "https://api.aircast.one"
    }

    QGCLabel {
        Layout.columnSpan:  2
        Layout.leftMargin:  _firstColumnWidth + _colSpacing
        visible:            apiField.validationError
        color:              QGroundControl.globalPalette.colorRed
        font.pointSize:     ScreenTools.smallFontPointSize
        text:               qsTr("Enter the account server address, starting with https://")
    }

    QGCLabel { text: qsTr("Device"); Layout.preferredWidth: _firstColumnWidth }
    QGCTextField {
        id:                     deviceField
        Layout.preferredWidth:  _secondColumnWidth
        inputMethodHints:       Qt.ImhNoAutoUppercase | Qt.ImhNoPredictiveText
        text:                   subEditConfig.deviceId
    }

    QGCLabel {
        Layout.columnSpan:  2
        Layout.leftMargin:  _firstColumnWidth + _colSpacing
        visible:            deviceField.validationError
        color:              QGroundControl.globalPalette.colorRed
        font.pointSize:     ScreenTools.smallFontPointSize
        text:               qsTr("Set up from the device to fill this in")
    }

    QGCLabel { text: qsTr("Account"); Layout.preferredWidth: _firstColumnWidth }
    RowLayout {
        Layout.preferredWidth: _secondColumnWidth

        QGCLabel {
            Layout.fillWidth:   true
            wrapMode:           Text.WordWrap
            text:               AircastAccount.signedIn ? qsTr("Signed in") : (AircastAccount.status || qsTr("Not signed in"))
        }
        QGCButton {
            visible:    !AircastAccount.signedIn && !AircastAccount.signingIn
            text:       qsTr("Sign In")
            onClicked: {
                AircastAccount.apiBase = apiField.text
                AircastAccount.signIn()
            }
        }
        QGCButton {
            visible:    AircastAccount.signingIn
            text:       qsTr("Cancel")
            onClicked:  AircastAccount.cancelSignIn()
        }
        QGCButton {
            visible:    AircastAccount.signedIn
            text:       qsTr("Sign Out")
            onClicked:  AircastAccount.signOut()
        }
    }

    QGCLabel {
        Layout.columnSpan:  2
        Layout.leftMargin:  _firstColumnWidth + _colSpacing
        Layout.preferredWidth: _secondColumnWidth
        visible:            AircastAccount.signingIn
        wrapMode:           Text.WordWrap
        font.pointSize:     ScreenTools.smallFontPointSize
        text:               qsTr("Your browser opened %1 — approve code %2 there.").arg(AircastAccount.verificationUrl).arg(AircastAccount.userCode)
    }
}
