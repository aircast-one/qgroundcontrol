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
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Controls
import QGroundControl.ScreenTools

SettingsPage {
    property var    _settingsManager:            QGroundControl.settingsManager
    property var    _videoManager:              QGroundControl.videoManager
    property var    _videoSettings:             _settingsManager.videoSettings
    property string _videoSource:               _videoSettings.videoSource.rawValue
    property bool   _isGST:                     _videoManager.gstreamerEnabled
    property bool   _isStreamSource:            _videoManager.isStreamSource
    property bool   _isUDP264:                  _isStreamSource && (_videoSource === _videoSettings.udp264VideoSource)
    property bool   _isUDP265:                  _isStreamSource && (_videoSource === _videoSettings.udp265VideoSource)
    property bool   _isRTSP:                    _isStreamSource && (_videoSource === _videoSettings.rtspVideoSource)
    property bool   _isTCP:                     _isStreamSource && (_videoSource === _videoSettings.tcpVideoSource)
    property bool   _isMPEGTS:                  _isStreamSource && (_videoSource === _videoSettings.mpegtsVideoSource)
    property bool   _isWHEP:                    _isStreamSource && (_videoSource === _videoSettings.webrtcVideoSource)
    property bool   _videoAutoStreamConfig:     _videoManager.autoStreamConfigured
    property bool   _videoSourceDisabled:       _videoSource === _videoSettings.disabledVideoSource
    property real   _urlFieldWidth:             ScreenTools.defaultFontPixelWidth * 40
    property bool   _requiresUDPUrl:            _isUDP264 || _isUDP265 || _isMPEGTS

    function _sourceNeedsUrl(src) {
        return src === _videoSettings.rtspVideoSource || src === _videoSettings.tcpVideoSource ||
               src === _videoSettings.udp264VideoSource || src === _videoSettings.udp265VideoSource ||
               src === _videoSettings.mpegtsVideoSource || src === _videoSettings.webrtcVideoSource
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Video Source")
        headingDescription: _videoAutoStreamConfig ? qsTr("Mavlink camera stream is automatically configured") : ""
        enabled:            !_videoAutoStreamConfig

        LabelledFactComboBox {
            Layout.fillWidth:   true
            label:              qsTr("Source")
            indexModel:         false
            fact:               _videoSettings.videoSource
            visible:            fact.visible
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Connection")
        visible:            !_videoSourceDisabled && !_videoAutoStreamConfig && (_isTCP || _isRTSP || _isWHEP || _requiresUDPUrl)

        LabelledFactTextField {
            Layout.fillWidth:           true
            textFieldPreferredWidth:    _urlFieldWidth
            label:                      qsTr("RTSP URL")
            fact:                       _videoSettings.rtspUrl
            visible:                    _isRTSP && _videoSettings.rtspUrl.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:           true
            textFieldPreferredWidth:    _urlFieldWidth
            label:                      qsTr("WHEP URL")
            fact:                       _videoSettings.whepUrl
            visible:                    _isWHEP && _videoSettings.whepUrl.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:           true
            label:                      qsTr("TCP URL")
            textFieldPreferredWidth:    _urlFieldWidth
            fact:                       _videoSettings.tcpUrl
            visible:                    _isTCP && _videoSettings.tcpUrl.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:           true
            textFieldPreferredWidth:    _urlFieldWidth
            label:                      qsTr("UDP URL")
            fact:                       _videoSettings.udpUrl
            visible:                    _requiresUDPUrl && _videoSettings.udpUrl.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:           true
            textFieldPreferredWidth:    _urlFieldWidth
            label:                      qsTr("Connection Timeout")
            fact:                       _videoSettings.rtspTimeout
            visible:                    (_isRTSP || _isWHEP) && _videoSettings.rtspTimeout.visible
        }
    }

    SettingsGroupLayout {
        id:                 camList
        Layout.fillWidth:   true
        heading:            qsTr("Additional Cameras")
        headingDescription: qsTr("Extra camera streams. Use the switch button on the video to cycle between all cameras.")
        visible:            !_videoAutoStreamConfig && _isGST

        ListModel { id: extraCamsModel }

        Component.onCompleted: {
            var arr = []
            try { arr = JSON.parse(_videoSettings.extraVideoSources.rawValue || "[]") } catch (e) { arr = [] }
            var known = _videoSettings.videoSource.enumValues
            for (var i = 0; i < arr.length; ++i) {
                var src = arr[i].source || ""
                if (known.indexOf(src) < 0) src = _videoSettings.disabledVideoSource
                extraCamsModel.append({ source: src, url: arr[i].url || "" })
            }
        }

        function saveExtraCams() {
            var arr = []
            for (var i = 0; i < extraCamsModel.count; ++i) {
                var row = extraCamsModel.get(i)
                arr.push({ source: row.source, url: row.url })
            }
            _videoSettings.extraVideoSources.rawValue = JSON.stringify(arr)
        }

        Repeater {
            model: extraCamsModel
            delegate: RowLayout {
                id:                 camRow
                Layout.fillWidth:   true
                spacing:            ScreenTools.defaultFontPixelWidth * 2

                property int    rowIndex:   index
                property string rowSource:  source
                property string rowUrl:     url

                QGCLabel {
                    Layout.fillWidth:   true
                    text:               qsTr("Camera %1").arg(camRow.rowIndex + 2) // Camera 1 is the primary source
                }

                QGCTextField {
                    Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 28
                    visible:                _sourceNeedsUrl(camRow.rowSource)
                    text:                   camRow.rowUrl
                    placeholderText:        qsTr("Stream URL")
                    onEditingFinished: {
                        extraCamsModel.setProperty(camRow.rowIndex, "url", text)
                        camList.saveExtraCams()
                    }
                }

                QGCComboBox {
                    sizeToContents:     true
                    model:              _videoSettings.videoSource.enumStrings
                    currentIndex:       Math.max(0, _videoSettings.videoSource.enumValues.indexOf(camRow.rowSource))
                    onActivated: (activeIndex) => {
                        extraCamsModel.setProperty(camRow.rowIndex, "source", _videoSettings.videoSource.enumValues[activeIndex])
                        camList.saveExtraCams()
                    }
                }

                QGCColoredImage {
                    height:             ScreenTools.minTouchPixels
                    width:              height
                    sourceSize.height:  height
                    fillMode:           Image.PreserveAspectFit
                    mipmap:             true
                    smooth:             true
                    color:              QGroundControl.globalPalette.text
                    source:             "/res/TrashDelete.svg"

                    QGCMouseArea {
                        fillItem: parent
                        onClicked: {
                            extraCamsModel.remove(camRow.rowIndex)
                            camList.saveExtraCams()
                            _videoManager.setActiveVideoSource(0)
                        }
                    }
                }
            }
        }

        LabelledButton {
            label:      qsTr("Add camera stream")
            buttonText: qsTr("Add")
            onClicked: {
                extraCamsModel.append({ source: _videoSettings.disabledVideoSource, url: "" })
                camList.saveExtraCams()
            }
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Settings")
        visible:            !_videoSourceDisabled

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
