#include "VideoSettings.h"
#include "VideoManager.h"

#include "QGCLoggingCategory.h"
#include <QtCore/QSettings>
#include <QtCore/QVariantList>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>

QGC_LOGGING_CATEGORY(VideoSettingsLog, "Settings.VideoSettings")

#ifdef QGC_GST_STREAMING
#include "GStreamer.h"
static constexpr bool kGstEnabled = true;
#else
static constexpr bool kGstEnabled = false;
#endif
#ifndef QGC_HEADLESS_CORE
#include "UVCReceiver.h"
#endif

DECLARE_SETTINGGROUP(Video, "Video")
{
    // Setup enum values for videoSource settings into meta data
    QVariantList videoSourceList;
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
#ifdef QGC_HEADLESS_CORE
    QStringList uvcDevices;
#else
    QStringList uvcDevices = UVCReceiver::getDeviceNameList();
#endif
    for (const QString& device : uvcDevices) {
        videoSourceList.append(device);
    }
    if (videoSourceList.count() == 0) {
        _noVideo = true;
        videoSourceList.append(videoSourceNoVideo);
        setUserVisible(false);
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

    // Migrate legacy gpuZeroCopyEnabled (pre-rename) into the new force-CPU semantics.
    {
        QSettings settings;
        settings.beginGroup(settingsGroup);
        const bool hasLegacy = settings.contains(QStringLiteral("gpuZeroCopyEnabled"));
        const bool hasNew    = settings.contains(forceCpuVideoPathName);
        if (hasLegacy) {
            if (!hasNew) {
                const bool gpuZeroCopy = settings.value(QStringLiteral("gpuZeroCopyEnabled")).toBool();
                forceCpuVideoPath()->setRawValue(!gpuZeroCopy);
            }
            settings.remove(QStringLiteral("gpuZeroCopyEnabled"));
        }
        settings.endGroup();
    }

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

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, multiViewEnabled)
{
    if (!_multiViewEnabledFact) {
        _multiViewEnabledFact = _createSettingsFact(multiViewEnabledName);
        connect(_multiViewEnabledFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _multiViewEnabledFact;
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

bool VideoSettings::_sourceNeedsUrl(const QString &source)
{
    static const QStringList urlSources = {
        videoSourceUDPH264, videoSourceUDPH265, videoSourceMPEGTS,
        videoSourceRTSP, videoSourceTCP, videoSourceWebRTC,
    };
    return urlSources.contains(source);
}

bool VideoSettings::sourceConfigured(int index)
{
    const QString source = videoSourceNameAt(index);
    return !_sourceNeedsUrl(source) || !videoUrlAt(index).isEmpty();
}

bool VideoSettings::sourceEnabled(int index)
{
    return videoSourceNameAt(index) != QString::fromUtf8(videoDisabled);
}

QList<int> VideoSettings::switchableIndices()
{
    QList<int> indices{0};
    const QJsonArray extras = _extraSourcesArray();
    for (int i = 0; i < extras.size(); ++i) {
        if (_isStreamSource(extras.at(i).toObject().value(QStringLiteral("source")).toString()) && sourceConfigured(i + 1)) {
            indices.append(i + 1);
        }
    }
    return indices;
}

QList<int> VideoSettings::tileCameraIndices()
{
    QList<int> tiles = switchableIndices();
    tiles.removeAll(currentIndex());
    return tiles;
}

int VideoSettings::currentIndex()
{
    const int index = activeVideoSource()->rawValue().toInt();
    if ((index <= 0) || (index >= videoSourceCount())) {
        return 0;
    }
    return sourceConfigured(index) ? index : 0;
}

QString VideoSettings::currentVideoSourceName()
{
    return videoSourceNameAt(currentIndex());
}

QString VideoSettings::currentVideoUrl()
{
    return videoUrlAt(currentIndex());
}

QString VideoSettings::videoSourceNameAt(int index)
{
    if (index <= 0) {
        return videoSource()->rawValue().toString();
    }
    const QJsonArray extras = _extraSourcesArray();
    if ((index - 1) >= extras.size()) {
        return videoSource()->rawValue().toString();
    }
    return extras.at(index - 1).toObject().value(QStringLiteral("source")).toString();
}

QString VideoSettings::cameraName(int index)
{
    if (index <= 0) {
        return primaryCameraName()->rawValue().toString();
    }
    const QJsonArray extras = _extraSourcesArray();
    if ((index - 1) >= extras.size()) {
        return QString();
    }
    return extras.at(index - 1).toObject().value(QStringLiteral("name")).toString();
}

QString VideoSettings::videoUrlAt(int index)
{
    if (index <= 0) {
        const QString source = videoSource()->rawValue().toString();
        if (source == videoSourceUDPH264 || source == videoSourceUDPH265 || source == videoSourceMPEGTS) {
            return udpUrl()->rawValue().toString().trimmed();
        }
        if (source == videoSourceRTSP) {
            return rtspUrl()->rawValue().toString().trimmed();
        }
        if (source == videoSourceTCP) {
            return tcpUrl()->rawValue().toString().trimmed();
        }
        if (source == videoSourceWebRTC) {
            return whepUrl()->rawValue().toString().trimmed();
        }
        return QString();
    }
    const QJsonArray extras = _extraSourcesArray();
    if ((index - 1) >= extras.size()) {
        return QString();
    }
    return extras.at(index - 1).toObject().value(QStringLiteral("url")).toString().trimmed();
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, forceVideoDecoder)
{
    if (!_forceVideoDecoderFact) {
        _forceVideoDecoderFact = _createSettingsFact(forceVideoDecoderName);

        _forceVideoDecoderFact->setUserVisible(kGstEnabled);

        connect(_forceVideoDecoderFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _forceVideoDecoderFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, lowLatencyMode)
{
    if (!_lowLatencyModeFact) {
        _lowLatencyModeFact = _createSettingsFact(lowLatencyModeName);

        _lowLatencyModeFact->setUserVisible(kGstEnabled);

        connect(_lowLatencyModeFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _lowLatencyModeFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, rtpJitterLatencyMs)
{
    if (!_rtpJitterLatencyMsFact) {
        _rtpJitterLatencyMsFact = _createSettingsFact(rtpJitterLatencyMsName);
        _rtpJitterLatencyMsFact->setUserVisible(kGstEnabled);
        connect(_rtpJitterLatencyMsFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _rtpJitterLatencyMsFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, rtspAutoReconnect)
{
    if (!_rtspAutoReconnectFact) {
        _rtspAutoReconnectFact = _createSettingsFact(rtspAutoReconnectName);
        _rtspAutoReconnectFact->setUserVisible(kGstEnabled);
        connect(_rtspAutoReconnectFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _rtspAutoReconnectFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, forceCpuVideoPath)
{
    if (!_forceCpuVideoPathFact) {
        _forceCpuVideoPathFact = _createSettingsFact(forceCpuVideoPathName);

#if defined(QGC_HAS_ANY_GPU_PATH)
        _forceCpuVideoPathFact->setUserVisible(kGstEnabled);
#else
        _forceCpuVideoPathFact->setUserVisible(false);
#endif
    }
    return _forceCpuVideoPathFact;
}

// videoConversionElement / disablePixelAspectRatio are read by VideoBackend::createSink()
// into a VideoSinkConfig and passed as construct-only bin properties — no env-var indirection.
DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, videoConversionElement)
{
    if (!_videoConversionElementFact) {
        _videoConversionElementFact = _createSettingsFact(videoConversionElementName);
        _videoConversionElementFact->setUserVisible(kGstEnabled);
    }
    return _videoConversionElementFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, disablePixelAspectRatio)
{
    if (!_disablePixelAspectRatioFact) {
        _disablePixelAspectRatioFact = _createSettingsFact(disablePixelAspectRatioName);
        _disablePixelAspectRatioFact->setUserVisible(kGstEnabled);
    }
    return _disablePixelAspectRatioFact;
}


DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, rtspTimeout)
{
    if (!_rtspTimeoutFact) {
        _rtspTimeoutFact = _createSettingsFact(rtspTimeoutName);

        _rtspTimeoutFact->setUserVisible(kGstEnabled);

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
        qCDebug(VideoSettingsLog) << "Stream auto configured";
        return true;
    }
    //-- Check if it's disabled (evaluate whichever source is currently active)
    QString vSource = currentVideoSourceName();
    if(vSource == videoSourceNoVideo || vSource == videoDisabled) {
        return false;
    }
    //-- Stream sources that require a URL are configured once that URL is set
    if (_sourceNeedsUrl(vSource)) {
        return !currentVideoUrl().isEmpty();
    }
    //-- If Herelink Air unit, good to go
    if(vSource == videoSourceHerelinkAirUnit) {
        qCDebug(VideoSettingsLog) << "Stream configured for Herelink Air Unit";
        return true;
    }
    //-- If Herelink Hotspot, good to go
    if(vSource == videoSourceHerelinkHotspot) {
        qCDebug(VideoSettingsLog) << "Stream configured for Herelink Hotspot";
        return true;
    }
#ifndef QGC_HEADLESS_CORE
    if (UVCReceiver::enabled() && UVCReceiver::deviceExists(vSource)) {
        qCDebug(VideoSettingsLog) << "Stream configured for UVC";
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

void VideoSettings::pruneUnavailableDecoders()
{
#ifdef QGC_GST_STREAMING
    static const QList<GStreamer::VideoDecoderOptions> hardwareFamilies{
        GStreamer::VideoDecoderOptions::ForceVideoDecoderNVIDIA,
        GStreamer::VideoDecoderOptions::ForceVideoDecoderVAAPI,
        GStreamer::VideoDecoderOptions::ForceVideoDecoderDirectX3D,
        GStreamer::VideoDecoderOptions::ForceVideoDecoderVideoToolbox,
        GStreamer::VideoDecoderOptions::ForceVideoDecoderIntel,
        GStreamer::VideoDecoderOptions::ForceVideoDecoderVulkan,
    };

    const QList<GStreamer::VideoDecoderOptions> available = GStreamer::availableDecoderFamilies();
    const auto metaIt = _nameToMetaDataMap.constFind(forceVideoDecoderName);
    if (metaIt == _nameToMetaDataMap.constEnd() || !metaIt.value()) {
        return;
    }
    FactMetaData* const metaData = metaIt.value();
    bool pruned = false;
    for (const auto family : hardwareFamilies) {
        // removeEnumInfo() qWarns on an absent value, so skip families not in the enum; values are
        // stored as QVariant(int), so match that representation.
        const QVariant familyValue = static_cast<int>(family);
        if (!available.contains(family) && metaData->enumValues().contains(familyValue)) {
            metaData->removeEnumInfo(familyValue);
            pruned = true;
        }
    }

    Fact* const fact = forceVideoDecoder();
    if (pruned) {
        // Backend init is async — refresh any live FactComboBox bound to this fact.
        emit fact->enumsChanged();
    }
    if (!metaData->enumValues().contains(fact->rawValue())) {
        fact->setRawValue(GStreamer::VideoDecoderOptions::ForceVideoDecoderDefault);
    }
#endif
}
