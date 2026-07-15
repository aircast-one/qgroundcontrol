/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "VideoSettings.h"
#include "VideoManager.h"

#include <QtQml/QQmlEngine>
#include <QtCore/QVariantList>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>

#ifdef QGC_GST_STREAMING
#include "GStreamer.h"
#endif
#ifndef QGC_DISABLE_UVC
#include "UVCReceiver.h"
#endif

DECLARE_SETTINGGROUP(Video, "Video")
{
    qmlRegisterUncreatableType<VideoSettings>("QGroundControl.SettingsManager", 1, 0, "VideoSettings", "Reference only");

    // Setup enum values for videoSource settings into meta data
    QVariantList videoSourceList;
#if defined(QGC_GST_STREAMING) || defined(QGC_QT_STREAMING)
    videoSourceList.append(videoSourceRTSP);
    videoSourceList.append(videoSourceUDPH264);
    videoSourceList.append(videoSourceUDPH265);
    videoSourceList.append(videoSourceTCP);
    videoSourceList.append(videoSourceMPEGTS);
#ifdef QGC_GST_STREAMING
    // WHEP is implemented only in the GStreamer receiver; QMediaPlayer can't consume it
    videoSourceList.append(videoSourceWebRTC);
#endif
    videoSourceList.append(videoSource3DRSolo);
    videoSourceList.append(videoSourceParrotDiscovery);
    videoSourceList.append(videoSourceYuneecMantisG);

    #ifdef QGC_HERELINK_AIRUNIT_VIDEO
        videoSourceList.append(videoSourceHerelinkAirUnit);
    #else
        videoSourceList.append(videoSourceHerelinkHotspot);
    #endif
#endif
#ifndef QGC_DISABLE_UVC
    videoSourceList.append(UVCReceiver::getDeviceNameList());
#endif
    if (videoSourceList.count() == 0) {
        _noVideo = true;
        videoSourceList.append(videoSourceNoVideo);
        setVisible(false);
    } else {
        videoSourceList.insert(0, videoDisabled);
    }

    // make translated strings
    QStringList videoSourceCookedList;
    for (const QVariant& videoSource: videoSourceList) {
        videoSourceCookedList.append( VideoSettings::tr(videoSource.toString().toStdString().c_str()) );
    }

    _nameToMetaDataMap[videoSourceName]->setEnumInfo(videoSourceCookedList, videoSourceList);

    _setForceVideoDecodeList();

    // Set default value for videoSource
    _setDefaults();
}

void VideoSettings::_setDefaults()
{
    if (_noVideo) {
        _nameToMetaDataMap[videoSourceName]->setRawDefaultValue(videoSourceNoVideo);
    } else {
        _nameToMetaDataMap[videoSourceName]->setRawDefaultValue(videoDisabled);
    }
}

