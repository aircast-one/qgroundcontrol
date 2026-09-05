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
    function _cameraTitle(camIndex) {
        const cam = camerasModel.get(camIndex)
        return cam && cam.camName !== "" ? cam.camName : qsTr("Camera %1").arg(camIndex + 1)
    }

    SettingsGroupLayout {
        id:                 camList
        Layout.fillWidth:   true
        heading:            qsTr("Cameras")
        description:        qsTr("Additional cameras appear picture-in-picture over the main view.")
        visible:            _isGST

        property int    selectedIndex:  -1
        property string selectedSource: ""

        readonly property bool _selectedLocked: selectedIndex === 0 && _videoAutoStreamConfig

        function selectedCamera() {
            return selectedIndex >= 0 && selectedIndex < camerasModel.count ? camerasModel.get(selectedIndex) : null
        }

        ListModel { id: camerasModel }

        function parseExtras() {
            try { return JSON.parse(_videoSettings.extraVideoSources.rawValue || "[]") }
            catch (e) { console.warn("VideoSettings: invalid extraVideoSources JSON:", e); return [] }
        }

        function reload() {
            const known = _videoSettings.videoSource.enumValues
            const extras = parseExtras().map((cam, i) => ({
                camIndex:  i + 1,
                isPrimary: false,
                camName:   cam.name || "",
                camSource: known.indexOf(cam.source || "") < 0 ? _videoSettings.disabledVideoSource : cam.source,
                camUrl:    cam.url || ""
            }))
            camerasModel.clear()
            camerasModel.append({
                camIndex:  0,
                isPrimary: true,
                camName:   _videoSettings.primaryCameraName.rawValue,
                camSource: _videoSettings.videoSource.rawValue,
                camUrl:    _primaryUrl()
            })
            extras.forEach(cam => camerasModel.append(cam))
            syncFields()
        }

        function saveCamera(camIndex, name, source, url) {
            if (camIndex === 0) {
                _videoSettings.primaryCameraName.rawValue = name
                _videoSettings.videoSource.rawValue = source
                _setPrimaryUrl(source, url)
            } else {
                const arr = parseExtras()
                _videoSettings.extraVideoSources.rawValue = JSON.stringify(
                    arr.map((cam, i) => i === camIndex - 1 ? { name: name, source: source, url: url } : cam))
            }
            reload()
        }

        function saveSelected(name, source, url) {
            if (selectedCamera()) saveCamera(selectedIndex, name, source, url)
        }

        function urlForSource(camIndex, source) {
            const fact = camIndex === 0 ? _primaryUrlFact(source) : null
            return fact ? fact.rawValue : ""
        }

        function addCamera() {
            const arr = parseExtras()
            _videoSettings.extraVideoSources.rawValue = JSON.stringify([...arr, { name: "", source: _videoSettings.disabledVideoSource, url: "" }])
            reload()
            selectedIndex = arr.length + 1
            nameField.forceActiveFocus()
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

        function confirmRemove(camIndex) {
            const row = camerasModel.get(camIndex)
            const name = row.camName !== "" ? row.camName : qsTr("Camera %1").arg(camIndex + 1)
            mainWindow.showMessageDialog(qsTr("Remove Camera"), qsTr("Remove “%1”?").arg(name), Dialog.Ok | Dialog.Cancel,
                                         function() { camList.removeCamera(camIndex) })
        }

        function syncFields() {
            const cam = selectedCamera()
            selectedSource = cam ? cam.camSource : ""
            nameField.text = cam ? cam.camName : ""
            urlField.text = cam ? cam.camUrl : ""
            sourceCombo.currentIndex = cam ? Math.max(0, _videoSettings.videoSource.enumValues.indexOf(cam.camSource)) : 0
        }

        onSelectedIndexChanged: syncFields()
        Component.onCompleted:  reload()

        Connections {
            target: _videoSettings.videoSource
            function onRawValueChanged() { camList.reload() }
        }

        component MiniButton: Rectangle {
            property string label
            property bool   enabled: true
            signal clicked

            readonly property real _size: Math.round(ScreenTools.defaultFontPixelHeight * 1.5)

            implicitWidth:      Math.max(_size, labelItem.implicitWidth + ScreenTools.defaultFontPixelWidth * 2)
            implicitHeight:     _size
            color:              QGroundControl.globalPalette.button
            border.color:       QGroundControl.globalPalette.groupBorder
            border.width:       1
            opacity:            enabled ? 1 : 0.4

            QGCLabel {
                id:                 labelItem
                anchors.centerIn:   parent
                text:               parent.label
            }

            QGCMouseArea {
                anchors.fill:   parent
                enabled:        parent.enabled
                onClicked:      parent.clicked()
            }
        }

        ColumnLayout {
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelHeight / 4

            Rectangle {
                Layout.fillWidth:   true
                implicitHeight:     camTableColumn.implicitHeight + 2
                color:              QGroundControl.globalPalette.window
                border.color:       QGroundControl.globalPalette.groupBorder
                border.width:       1
                radius:             Math.round(ScreenTools.defaultFontPixelHeight * 0.3)
                clip:               true

                ColumnLayout {
                    id:                 camTableColumn
                    anchors.fill:       parent
                    anchors.margins:    1
                    spacing:            0

                    Repeater {
                        model: camerasModel

                        delegate: Rectangle {
                            id:                 camRow
                            Layout.fillWidth:   true
                            implicitHeight:     Math.round(ScreenTools.defaultFontPixelHeight * 1.7)
                            color:              selected ? QGroundControl.globalPalette.buttonHighlight
                                                         : index % 2 ? QGroundControl.globalPalette.windowShadeDark : "transparent"

                            readonly property bool selected:  camList.selectedIndex === model.camIndex
                            readonly property bool receiving: {
                                const statuses = _videoManager.cameraStatuses
                                return model.camIndex < statuses.length && statuses[model.camIndex] === ""
                            }

                            RowLayout {
                                anchors.fill:           parent
                                anchors.leftMargin:     ScreenTools.defaultFontPixelWidth
                                anchors.rightMargin:    ScreenTools.defaultFontPixelWidth
                                spacing:                ScreenTools.defaultFontPixelWidth * 2

                                QGCLabel {
                                    Layout.fillWidth:   true
                                    text:               model.camName !== "" ? model.camName : qsTr("Camera %1").arg(model.camIndex + 1)
                                    elide:              Text.ElideRight
                                    color:              camRow.selected ? QGroundControl.globalPalette.buttonHighlightText : QGroundControl.globalPalette.text
                                }

                                Rectangle {
                                    width:      ScreenTools.defaultFontPixelHeight * 0.5
                                    height:     width
                                    radius:     width / 2
                                    color:      QGroundControl.globalPalette.colorGreen
                                    visible:    camRow.receiving
                                }

                                QGCLabel {
                                    readonly property bool _unconfigured: _sourceNeedsUrl(model.camSource) && model.camUrl === ""

                                    text:   _unconfigured ? qsTr("Needs URL") : _sourceDisplay(model.camSource)
                                    color:  camRow.selected ? QGroundControl.globalPalette.buttonHighlightText
                                                            : _unconfigured ? QGroundControl.globalPalette.colorOrange
                                                                            : QGroundControl.globalPalette.colorGrey
                                }
                            }

                            QGCMouseArea {
                                anchors.fill:   parent
                                onClicked:      camList.selectedIndex = model.camIndex
                            }
                        }
                    }
                }
            }

            RowLayout {
                Layout.fillWidth:   true
                spacing:            0

                readonly property real _radius: Math.round(ScreenTools.defaultFontPixelHeight * 0.3)

                MiniButton {
                    label:              "+"
                    topLeftRadius:      parent._radius
                    bottomLeftRadius:   parent._radius
                    onClicked:          camList.addCamera()
                }

                MiniButton {
                    Layout.leftMargin:  -1
                    label:              "−"
                    topRightRadius:     parent._radius
                    bottomRightRadius:  parent._radius
                    enabled:            camList.selectedIndex > 0
                    onClicked:          camList.confirmRemove(camList.selectedIndex)
                }

                Item { Layout.fillWidth: true }
            }
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Show all cameras")
            fact:               _videoSettings.multiViewEnabled
            visible:            fact.visible
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            camList.selectedIndex >= 0 ? _cameraTitle(camList.selectedIndex) : ""
        description:        camList._selectedLocked ? qsTr("Camera 1 is configured automatically over MAVLink.") : ""
        visible:            _isGST && camList.selectedIndex >= 0
        enabled:            !camList._selectedLocked

        RowLayout {
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelWidth * 2

            QGCLabel { Layout.fillWidth: true; text: qsTr("Name") }

            QGCTextField {
                id:                     nameField
                objectName:             "camNameField"
                Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 24
                placeholderText:        qsTr("Camera %1").arg(camList.selectedIndex + 1)
                onEditingFinished:      camList.saveSelected(text, camList.selectedSource, urlField.text)
            }
        }

        LabelledComboBox {
            id:         sourceCombo
            objectName: "camSourceCombo"
            label:      qsTr("Source")
            model:      _videoSettings.videoSource.enumStrings.map(_sourceLabel)
            onActivated: (i) => {
                const source = _videoSettings.videoSource.enumValues[i]
                if (source !== camList.selectedSource) {
                    camList.saveSelected(nameField.text, source, camList.urlForSource(camList.selectedIndex, source))
                }
            }
        }

        RowLayout {
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelWidth * 2
            visible:            _sourceNeedsUrl(camList.selectedSource)

            QGCLabel { Layout.fillWidth: true; text: qsTr("URL") }

            QGCTextField {
                id:                     urlField
                objectName:             "camUrlField"
                Layout.preferredWidth:  _fieldWidth
                placeholderText:        qsTr("Stream URL")
                onEditingFinished:      camList.saveSelected(nameField.text, camList.selectedSource, text)
            }
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
