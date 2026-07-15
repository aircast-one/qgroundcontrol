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

    // Single source of truth for which sources need a URL and, for Camera 1, which fact holds it.
    // (Extra cameras store their URL in the extraVideoSources JSON instead.)
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

    function _sourceDisplay(src) {
        if (src === "" || src === _videoSettings.disabledVideoSource) return qsTr("Disabled")
        var i = _videoSettings.videoSource.enumValues.indexOf(src)
        return i >= 0 ? _videoSettings.videoSource.enumStrings[i] : src
    }

    SettingsGroupLayout {
        id:                 camList
        Layout.fillWidth:   true
        heading:            qsTr("Cameras")
        headingDescription: _videoAutoStreamConfig
                                ? qsTr("Camera 1 is configured automatically over MAVLink.")
                                : qsTr("Main view shows the selected camera. Use the switch button on the video to cycle between them.")
        visible:            _isGST

        // camerasModel row index == camIndex (row 0 = Camera 1, rows 1..N = extras in order).
        // openEditor/saveCamera/removeCamera rely on this — keep reload() appending in order.
        ListModel { id: camerasModel }

        function parseExtras() {
            try { return JSON.parse(_videoSettings.extraVideoSources.rawValue || "[]") }
            catch (e) { console.warn("VideoSettings: invalid extraVideoSources JSON:", e); return [] }
        }

        function reload() {
            camerasModel.clear()
            camerasModel.append({
                camIndex:  0,
                isPrimary: true,
                camName:   _videoSettings.primaryCameraName.rawValue,
                camSource: _videoSettings.videoSource.rawValue,
                camUrl:    _primaryUrl()
            })
            var known = _videoSettings.videoSource.enumValues
            var arr = parseExtras()
            for (var i = 0; i < arr.length; ++i) {
                var src = arr[i].source || ""
                if (known.indexOf(src) < 0) src = _videoSettings.disabledVideoSource
                camerasModel.append({
                    camIndex:  i + 1,
                    isPrimary: false,
                    camName:   arr[i].name || "",
                    camSource: src,
                    camUrl:    arr[i].url || ""
                })
            }
        }

        function saveCamera(camIndex, name, source, url) {
            if (camIndex === 0) {
                _videoSettings.primaryCameraName.rawValue = name
                _videoSettings.videoSource.rawValue = source
                _setPrimaryUrl(source, url)
            } else {
                var arr = parseExtras()
                arr[camIndex - 1] = { name: name, source: source, url: url }
                _videoSettings.extraVideoSources.rawValue = JSON.stringify(arr)
            }
            reload()
        }

        function addCamera() {
            var arr = parseExtras()
            arr.push({ name: "", source: _videoSettings.disabledVideoSource, url: "" })
            _videoSettings.extraVideoSources.rawValue = JSON.stringify(arr)
            reload()
        }

        function removeCamera(camIndex) {
            var arr = parseExtras()
            arr.splice(camIndex - 1, 1)
            _videoSettings.extraVideoSources.rawValue = JSON.stringify(arr)
            _videoManager.setActiveVideoSource(0)
            reload()
        }

        function openEditor(camIndex) {
            var row = camerasModel.get(camIndex)
            cameraDialogComponent.createObject(mainWindow, {
                editIndex:  camIndex,
                initName:   row.camName,
                initSource: row.camSource,
                initUrl:    row.camUrl
            }).open()
        }

        Component.onCompleted: reload()

        // Refresh Camera 1 when MAVLink auto-stream rewrites its source externally.
        // Extra-camera edits go through addCamera/removeCamera/saveCamera, which reload directly.
        Connections {
            target: _videoSettings.videoSource
            function onRawValueChanged() { camList.reload() }
        }

        Repeater {
            model: camerasModel

            delegate: RowLayout {
                id:                 camRow
                Layout.fillWidth:   true
                spacing:            ScreenTools.defaultFontPixelWidth * 2

                // cameraReceiving() isn't a bindable property, so recompute on the relevant signals.
                property bool receiving: false
                function refreshReceiving() { receiving = _videoManager.cameraReceiving(model.camIndex) }
                Component.onCompleted: refreshReceiving()
                Connections {
                    target: _videoManager
                    function onVideoReceivingChanged() { camRow.refreshReceiving() }
                    function onActiveVideoSourceChanged() { camRow.refreshReceiving() }
                }

                Rectangle {
                    width:              ScreenTools.defaultFontPixelHeight * 0.7
                    height:             width
                    radius:             width / 2
                    color:              camRow.receiving ? QGroundControl.globalPalette.colorGreen : QGroundControl.globalPalette.colorGrey
                    border.color:       QGroundControl.globalPalette.text
                    border.width:       1
                }

                QGCLabel {
                    Layout.fillWidth:   true
                    text:               model.camName !== "" ? model.camName : qsTr("Camera %1").arg(model.camIndex + 1)
                    elide:              Text.ElideRight
                }

                QGCLabel {
                    text:               _sourceDisplay(model.camSource)
                    opacity:            0.7
                }

                QGCColoredImage {
                    height:             ScreenTools.minTouchPixels
                    width:              height
                    sourceSize.height:  height
                    fillMode:           Image.PreserveAspectFit
                    mipmap:             true
                    smooth:             true
                    color:              QGroundControl.globalPalette.text
                    source:             "/res/pencil.svg"
                    enabled:            !(model.isPrimary && _videoAutoStreamConfig)
                    opacity:            enabled ? 1 : 0.3

                    QGCMouseArea {
                        fillItem:   parent
                        enabled:    parent.enabled
                        onClicked:  camList.openEditor(model.camIndex)
                    }
                }

                QGCColoredImage {
                    height:             ScreenTools.minTouchPixels
                    width:              height
                    sourceSize.height:  height
                    fillMode:           Image.PreserveAspectFit
                    mipmap:             true
                    smooth:             true
                    visible:            !model.isPrimary
                    color:              QGroundControl.globalPalette.text
                    source:             "/res/TrashDelete.svg"

                    QGCMouseArea {
                        fillItem:   parent
                        onClicked:  camList.removeCamera(model.camIndex)
                    }
                }
            }
        }

        LabelledButton {
            label:      qsTr("Add camera stream")
            buttonText: qsTr("Add")
            onClicked:  camList.addCamera()
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Show all cameras at once (picture-in-picture)")
            fact:               _videoSettings.multiViewEnabled
            visible:            fact.visible
        }
    }

    Component {
        id: cameraDialogComponent

        QGCPopupDialog {
            id:         dlg
            title:      qsTr("Camera %1").arg(editIndex + 1)
            buttons:    Dialog.Save | Dialog.Cancel

            property int    editIndex
            property string initName
            property string initSource
            property string initUrl
            property string dlgSource: initSource

            onAccepted: camList.saveCamera(editIndex, nameField.text, dlg.dlgSource,
                                           _sourceNeedsUrl(dlg.dlgSource) ? urlField.text : "")

            ColumnLayout {
                spacing: ScreenTools.defaultFontPixelHeight / 2

                RowLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelWidth * 2
                    QGCLabel { Layout.fillWidth: true; text: qsTr("Name") }
                    QGCTextField {
                        id:                     nameField
                        Layout.preferredWidth:  _fieldWidth
                        text:                   dlg.initName
                        placeholderText:        qsTr("Camera %1").arg(dlg.editIndex + 1)
                    }
                }

                RowLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelWidth * 2
                    QGCLabel { Layout.fillWidth: true; text: qsTr("Source") }
                    QGCComboBox {
                        Layout.preferredWidth:  _fieldWidth
                        model:                  _videoSettings.videoSource.enumStrings
                        currentIndex:           Math.max(0, _videoSettings.videoSource.enumValues.indexOf(dlg.initSource))
                        onActivated: (i) => {
                            var newSource = _videoSettings.videoSource.enumValues[i]
                            if (newSource !== dlg.dlgSource) urlField.text = ""   // old URL doesn't apply to a different protocol
                            dlg.dlgSource = newSource
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelWidth * 2
                    visible:            _sourceNeedsUrl(dlg.dlgSource)
                    QGCLabel { Layout.fillWidth: true; text: qsTr("URL") }
                    QGCTextField {
                        id:                     urlField
                        Layout.preferredWidth:  _fieldWidth
                        text:                   dlg.initUrl
                        placeholderText:        qsTr("Stream URL")
                    }
                }
            }
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Settings")
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
            text:               qsTr("Low Latency Mode")
            fact:               _videoSettings.lowLatencyMode
            visible:            !_videoAutoStreamConfig && _isStreamSource && fact.visible && _isGST
        }

        LabelledFactComboBox {
            Layout.fillWidth:   true
            label:              qsTr("Video decode priority")
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
            label:              qsTr("Record File Format")
            fact:               _videoSettings.recordingFormat
            visible:            _videoSettings.recordingFormat.visible
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Auto-Delete Saved Recordings")
            fact:               _videoSettings.enableStorageLimit
            visible:            fact.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Max Storage Usage")
            fact:               _videoSettings.maxVideoSize
            visible:            fact.visible
            enabled:            _videoSettings.enableStorageLimit.rawValue
        }
    }
}