DECLARE_SETTINGSFACT(VideoSettings, aspectRatio)
DECLARE_SETTINGSFACT(VideoSettings, videoFit)
DECLARE_SETTINGSFACT(VideoSettings, gridLines)
DECLARE_SETTINGSFACT(VideoSettings, showRecControl)
DECLARE_SETTINGSFACT(VideoSettings, recordingFormat)
DECLARE_SETTINGSFACT(VideoSettings, maxVideoSize)
DECLARE_SETTINGSFACT(VideoSettings, enableStorageLimit)
DECLARE_SETTINGSFACT(VideoSettings, streamEnabled)
DECLARE_SETTINGSFACT(VideoSettings, disableWhenDisarmed)

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, videoSource)
{
    if (!_videoSourceFact) {
        _videoSourceFact = _createSettingsFact(videoSourceName);
        //-- Check for sources no longer available
        if(!_videoSourceFact->enumValues().contains(_videoSourceFact->rawValue().toString())) {
            if (_noVideo) {
                _videoSourceFact->setRawValue(videoSourceNoVideo);
            } else {
                _videoSourceFact->setRawValue(videoDisabled);
            }
        }
        connect(_videoSourceFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _videoSourceFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, primaryCameraName)
{
    if (!_primaryCameraNameFact) {
        _primaryCameraNameFact = _createSettingsFact(primaryCameraNameName);
        connect(_primaryCameraNameFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _primaryCameraNameFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, extraVideoSources)
{
    if (!_extraVideoSourcesFact) {
        _extraVideoSourcesFact = _createSettingsFact(extraVideoSourcesName);
        connect(_extraVideoSourcesFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _extraVideoSourcesFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, activeVideoSource)
{
    if (!_activeVideoSourceFact) {
        _activeVideoSourceFact = _createSettingsFact(activeVideoSourceName);
        connect(_activeVideoSourceFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _activeVideoSourceFact;
}

QJsonArray VideoSettings::_extraSourcesArray()
{
    return QJsonDocument::fromJson(extraVideoSources()->rawValue().toString().toUtf8()).array();
}

int VideoSettings::videoSourceCount()
{
    return 1 + _extraSourcesArray().size();
}

bool VideoSettings::_isStreamSource(const QString &source)
{
    static const QStringList streamSources = {
        videoSourceUDPH264, videoSourceUDPH265, videoSourceRTSP, videoSourceTCP,
        videoSourceMPEGTS, videoSourceWebRTC, videoSource3DRSolo,
        videoSourceParrotDiscovery, videoSourceYuneecMantisG,
        videoSourceHerelinkAirUnit, videoSourceHerelinkHotspot,
    };
    return streamSources.contains(source);
}

QList<int> VideoSettings::switchableIndices()
{
    QList<int> indices{0};
    const QJsonArray extras = _extraSourcesArray();
    for (int i = 0; i < extras.size(); ++i) {
        if (_isStreamSource(extras.at(i).toObject().value(QStringLiteral("source")).toString())) {
            indices.append(i + 1);
        }
    }
    return indices;
}

int VideoSettings::currentIndex()
{
    const int index = activeVideoSource()->rawValue().toInt();
    return (index >= 0 && index < videoSourceCount()) ? index : 0;
}

QString VideoSettings::currentVideoSourceName()
{
    const int index = currentIndex();
    if (index == 0) {
        return videoSource()->rawValue().toString();
    }
    return _extraSourcesArray().at(index - 1).toObject().value(QStringLiteral("source")).toString();
}

QString VideoSettings::currentVideoUrl()
{
    const int index = currentIndex();
    if (index == 0) {
        const QString source = videoSource()->rawValue().toString();
        if (source == videoSourceUDPH264 || source == videoSourceUDPH265 || source == videoSourceMPEGTS) {
            return udpUrl()->rawValue().toString();
        }
        if (source == videoSourceRTSP) {
            return rtspUrl()->rawValue().toString();
        }
        if (source == videoSourceTCP) {
            return tcpUrl()->rawValue().toString();
        }
        if (source == videoSourceWebRTC) {
            return whepUrl()->rawValue().toString();
        }
        return QString();
    }
    return _extraSourcesArray().at(index - 1).toObject().value(QStringLiteral("url")).toString();
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, forceVideoDecoder)
{
    if (!_forceVideoDecoderFact) {
        _forceVideoDecoderFact = _createSettingsFact(forceVideoDecoderName);

        _forceVideoDecoderFact->setVisible(
#ifdef QGC_GST_STREAMING
            true
#else
            false
#endif
        );

        connect(_forceVideoDecoderFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _forceVideoDecoderFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, lowLatencyMode)
{
    if (!_lowLatencyModeFact) {
        _lowLatencyModeFact = _createSettingsFact(lowLatencyModeName);

        _lowLatencyModeFact->setVisible(
#ifdef QGC_GST_STREAMING
            true
#else
            false
#endif
        );

        connect(_lowLatencyModeFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _lowLatencyModeFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, rtspTimeout)
{
    if (!_rtspTimeoutFact) {
        _rtspTimeoutFact = _createSettingsFact(rtspTimeoutName);

        _rtspTimeoutFact->setVisible(
#ifdef QGC_GST_STREAMING
            true
#else
            false
#endif
        );

        connect(_rtspTimeoutFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _rtspTimeoutFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, udpUrl)
{
    if (!_udpUrlFact) {
        _udpUrlFact = _createSettingsFact(udpUrlName);
        connect(_udpUrlFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _udpUrlFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, rtspUrl)
{
    if (!_rtspUrlFact) {
        _rtspUrlFact = _createSettingsFact(rtspUrlName);
        connect(_rtspUrlFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _rtspUrlFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, tcpUrl)
{
    if (!_tcpUrlFact) {
        _tcpUrlFact = _createSettingsFact(tcpUrlName);
        connect(_tcpUrlFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _tcpUrlFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, whepUrl)
{
    if (!_whepUrlFact) {
        _whepUrlFact = _createSettingsFact(whepUrlName);
        connect(_whepUrlFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _whepUrlFact;
}

bool VideoSettings::streamConfigured(void)
{
    //-- First, check if it's autoconfigured
    if(VideoManager::instance()->autoStreamConfigured()) {
        qCDebug(VideoManagerLog) << "Stream auto configured";
        return true;
    }
    //-- Check if it's disabled (evaluate whichever source is currently active)
    QString vSource = currentVideoSourceName();
    if(vSource == videoSourceNoVideo || vSource == videoDisabled) {
        return false;
    }
    //-- Stream sources that require a URL are configured once that URL is set
    if(vSource == videoSourceUDPH264 || vSource == videoSourceUDPH265 || vSource == videoSourceMPEGTS ||
       vSource == videoSourceRTSP || vSource == videoSourceTCP || vSource == videoSourceWebRTC) {
        return !currentVideoUrl().isEmpty();
    }
    //-- If Herelink Air unit, good to go
    if(vSource == videoSourceHerelinkAirUnit) {
        qCDebug(VideoManagerLog) << "Stream configured for Herelink Air Unit";
        return true;
    }
    //-- If Herelink Hotspot, good to go
    if(vSource == videoSourceHerelinkHotspot) {
        qCDebug(VideoManagerLog) << "Stream configured for Herelink Hotspot";
        return true;
    }
#ifndef QGC_DISABLE_UVC
    if (UVCReceiver::enabled() && UVCReceiver::deviceExists(vSource)) {
        qCDebug(VideoManagerLog) << "Stream configured for UVC";
        return true;
    }
#endif
    return false;
}

void VideoSettings::_configChanged(QVariant)
{
    emit streamConfiguredChanged(streamConfigured());
}

void VideoSettings::_setForceVideoDecodeList()
{
#ifdef QGC_GST_STREAMING
    static const QList<GStreamer::VideoDecoderOptions> removeForceVideoDecodeList{
#if defined(Q_OS_ANDROID)
    GStreamer::VideoDecoderOptions::ForceVideoDecoderDirectX3D,
    GStreamer::VideoDecoderOptions::ForceVideoDecoderVideoToolbox,
    GStreamer::VideoDecoderOptions::ForceVideoDecoderVAAPI,
    GStreamer::VideoDecoderOptions::ForceVideoDecoderNVIDIA,
    GStreamer::VideoDecoderOptions::ForceVideoDecoderIntel,
#elif defined(Q_OS_LINUX)
    GStreamer::VideoDecoderOptions::ForceVideoDecoderDirectX3D,
    GStreamer::VideoDecoderOptions::ForceVideoDecoderVideoToolbox,
#elif defined(Q_OS_WIN)
    GStreamer::VideoDecoderOptions::ForceVideoDecoderVideoToolbox,
    GStreamer::VideoDecoderOptions::ForceVideoDecoderVulkan,
#elif defined(Q_OS_MACOS)
    GStreamer::VideoDecoderOptions::ForceVideoDecoderDirectX3D,
    GStreamer::VideoDecoderOptions::ForceVideoDecoderVAAPI,
#elif defined(Q_OS_IOS)
    GStreamer::VideoDecoderOptions::ForceVideoDecoderDirectX3D,
    GStreamer::VideoDecoderOptions::ForceVideoDecoderVAAPI,
    GStreamer::VideoDecoderOptions::ForceVideoDecoderNVIDIA,
    GStreamer::VideoDecoderOptions::ForceVideoDecoderIntel,
#endif
    };

    for (const auto &value : removeForceVideoDecodeList) {
        _nameToMetaDataMap[forceVideoDecoderName]->removeEnumInfo(value);
    }
#endif
}
