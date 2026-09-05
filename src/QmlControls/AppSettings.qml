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

import QGroundControl
import QGroundControl.Palette
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.AppSettings

Rectangle {
    id:         settingsView
    objectName: "appSettingsView"
    color:      "transparent"
    z:          QGroundControl.zOrderTopMost

    readonly property real _defaultTextHeight:  ScreenTools.defaultFontPixelHeight
    readonly property real _defaultTextWidth:   ScreenTools.defaultFontPixelWidth
    readonly property real _horizontalMargin:   _defaultTextWidth / 2
    readonly property real _verticalMargin:     _defaultTextHeight / 2
    readonly property real _inset:              Math.round(_defaultTextHeight * 0.75)
    readonly property real _sidebarRadius:      mainWindow.panelRadius - _inset
    readonly property real _buttonHeight:       ScreenTools.isTinyScreen ? ScreenTools.defaultFontPixelHeight * 3 : ScreenTools.defaultFontPixelHeight * 2

    readonly property real preferredWidth:  sidebarSlab.width + _inset * 2 +
                                                _defaultTextWidth * 4 + _pageWidth
    readonly property real preferredHeight: Math.max(buttonColumn.height, _pageHeight) +
                                                _inset * 4

    readonly property var  _page:       rightPanel.item
    readonly property real _pageWidth:  _page && _page.contentWidth  > 0 ? _page.contentWidth
                                                                         : _defaultTextWidth * 60
    readonly property real _pageHeight: _page && _page.contentHeight > 0 ? _page.contentHeight : 0

    property bool   _first: true
    property string _pageTitle

    property bool _commingFromRIDSettings:  false

    function showSettingsPage(settingsPage) {
        for (var i=0; i<buttonRepeater.count; i++) {
            var button = buttonRepeater.itemAt(i)
            if (button.text === settingsPage) {
                button.clicked()
                break
            }
        }
    }

    DeadMouseArea {
        anchors.fill: parent
    }

    QGCPalette { id: qgcPal }

    Component.onCompleted: {
        if (globals.commingFromRIDIndicator) {
            rightPanel.source = "qrc:/qml/QGroundControl/AppSettings/RemoteIDSettings.qml"
            _pageTitle = qsTr("Remote ID")
            globals.commingFromRIDIndicator = false
        } else {
            rightPanel.source =  "qrc:/qml/QGroundControl/AppSettings/GeneralSettings.qml"
            _pageTitle = qsTr("General")
        }
    }


    SettingsPagesModel { id: settingsPagesModel }

    OverlayGlass {
        id:                     sidebarSlab
        anchors.left:           parent.left
        anchors.top:            parent.top
        anchors.bottom:         parent.bottom
        anchors.margins:        _inset
        width:                  buttonColumn.width + _horizontalMargin * 2
        radius:                 _sidebarRadius
        frosted:                mainWindow.flyViewBackdropVisible
        tint:                   QGroundControl.globalPalette.windowShade
        material:               OverlayGlass.Panel
    }

    QGCFlickable {
        id:                 buttonList
        width:              buttonColumn.width
        anchors.topMargin:  _verticalMargin
        anchors.bottomMargin: _verticalMargin
        anchors.top:        sidebarSlab.top
        anchors.bottom:     sidebarSlab.bottom
        anchors.leftMargin: _horizontalMargin
        anchors.left:       sidebarSlab.left
        contentHeight:      buttonColumn.height + _verticalMargin
        flickableDirection: Flickable.VerticalFlick
        clip:               true

        ColumnLayout {
            id:         buttonColumn
            spacing:    ScreenTools.defaultFontPixelHeight / 4

            property real _maxButtonWidth: 0

            Repeater {
                id:     buttonRepeater
                model:  settingsPagesModel

                SettingsButton {
                    objectName:         "settingsPage" + name.replace(/\s/g, "")
                    Layout.fillWidth:   true
                    Layout.topMargin:   newSection && index > 0 ? _defaultTextHeight * 0.75 : 0
                    text:               name
                    icon.source:        iconUrl
                    tileColor:          model.tileColor
                    visible:            pageVisible()

                    onClicked: {
                        if (mainWindow.allowViewSwitch()) {
                            if (rightPanel.source !== url) {
                                rightPanel.source = url
                            }
                            _pageTitle = name
                            checked = true
                        }
                    }

                    Component.onCompleted: {
                        if (globals.commingFromRIDIndicator) {
                            _commingFromRIDSettings = true
                        }
                        if(_first) {
                            _first = false
                            checked = true
                        }
                        if (_commingFromRIDSettings) {
                            checked = false
                            _commingFromRIDSettings = false
                            if (modelData.url == "qrc:/qml/QGroundControl/AppSettings/RemoteIDSettings.qml") {
                                checked = true
                            }
                        }
                    }
                }
            }
        }
    }

    QGCLabel {
        id:                     pageTitle
        anchors.leftMargin:     _defaultTextWidth * 3
        anchors.topMargin:      _inset
        anchors.left:           sidebarSlab.right
        anchors.top:            parent.top
        text:                   _pageTitle
        font.pointSize:         ScreenTools.largeFontPointSize
        font.bold:              true
    }

    Loader {
        id:                     rightPanel
        anchors.leftMargin:     _defaultTextWidth * 3
        anchors.rightMargin:    _inset
        anchors.topMargin:      _verticalMargin
        anchors.bottomMargin:   _inset
        anchors.left:           sidebarSlab.right
        anchors.right:          parent.right
        anchors.top:            pageTitle.bottom
        anchors.bottom:         parent.bottom
    }
}

