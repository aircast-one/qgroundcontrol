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
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Controls
import QGroundControl.ScreenTools

SettingsPage {
    property var    _settingsManager:       QGroundControl.settingsManager
    property var    _videoManager:          QGroundControl.videoManager
    property var    _videoSettings:         _settingsManager.videoSettings
    property bool   _isGST:                 _videoManager.gstreamerEnabled
    property bool   _isStreamSource:        _videoManager.isStreamSource
    property bool   _videoAutoStreamConfig: _videoManager.autoStreamConfigured
    property real   _fieldWidth:            ScreenTools.defaultFontPixelWidth * 40

    function _primaryUrlFact(source) {
        if (source === _videoSettings.udp264VideoSource || source === _videoSettings.udp265VideoSource || source === _videoSettings.mpegtsVideoSource)
            return _videoSettings.udpUrl
        if (source === _videoSettings.rtspVideoSource)   return _videoSettings.rtspUrl
        if (source === _videoSettings.tcpVideoSource)    return _videoSettings.tcpUrl
        if (source === _videoSettings.webrtcVideoSource) return _videoSettings.whepUrl
        return null
    }
    function _sourceNeedsUrl(src) { return _primaryUrlFact(src) !== null }
    function _primaryUrl() { var f = _primaryUrlFact(_videoSettings.videoSource.rawValue); return f ? f.rawValue : "" }
    function _setPrimaryUrl(source, url) { var f = _primaryUrlFact(source); if (f) f.rawValue = url }

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

        function parseExtras() {
            try { return JSON.parse(_videoSettings.extraVideoSources.rawValue || "[]") }
            catch (e) { console.warn("VideoSettings: invalid extraVideoSources JSON:", e); return [] }
        }

        function camRow(camIndex, name, source, url) {
            return { camIndex: camIndex, camName: name, camSource: source, camUrl: url }
        }

        function reload() {
            const known = _videoSettings.videoSource.enumValues
            const extras = parseExtras().map((cam, i) => camRow(i + 1, cam.name || "",
                known.indexOf(cam.source || "") < 0 ? _videoSettings.disabledVideoSource : cam.source, cam.url || ""))
            camerasModel.clear()
            camerasModel.append(camRow(0, _videoSettings.primaryCameraName.rawValue, _videoSettings.videoSource.rawValue, _primaryUrl()))
            extras.forEach(cam => camerasModel.append(cam))
        }

        function saveCamera(camIndex, name, source, url) {
            const extras = camIndex === 0 ? null : parseExtras()
            if (extras && extras.length !== camerasModel.count - 1) {
                reload()
                return
            }
            camerasModel.set(camIndex, camRow(camIndex, name, source, url))
            if (camIndex === 0) {
                _videoSettings.primaryCameraName.rawValue = name
                _setPrimaryUrl(source, url)
                _videoSettings.videoSource.rawValue = source
            } else {
                _videoSettings.extraVideoSources.rawValue = JSON.stringify(
                    extras.map((cam, i) => i === camIndex - 1 ? { name: name, source: source, url: url } : cam))
            }
        }

        function urlForSource(camIndex, source, currentUrl) {
            const fact = camIndex === 0 ? _primaryUrlFact(source) : null
            return fact ? fact.rawValue : _sourceNeedsUrl(source) ? currentUrl : ""
        }

        function addCamera() {
            const arr = parseExtras()
            _videoSettings.extraVideoSources.rawValue = JSON.stringify([...arr, { name: "", source: _videoSettings.disabledVideoSource, url: "" }])
            camerasModel.append(camRow(arr.length + 1, "", _videoSettings.disabledVideoSource, ""))
            selectedIndex = arr.length + 1
        }

        function removeCamera(camIndex) {
            _videoSettings.extraVideoSources.rawValue = JSON.stringify(parseExtras().filter((_, i) => i !== camIndex - 1))
            if (_videoManager.activeVideoSource === camIndex) {
                _videoManager.setActiveVideoSource(0)
            } else if (_videoManager.activeVideoSource > camIndex) {
                _videoManager.setActiveVideoSource(_videoManager.activeVideoSource - 1)
            }
            selectedIndex = -1
            reload()
        }

        function reloadIfStale() {
            if (camerasModel.count === 0 || camerasModel.get(0).camSource !== _videoSettings.videoSource.rawValue) reload()
        }

        function confirmRemove(camIndex, name) {
            mainWindow.showMessageDialog(qsTr("Remove Camera"), qsTr("Remove “%1”?").arg(name), Dialog.Ok | Dialog.Cancel,
                                         () => camList.removeCamera(camIndex))
        }

        Component.onCompleted: reload()

        Connections {
            target: _videoSettings.videoSource
            function onRawValueChanged() { camList.reloadIfStale() }
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
                    readonly property bool   _locked:   _index === 0 && _videoAutoStreamConfig
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
                            enabled: !camEntry._locked

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
                            description: camEntry._locked ? qsTr("Configured automatically over MAVLink.") : ""
                            interactive: !camEntry._locked
                            enabled:     !camEntry._locked
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
                                    model: _videoSettings.videoSource.enumStrings

                                    OverlayMenuItem {
                                        text:      _sourceLabel(modelData)
                                        checkable: true
                                        checked:   _videoSettings.videoSource.enumValues[index] === camEntry._source

                                        onClicked: {
                                            sourceMenu.close()
                                            const source = _videoSettings.videoSource.enumValues[index]
                                            if (source !== camEntry._source) {
                                                camList.saveCamera(camEntry._index, camEntry._name, source,
                                                                   camList.urlForSource(camEntry._index, source, camEntry._url))
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        PlanGroupRow {
                            text:    qsTr("URL")
                            enabled: !camEntry._locked
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
                            visible:     camEntry._index > 0
                            onClicked:   camList.confirmRemove(camEntry._index, camEntry._title)
                        }
                    }
                }
            }

            PlanGroupRow {
                objectName:  "addCameraRow"
                text:        "＋  " + qsTr("Add Camera")
                textColor:   camList._qgcPal.primaryButton
                interactive: true
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
