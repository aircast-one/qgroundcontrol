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
import QtQuick.Dialogs
import QtQuick.Layouts

import QGroundControl
import QGroundControl.FactControls
import QGroundControl.Controls

SettingsPage {
    property var    _settingsManager:       QGroundControl.settingsManager
    property var    _videoManager:          QGroundControl.videoManager
    property var    _videoSettings:         _settingsManager.videoSettings
    property bool   _isGST:                 _videoManager.gstreamerEnabled
    property bool   _isStreamSource:        _videoManager.isStreamSource
    property bool   _videoAutoStreamConfig: _videoManager.autoStreamConfigured
    property real   _fieldWidth:            ScreenTools.defaultFontPixelWidth * 40

    function _sourceNeedsUrl(src) {
        return [_videoSettings.udp264VideoSource, _videoSettings.udp265VideoSource, _videoSettings.mpegtsVideoSource,
                _videoSettings.rtspVideoSource, _videoSettings.tcpVideoSource, _videoSettings.webrtcVideoSource].indexOf(src) >= 0
    }

    function _sourceLabel(enumString) {
        return enumString === _videoSettings.disabledVideoSource ? qsTr("Disabled") : enumString.replace(/ Video Stream$/, "")
    }
    function _sourceDisplay(src) {
        if (src === "") return qsTr("Disabled")
        const i = _videoSettings.videoSource.enumValues.indexOf(src)
        return _sourceLabel(i >= 0 ? _videoSettings.videoSource.enumStrings[i] : src)
    }

    SettingsGroupLayout {
        id:                 camList
        Layout.fillWidth:   true
        heading:            qsTr("Cameras")
        visible:            _isGST

        property int selectedIndex: -1

        readonly property var  _qgcPal: QGroundControl.globalPalette
        readonly property real _indent: ScreenTools.defaultFontPixelWidth * 1.5

        ListModel { id: camerasModel }

        property bool readable: true

        function parseCameras() {
            try {
                const parsed = JSON.parse(_videoSettings.cameras.rawValue || "[]")
                return Array.isArray(parsed) ? parsed : null
            } catch (e) {
                return null
            }
        }

        function camRow(camIndex, name, source, url) {
            return { camIndex: camIndex, camName: name, camSource: source, camUrl: url }
        }

        function rowsOf(cameras) {
            return cameras.map((cam, i) => camRow(i, cam.name || "", cam.source || _videoSettings.disabledVideoSource, cam.url || ""))
        }

        function reload() {
            const cameras = parseCameras()
            readable = cameras !== null
            const rows = rowsOf(cameras || [])
            const same = rows.length === camerasModel.count && rows.every((row, i) => {
                const shown = camerasModel.get(i)
                return shown.camName === row.camName && shown.camSource === row.camSource && shown.camUrl === row.camUrl
            })
            if (same) {
                return
            }
            camerasModel.clear()
            rows.forEach(row => camerasModel.append(row))
        }

        function saveCamera(camIndex, name, source, url) {
            if (!readable || camIndex >= camerasModel.count) {
                return
            }
            camerasModel.set(camIndex, camRow(camIndex, name, source, url))
            _videoSettings.updateCamera(camIndex, name, source, url)
        }

        function urlForSource(source, currentUrl) {
            return _sourceNeedsUrl(source) ? currentUrl : ""
        }

        function addCamera() {
            const at = _videoSettings.addCamera("", _videoSettings.rtspVideoSource, "")
            if (at >= 0) {
                selectedIndex = at
            }
        }

        function removeCamera(camIndex) {
            _videoSettings.removeCamera(camIndex)
            selectedIndex = -1
        }

        function confirmRemove(camIndex, name) {
            mainWindow.showMessageDialog(qsTr("Remove Camera"), qsTr("Remove “%1”?").arg(name), Dialog.Ok | Dialog.Cancel,
                                         () => camList.removeCamera(camIndex))
        }

        Component.onCompleted: reload()

        Connections {
            target: _videoSettings.cameras
            function onRawValueChanged() { camList.reload() }
        }

        Column {
            Layout.fillWidth:   true
            Layout.leftMargin:  -camList._margins
            Layout.rightMargin: -camList._margins

            Repeater {
                model: camerasModel

                Column {
                    id:    camEntry
                    width: parent.width

                    readonly property int    _index:  model.camIndex
                    readonly property string _name:   model.camName
                    readonly property string _source: model.camSource
                    readonly property string _url:    model.camUrl

                    readonly property bool   _open:     camList.selectedIndex === _index
                    readonly property bool   _needsUrl: _sourceNeedsUrl(_source) && _url === ""
                    readonly property string _title:    _name !== "" ? _name : qsTr("Camera %1").arg(_index + 1)

                    PlanGroupRow {
                        text:          camEntry._title
                        interactive:   true
                        current:       camEntry._open
                        showSeparator: camEntry._index > 0
                        onClicked:     camList.selectedIndex = camEntry._open ? -1 : camEntry._index

                        Rectangle {
                            anchors.verticalCenter: parent.verticalCenter
                            width:                  ScreenTools.defaultFontPixelHeight * 0.5
                            height:                 width
                            radius:                 width / 2
                            color:                  camList._qgcPal.colorGreen
                            visible:                camEntry._index < _videoManager.cameraStatuses.length &&
                                                        _videoManager.cameraStatuses[camEntry._index] === ""
                        }

                        QGCLabel {
                            anchors.verticalCenter: parent.verticalCenter
                            text:                   camEntry._needsUrl ? qsTr("Needs URL") : _sourceDisplay(camEntry._source)
                            color:                  camEntry._needsUrl ? camList._qgcPal.colorOrange
                                                                       : Qt.alpha(camList._qgcPal.text, 0.6)
                        }
                    }

                    Column {
                        x:       camList._indent
                        width:   camEntry.width - camList._indent
                        visible: camEntry._open

                        onVisibleChanged: {
                            if (!visible && (nameField.text !== camEntry._name || urlField.text !== camEntry._url)) {
                                camList.saveCamera(camEntry._index, nameField.text, camEntry._source, urlField.text)
                            }
                        }

                        PlanGroupRow {
                            text:    qsTr("Name")

                            QGCTextField {
                                id:                     nameField
                                objectName:             "camNameField"
                                anchors.verticalCenter: parent.verticalCenter
                                width:                  ScreenTools.defaultFontPixelWidth * 20
                                showFrame:              false
                                horizontalAlignment:    TextInput.AlignRight
                                text:                   camEntry._name
                                placeholderText:        qsTr("Camera %1").arg(camEntry._index + 1)
                                onEditingFinished:      camList.saveCamera(camEntry._index, text, camEntry._source, camEntry._url)
                            }
                        }

                        PlanGroupRow {
                            id:          sourceRow
                            objectName:  "camSourceRow"
                            text:        qsTr("Source")
                            interactive: true
                            onClicked:   sourceMenu.openFrom(sourceRow)

                            QGCLabel {
                                anchors.verticalCenter: parent.verticalCenter
                                text:                   _sourceDisplay(camEntry._source)
                                color:                  Qt.alpha(camList._qgcPal.text, 0.6)
                            }

                            QGCColoredImage {
                                anchors.verticalCenter: parent.verticalCenter
                                height:                 ScreenTools.defaultFontPixelHeight / 2
                                width:                  height
                                source:                 "/res/DropArrow.svg"
                                color:                  Qt.alpha(camList._qgcPal.text, 0.6)
                            }

                            OverlayPopover {
                                id: sourceMenu

                                Repeater {
                                    model: _videoSettings.offeredSources()

                                    OverlayMenuItem {
                                        text:      _sourceDisplay(modelData)
                                        checkable: true
                                        checked:   modelData === camEntry._source

                                        onClicked: {
                                            sourceMenu.close()
                                            const source = modelData
                                            if (source !== camEntry._source) {
                                                camList.saveCamera(camEntry._index, camEntry._name, source,
                                                                   camList.urlForSource(source, camEntry._url))
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        PlanGroupRow {
                            text:    qsTr("URL")
                            visible: _sourceNeedsUrl(camEntry._source)

                            QGCTextField {
                                id:                     urlField
                                objectName:             "camUrlField"
                                anchors.verticalCenter: parent.verticalCenter
                                width:                  Math.min(_fieldWidth, camEntry.width * 0.6)
                                showFrame:              false
                                horizontalAlignment:    TextInput.AlignRight
                                text:                   camEntry._url
                                placeholderText:        qsTr("Stream URL")
                                onEditingFinished:      camList.saveCamera(camEntry._index, camEntry._name, camEntry._source, text)
                            }
                        }

                        PlanGroupRow {
                            text:        qsTr("Remove Camera")
                            textColor:   camList._qgcPal.colorRed
                            interactive: true
                            onClicked:   camList.confirmRemove(camEntry._index, camEntry._title)
                        }
                    }
                }
            }

            QGCLabel {
                width:     parent.width
                visible:   !camList.readable
                text:      qsTr("The saved camera list cannot be read, so it is left as it is.")
                color:     camList._qgcPal.colorRed
                wrapMode:  Text.WordWrap
            }

            PlanGroupRow {
                objectName:  "addCameraRow"
                text:        "＋  " + qsTr("Add Camera")
                textColor:   camList._qgcPal.primaryButton
                interactive: camList.readable
                onClicked:   camList.addCamera()
            }
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        visible:            _isGST && _videoSettings.multiViewEnabled.visible
        description:        qsTr("Additional cameras appear picture-in-picture over the main view.")

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Show all cameras")
            fact:               _videoSettings.multiViewEnabled
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Stream")
        visible:            _isStreamSource

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Connection Timeout")
            fact:               _videoSettings.rtspTimeout
            visible:            !_videoAutoStreamConfig && _isStreamSource && _isGST && fact.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Aspect Ratio")
            fact:               _videoSettings.aspectRatio
            visible:            !_videoAutoStreamConfig && _isStreamSource && _videoSettings.aspectRatio.visible
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Stop recording when disarmed")
            fact:               _videoSettings.disableWhenDisarmed
            visible:            !_videoAutoStreamConfig && _isStreamSource && fact.visible
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Low latency mode")
            fact:               _videoSettings.lowLatencyMode
            visible:            !_videoAutoStreamConfig && _isStreamSource && fact.visible && _isGST
        }

        LabelledFactComboBox {
            Layout.fillWidth:   true
            label:              qsTr("Video Decode Priority")
            fact:               _videoSettings.forceVideoDecoder
            visible:            fact.visible
            indexModel:         false
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth: true
        heading:            qsTr("Local Video Storage")

        LabelledFactComboBox {
            Layout.fillWidth:   true
            label:              qsTr("File Format")
            fact:               _videoSettings.recordingFormat
            visible:            _videoSettings.recordingFormat.visible
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Delete old recordings automatically")
            fact:               _videoSettings.enableStorageLimit
            visible:            fact.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Storage Limit")
            fact:               _videoSettings.maxVideoSize
            visible:            fact.visible
            enabled:            _videoSettings.enableStorageLimit.rawValue
        }
    }
}
