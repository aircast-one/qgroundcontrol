#include "VideoSettings.h"
#include "VideoManager.h"

#include "QGCLoggingCategory.h"
#include <QtCore/QSettings>
#include <QtCore/QVariantList>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QUrl>

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
    QVariantList videoSourceList;
    videoSourceList.append(videoSourceRTSP);
    videoSourceList.append(videoSourceUDPH264);
    videoSourceList.append(videoSourceUDPH265);
    videoSourceList.append(videoSourceTCP);
    videoSourceList.append(videoSourceMPEGTS);
#ifdef QGC_GST_STREAMING
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
        videoSourceList.append(videoSourceNoVideo);
        setUserVisible(false);
    } else {
        videoSourceList.insert(0, videoDisabled);
    }

    QStringList videoSourceCookedList;
    for (const QVariant& videoSource: videoSourceList) {
        videoSourceCookedList.append( VideoSettings::tr(videoSource.toString().toStdString().c_str()) );
    }

    _nameToMetaDataMap[videoSourceName]->setEnumInfo(videoSourceCookedList, videoSourceList);

    _setForceVideoDecodeList();

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

DECLARE_SETTINGSFACT(VideoSettings, videoSource)

DECLARE_SETTINGSFACT_NO_FUNC(VideoSettings, cameras)
{
    if (!_camerasFact) {
        _camerasFact = _createSettingsFact(camerasName);
        connect(_camerasFact, &Fact::valueChanged, this, &VideoSettings::_configChanged);
    }
    return _camerasFact;
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

QJsonArray VideoSettings::cameraList()
{
    return QJsonDocument::fromJson(cameras()->rawValue().toString().toUtf8()).array();
}

QJsonObject VideoSettings::_cameraAt(int index)
{
    const QJsonArray list = cameraList();
    return (index >= 0 && index < list.size()) ? list.at(index).toObject() : QJsonObject{};
}

QJsonObject VideoSettings::camera(const QString &title, const QString &source, const QString &url)
{
    return QJsonObject{
        {QStringLiteral("name"), title.trimmed()},
        {QStringLiteral("source"), source.trimmed()},
        {QStringLiteral("url"), url.trimmed()},
    };
}

void VideoSettings::storeCameras(const QJsonArray &list, int active)
{
    const QString text = QString::fromUtf8(QJsonDocument(list).toJson(QJsonDocument::Compact));
    if (cameras()->rawValue().toString() != text) {
        cameras()->setRawValue(text);
    }
    if (activeVideoSource()->rawValue().toInt() != active) {
        activeVideoSource()->setRawValue(active);
    }
}

void VideoSettings::adoptCamera(const QString &title, const QString &source, const QString &url)
{
    const QJsonObject adopted = camera(title, source, url);
    QJsonArray list = cameraList();
    int at = list.size();
    for (int i = 0; i < list.size(); ++i) {
        if (!adopted.value(QStringLiteral("name")).toString().isEmpty() && list.at(i).toObject().value(QStringLiteral("name")) == adopted.value(QStringLiteral("name"))) {
            at = i;
            break;
        }
    }
    if (at < list.size()) {
        list.replace(at, adopted);
    } else {
        list.append(adopted);
    }
    storeCameras(list, at);
}

void VideoSettings::adoptDeviceCameras(const QString &host, const QJsonArray &device)
{
    if (device.isEmpty()) {
        return;
    }
    QJsonArray list;
    for (const QJsonValue &entry : cameraList()) {
        if (QUrl(entry.toObject().value(QStringLiteral("url")).toString()).host() != host) {
            list.append(entry);
        }
    }
    const int first = list.size();
    for (const QJsonValue &entry : device) {
        list.append(entry);
    }
    storeCameras(list, first);
}

int VideoSettings::videoSourceCount()
{
    return cameraList().size();
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
    return (index >= 0) && (index < videoSourceCount()) && (!_sourceNeedsUrl(videoSourceNameAt(index)) || !videoUrlAt(index).isEmpty());
}

bool VideoSettings::sourceEnabled(int index)
{
    return videoSourceNameAt(index) != QString::fromUtf8(videoDisabled);
}

bool VideoSettings::sourceUsable(int index)
{
    const QString source = videoSourceNameAt(index);
    return sourceConfigured(index) && (source != QString::fromUtf8(videoDisabled)) && (source != QString::fromUtf8(videoSourceNoVideo)) && !source.isEmpty();
}

QList<int> VideoSettings::switchableIndices()
{
    QList<int> indices;
    const int count = videoSourceCount();
    for (int i = 0; i < count; ++i) {
        if (sourceUsable(i)) {
            indices.append(i);
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
    if (sourceUsable(index)) {
        return index;
    }
    const QList<int> usable = switchableIndices();
    return usable.isEmpty() ? 0 : usable.first();
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
    const QJsonObject entry = _cameraAt(index);
    return entry.isEmpty() ? QString::fromUtf8(videoDisabled) : entry.value(QStringLiteral("source")).toString().trimmed();
}

QString VideoSettings::cameraName(int index)
{
    return _cameraAt(index).value(QStringLiteral("name")).toString().trimmed();
}

QString VideoSettings::videoUrlAt(int index)
{
    return _cameraAt(index).value(QStringLiteral("url")).toString().trimmed();
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

bool VideoSettings::streamConfigured(void)
{
    if(VideoManager::instance()->autoStreamConfigured()) {
        qCDebug(VideoSettingsLog) << "Stream auto configured";
        return true;
    }
    QString vSource = currentVideoSourceName();
    if(vSource == videoSourceNoVideo || vSource == videoDisabled) {
        return false;
    }
    if (_sourceNeedsUrl(vSource)) {
        return !currentVideoUrl().isEmpty();
    }
    if(vSource == videoSourceHerelinkAirUnit) {
        qCDebug(VideoSettingsLog) << "Stream configured for Herelink Air Unit";
        return true;
    }
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
        const QVariant familyValue = static_cast<int>(family);
        if (!available.contains(family) && metaData->enumValues().contains(familyValue)) {
            metaData->removeEnumInfo(familyValue);
            pruned = true;
        }
    }

    Fact* const fact = forceVideoDecoder();
    if (pruned) {
        emit fact->enumsChanged();
    }
    if (!metaData->enumValues().contains(fact->rawValue())) {
        fact->setRawValue(GStreamer::VideoDecoderOptions::ForceVideoDecoderDefault);
    }
#endif
}
