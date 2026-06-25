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
    _nameToMetaDataMap[videoSource2Name]->setEnumInfo(videoSourceCookedList, videoSourceList);

    _setForceVideoDecodeList();

    // Set default value for videoSource
    _setDefaults();
}

void VideoSettings::_setDefaults()
{
    if (_noVideo) {
        _nameToMetaDataMap[videoSourceName]->setRawDefaultValue(videoSourceNoVideo);
        _nameToMetaDataMap[videoSource2Name]->setRawDefaultValue(videoSourceNoVideo);
    } else {
        _nameToMetaDataMap[videoSourceName]->setRawDefaultValue(videoDisabled);
        _nameToMetaDataMap[videoSource2Name]->setRawDefaultValue(videoDisabled);
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

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, videoSource2)
{
    if (!_videoSource2Fact) {
        _videoSource2Fact = _createSettingsFact(videoSource2Name);
        //-- Check for sources no longer available
        if(!_videoSource2Fact->enumValues().contains(_videoSource2Fact->rawValue().toString())) {
            if (_noVideo) {
                _videoSource2Fact->setRawValue(videoSourceNoVideo);
            } else {
                _videoSource2Fact->setRawValue(videoDisabled);
            }
        }
        connect(_videoSource2Fact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _videoSource2Fact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, udpUrl2)
{
    if (!_udpUrl2Fact) {
        _udpUrl2Fact = _createSettingsFact(udpUrl2Name);
        connect(_udpUrl2Fact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _udpUrl2Fact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, tcpUrl2)
{
    if (!_tcpUrl2Fact) {
        _tcpUrl2Fact = _createSettingsFact(tcpUrl2Name);
        connect(_tcpUrl2Fact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _tcpUrl2Fact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, rtspUrl2)
{
    if (!_rtspUrl2Fact) {
        _rtspUrl2Fact = _createSettingsFact(rtspUrl2Name);
        connect(_rtspUrl2Fact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _rtspUrl2Fact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, activeVideoSource)
{
    if (!_activeVideoSourceFact) {
        _activeVideoSourceFact = _createSettingsFact(activeVideoSourceName);
        connect(_activeVideoSourceFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _activeVideoSourceFact;
}

Fact *VideoSettings::currentVideoSource()
{
    return (activeVideoSource()->rawValue().toInt() == 1) ? videoSource2() : videoSource();
}

Fact *VideoSettings::currentUdpUrl()
{
    return (activeVideoSource()->rawValue().toInt() == 1) ? udpUrl2() : udpUrl();
}

Fact *VideoSettings::currentRtspUrl()
{
    return (activeVideoSource()->rawValue().toInt() == 1) ? rtspUrl2() : rtspUrl();
}

Fact *VideoSettings::currentTcpUrl()
{
    return (activeVideoSource()->rawValue().toInt() == 1) ? tcpUrl2() : tcpUrl();
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

bool VideoSettings::streamConfigured(void)
{
    //-- First, check if it's autoconfigured
    if(VideoManager::instance()->autoStreamConfigured()) {
        qCDebug(VideoManagerLog) << "Stream auto configured";
        return true;
    }
    //-- Check if it's disabled (evaluate whichever source is currently active)
    QString vSource = currentVideoSource()->rawValue().toString();
    if(vSource == videoSourceNoVideo || vSource == videoDisabled) {
        return false;
    }
    //-- If UDP, check for URL
    if(vSource == videoSourceUDPH264 || vSource == videoSourceUDPH265) {
        qCDebug(VideoManagerLog) << "Testing configuration for UDP Stream:" << currentUdpUrl()->rawValue().toString();
        return !currentUdpUrl()->rawValue().toString().isEmpty();
    }
    //-- If RTSP, check for URL
    if(vSource == videoSourceRTSP) {
        qCDebug(VideoManagerLog) << "Testing configuration for RTSP Stream:" << currentRtspUrl()->rawValue().toString();
        return !currentRtspUrl()->rawValue().toString().isEmpty();
    }
    //-- If TCP, check for URL
    if(vSource == videoSourceTCP) {
        qCDebug(VideoManagerLog) << "Testing configuration for TCP Stream:" << currentTcpUrl()->rawValue().toString();
        return !currentTcpUrl()->rawValue().toString().isEmpty();
    }
    //-- If MPEG-TS, check for URL
    if(vSource == videoSourceMPEGTS) {
        qCDebug(VideoManagerLog) << "Testing configuration for MPEG-TS Stream:" << currentUdpUrl()->rawValue().toString();
        return !currentUdpUrl()->rawValue().toString().isEmpty();
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
