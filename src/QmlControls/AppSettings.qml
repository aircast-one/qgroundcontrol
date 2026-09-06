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

ToolDrawerPage {
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

    readonly property bool _fullScreen: ScreenTools.isMobile
    readonly property bool _stacked:    _fullScreen && mainWindow.height > mainWindow.width

    preferredWidth:  _fullScreen ? 0
                                 : sidebarSlab.width + _inset * 2 + _defaultTextWidth * 4 + _pageWidth
    preferredHeight: _fullScreen ? 0
                                 : Math.max(buttonColumn.height, _pageHeight) + _inset * 4

    readonly property var  _page:       rightPanel.item
    readonly property real _pageWidth:  _page && _page.contentWidth  > 0 ? _page.contentWidth
                                                                         : _defaultTextWidth * 60
    readonly property real _pageHeight: _page && _page.contentHeight > 0 ? _page.contentHeight : 0

    readonly property string _filter:   searchField.text.trim().toLowerCase()

    readonly property int _matchCount: settingsPagesModel.matchCount(_filter)

    property string _pageTitle
    property bool   _pageOpen: !_stacked

    pageTitle: !_stacked ? "" : _pageOpen ? _pageTitle : qsTr("Settings")

    function popPage(): bool {
        if (_stacked && _pageOpen) {
            _pageOpen = false
            return true
        }
        return false
    }

    function openPage(url, name) {
        if (mainWindow.allowViewSwitch()) {
            if (rightPanel.source !== url) {
                rightPanel.source = url
            }
            _pageTitle = name
            _pageOpen = true
        }
    }

    function showSettingsPage(settingsPageUrl) {
        const page = settingsPagesModel.pages().find(candidate => candidate.url === settingsPageUrl)
        if (!page) {
            console.warn("showSettingsPage: no settings page with url", settingsPageUrl)
            return
        }
        openPage(page.url, page.name)
    }

    DeadMouseArea {
        anchors.fill: parent
    }

    QGCPalette { id: qgcPal }

    Component.onCompleted: {
        if (globals.commingFromRIDIndicator) {
            globals.commingFromRIDIndicator = false
            openPage("qrc:/qml/QGroundControl/AppSettings/RemoteIDSettings.qml", qsTr("Remote ID"))
        } else {
            rightPanel.source = "qrc:/qml/QGroundControl/AppSettings/GeneralSettings.qml"
            _pageTitle = qsTr("General")
        }
    }

    SettingsPagesModel { id: settingsPagesModel }

    OverlayGlass {
        id:                     sidebarSlab
        visible:                !_stacked
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

    QGCTextField {
        id:                 searchField
        objectName:         "settingsSearchField"
        visible:            _fullScreen && (!_stacked || !_pageOpen)
        x:                  _inset + _horizontalMargin
        y:                  _inset + _verticalMargin
        width:              _stacked ? settingsView.width - x * 2 : buttonColumn.width
        height:             ScreenTools.minTouchPixels
        leftPadding:        height * 0.8
        rightPadding:       height * 0.8
        onVisibleChanged:   if (!visible) text = ""

        background: Rectangle {
            radius:         height / 2
            color:          Qt.alpha(qgcPal.text, searchField.activeFocus ? 0.14 : 0.08)
            border.width:   searchField.activeFocus ? 1 : 0
            border.color:   qgcPal.buttonHighlight
        }

        QGCLabel {
            anchors.left:           parent.left
            anchors.leftMargin:     searchField.leftPadding
            anchors.verticalCenter: parent.verticalCenter
            visible:                searchField.text === ""
            text:                   qsTr("Search settings")
            color:                  Qt.alpha(qgcPal.text, 0.5)
        }

        QGCColoredImage {
            anchors.left:           parent.left
            anchors.leftMargin:     parent.height * 0.28
            anchors.verticalCenter: parent.verticalCenter
            width:                  parent.height * 0.42
            height:                 width
            source:                 "/InstrumentValueIcons/search.svg"
            color:                  Qt.alpha(qgcPal.text, 0.5)
            fillMode:               Image.PreserveAspectFit
            sourceSize.height:      height
        }

        QGCColoredImage {
            id:                     searchClear
            objectName:             "settingsSearchClear"
            anchors.right:          parent.right
            anchors.rightMargin:    parent.height * 0.28
            anchors.verticalCenter: parent.verticalCenter
            width:                  parent.height * 0.36
            height:                 width
            visible:                searchField.text !== ""
            source:                 "/res/XDelete.svg"
            color:                  Qt.alpha(qgcPal.text, 0.5)
            fillMode:               Image.PreserveAspectFit
            sourceSize.height:      height

            QGCMouseArea {
                anchors.fill:       parent
                anchors.margins:    -ScreenTools.defaultFontPixelWidth
                onClicked:          searchField.text = ""
            }
        }
    }

    QGCFlickable {
        id:                 buttonList
        objectName:         "settingsList"
        visible:            !_stacked || !_pageOpen
        x:                  _inset + _horizontalMargin
        y:                  searchField.visible ? searchField.y + searchField.height + _verticalMargin
                                                 : _inset + _verticalMargin
        width:              _stacked ? settingsView.width - x * 2 : buttonColumn.width
        height:             settingsView.height - y - (_inset + _verticalMargin)
        contentHeight:      buttonColumn.height + _verticalMargin
        flickableDirection: Flickable.VerticalFlick
        clip:               true

        ColumnLayout {
            id:         buttonColumn
            width:      _stacked ? buttonList.width : implicitWidth
            spacing:    _stacked ? 0 : ScreenTools.defaultFontPixelHeight / 4

            Repeater {
                id:     buttonRepeater
                model:  settingsPagesModel

                ColumnLayout {
                    Layout.fillWidth:   true
                    Layout.topMargin:   sectionHeader.visible ? _defaultTextHeight :
                                            (!_stacked && newSection && index > 0 ? _defaultTextHeight * 0.75 : 0)
                    spacing:            0
                    visible:            settingsPagesModel.matches(model, _filter)

                    QGCLabel {
                        id:                 sectionHeader
                        Layout.fillWidth:   true
                        Layout.bottomMargin: _verticalMargin / 2
                        Layout.leftMargin:  ScreenTools.defaultFontPixelWidth * 0.75
                        visible:            _fullScreen && section !== "" && _filter === ""
                        text:               section
                        color:              qgcPal.primaryButton
                        font.pointSize:     ScreenTools.smallFontPointSize
                        font.bold:          true
                    }

                    SettingsButton {
                        objectName:         "settingsPage" + name.replace(/\s/g, "")
                        Layout.fillWidth:   true
                        autoExclusive:      false
                        listStyle:          _stacked
                        text:               name
                        description:        _stacked ? summary : ""
                        icon.source:        iconUrl
                        tileColor:          model.tileColor
                        checked:            !_stacked && _pageTitle === name
                        onClicked:          openPage(url, name)
                    }
                }
            }
        }
    }

    QGCLabel {
        id:                     noResults
        objectName:             "settingsSearchNoResults"
        visible:                searchField.visible && _filter !== "" && _matchCount === 0
        x:                      buttonList.x
        y:                      buttonList.y + _defaultTextHeight
        width:                  buttonList.width
        wrapMode:               Text.WordWrap
        horizontalAlignment:    Text.AlignHCenter
        color:                  Qt.alpha(qgcPal.text, 0.6)
        text:                   qsTr("No settings match “%1”.").arg(searchField.text.trim())
    }

    QGCLabel {
        id:                     pageTitleLabel
        visible:                !_stacked
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
        objectName:             "settingsPageLoader"
        visible:                !_stacked || _pageOpen
        anchors.leftMargin:     _stacked ? _inset + _horizontalMargin : _defaultTextWidth * 3
        anchors.rightMargin:    _stacked ? _inset + _horizontalMargin : _inset
        anchors.topMargin:      _verticalMargin
        anchors.bottomMargin:   _inset
        anchors.left:           _stacked ? parent.left : sidebarSlab.right
        anchors.right:          parent.right
        anchors.top:            _stacked ? parent.top : pageTitleLabel.bottom
        anchors.bottom:         parent.bottom
    }
}
