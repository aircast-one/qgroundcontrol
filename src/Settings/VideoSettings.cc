#include "VideoSettings.h"

#include "QGCLoggingCategory.h"
#include <QtCore/QSettings>
#include <QtCore/QVariantList>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonParseError>
#include <QtCore/QJsonObject>
#include <QtCore/QRegularExpression>
#include <QtCore/QSignalBlocker>
#include <QtCore/QUrl>

#include <algorithm>
#include <ranges>

QGC_LOGGING_CATEGORY(VideoSettingsLog, "Settings.VideoSettings")

namespace {

constexpr const char *kUnreadable     = QT_TRANSLATE_NOOP("VideoSettings", "The camera list is not a readable list, so its cameras cannot be shown. Changing it now would replace it.");
constexpr const char *kNoSuchCamera   = QT_TRANSLATE_NOOP("VideoSettings", "There is no camera at that position.");
constexpr const char *kNeedsKind      = QT_TRANSLATE_NOOP("VideoSettings", "Pick the kind of stream this camera sends.");
constexpr const char *kUnplayable     = QT_TRANSLATE_NOOP("VideoSettings", "This kind of camera cannot show video in this app.");
constexpr const char *kNeedsAddress   = QT_TRANSLATE_NOOP("VideoSettings", "This kind of stream needs an address.");
constexpr const char *kRtspScheme     = QT_TRANSLATE_NOOP("VideoSettings", "An RTSP address starts with rtsp://.");
constexpr const char *kWhepScheme     = QT_TRANSLATE_NOOP("VideoSettings", "A WebRTC address starts with http:// or https://.");
constexpr const char *kDoubledScheme  = QT_TRANSLATE_NOOP("VideoSettings", "Leave the scheme off. The app adds it, and a doubled one fails to resolve.");

void announce(Fact *fact)
{
    const QVariant raw = fact->rawValue();
    emit fact->valueChanged(fact->cookedValue());
    emit fact->containerRawValueChanged(raw);
    emit fact->rawValueChanged(raw);
}

}

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

std::optional<QJsonArray> VideoSettings::cameraList()
{
    const QByteArray text = cameras()->rawValue().toString().trimmed().toUtf8();
    if (text.isEmpty()) {
        return QJsonArray{};
    }
    QJsonParseError error;
    const QJsonDocument parsed = QJsonDocument::fromJson(text, &error);
    if ((error.error != QJsonParseError::NoError) || !parsed.isArray()) {
        return std::nullopt;
    }
    return parsed.array();
}

bool VideoSettings::camerasReadable()
{
    return cameraList().has_value();
}

QJsonArray VideoSettings::_allCameras()
{
    QJsonArray all = cameraList().value_or(QJsonArray{});
    for (const QJsonValue &entry : std::as_const(_droneCameras)) {
        all.append(entry);
    }
    return all;
}

QJsonObject VideoSettings::_cameraAt(int index)
{
    const QJsonArray list = _allCameras();
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

int VideoSettings::storedCameraCount()
{
    return cameraList().value_or(QJsonArray{}).size();
}

bool VideoSettings::cameraFromDrone(int index)
{
    return (index >= storedCameraCount()) && (index < videoSourceCount());
}

bool VideoSettings::setDroneCameras(const QJsonArray &drone)
{
    if (drone == _droneCameras) {
        return false;
    }
    _droneCameras = drone;
    emit streamConfiguredChanged(streamConfigured());
    return true;
}

int VideoSettings::activeAfterAdd(int active, int stored)
{
    return activeAfterAppend(active, stored, 1);
}

int VideoSettings::activeAfterAppend(int active, int stored, int appended)
{
    if (stored == 0) {
        return 0;
    }
    return (active >= stored) ? active + appended : active;
}

int VideoSettings::activeAfterRemoval(int active, int removed)
{
    if (active == removed) {
        return 0;
    }
    return (active > removed) ? active - 1 : active;
}

int VideoSettings::activeAfterMove(int active, int from, int to)
{
    if (active == from) {
        return to;
    }
    if ((from < to) && (active > from) && (active <= to)) {
        return active - 1;
    }
    if ((to < from) && (active >= to) && (active < from)) {
        return active + 1;
    }
    return active;
}

void VideoSettings::storeCameras(const QJsonArray &list, int active)
{
    const QString text = QString::fromUtf8(QJsonDocument(list).toJson(QJsonDocument::Compact));
    const bool listChanged = cameras()->rawValue().toString() != text;
    const bool activeChanged = activeVideoSource()->rawValue().toInt() != active;
    {
        const QSignalBlocker holdList(cameras());
        const QSignalBlocker holdActive(activeVideoSource());
        if (listChanged) {
            cameras()->setRawValue(text);
        }
        if (activeChanged) {
            activeVideoSource()->setRawValue(active);
        }
    }
    if (listChanged) {
        announce(cameras());
    }
    if (activeChanged) {
        announce(activeVideoSource());
    }
}

void VideoSettings::adoptCamera(const QString &title, const QString &source, const QString &url)
{
    std::optional<QJsonArray> list = cameraList();
    if (!list) {
        return;
    }
    const QJsonObject adopted = camera(title, source, url);
    const QString adoptedName = adopted.value(QStringLiteral("name")).toString();
    const auto named = std::find_if(list->begin(), list->end(), [&adoptedName, &adopted](const QJsonValue &entry) {
        const QJsonObject existing = entry.toObject();
        return adoptedName.isEmpty()
            ? (existing.value(QStringLiteral("source")) == adopted.value(QStringLiteral("source"))) && (existing.value(QStringLiteral("url")) == adopted.value(QStringLiteral("url")))
            : (existing.value(QStringLiteral("name")).toString() == adoptedName);
    });
    const int at = static_cast<int>(std::distance(list->begin(), named));
    if (at < list->size()) {
        list->replace(at, adopted);
    } else {
        list->append(adopted);
    }
    storeCameras(*list, at);
}

void VideoSettings::adoptDeviceCameras(const QString &host, const QJsonArray &device)
{
    const std::optional<QJsonArray> listed = cameraList();
    if (device.isEmpty() || !listed) {
        return;
    }
    const QString bare = urlHost(host);
    QJsonArray list;
    for (const QJsonValue &entry : *listed) {
        if (urlHost(entry.toObject().value(QStringLiteral("url")).toString()) != bare) {
            list.append(entry);
        }
    }
    const int first = list.size();
    for (const QJsonValue &entry : device) {
        list.append(entry);
    }
    storeCameras(list, first);
}

QString VideoSettings::addCamera(const QString &title, const QString &source, const QString &url)
{
    std::optional<QJsonArray> list = cameraList();
    if (!list) {
        return tr(kUnreadable);
    }
    const QString kind = source.trimmed();
    const QString address = normalizedUrl(kind, url.trimmed());
    const QString refusal = problem(kind, address);
    if (!refusal.isEmpty()) {
        return refusal;
    }
    const int at = list->size();
    list->append(camera(title, kind, address));
    storeCameras(*list, activeAfterAdd(activeVideoSource()->rawValue().toInt(), at));
    return QString();
}

QString VideoSettings::updateCamera(int index, const QString &title, const QString &source, const QString &url)
{
    std::optional<QJsonArray> list = cameraList();
    if (!list) {
        return tr(kUnreadable);
    }
    if ((index < 0) || (index >= list->size())) {
        return tr(kNoSuchCamera);
    }
    const QString kind = source.trimmed();
    const QString address = normalizedUrl(kind, url.trimmed());
    const QString refusal = problem(kind, address);
    if (!refusal.isEmpty()) {
        return refusal;
    }
    list->replace(index, camera(title, kind, address));
    storeCameras(*list, activeVideoSource()->rawValue().toInt());
    return QString();
}

QString VideoSettings::problem(const QString &source, const QString &url)
{
    if (!offeredSource(source)) {
        const bool known = !source.isEmpty()
            && (source != QString::fromUtf8(videoDisabled))
            && (source != QString::fromUtf8(videoSourceNoVideo))
            && videoSource()->enumValues().contains(source);
        return known ? tr(kUnplayable) : tr(kNeedsKind);
    }
    if (_sourceNeedsUrl(source) && url.isEmpty()) {
        return tr(kNeedsAddress);
    }
    if ((source == QString::fromUtf8(videoSourceRTSP)) && _schemeOf(source, url).isEmpty()) {
        return tr(kRtspScheme);
    }
    if ((source == QString::fromUtf8(videoSourceWebRTC)) && url.contains(QStringLiteral("://")) && _schemeOf(source, url).isEmpty()) {
        return tr(kWhepScheme);
    }
    if (url.contains(QStringLiteral("://")) && _schemeAdded(source)) {
        return tr(kDoubledScheme);
    }
    return QString();
}

QStringList VideoSettings::_schemes(const QString &source)
{
    if (source == QString::fromUtf8(videoSourceRTSP)) {
        return {QStringLiteral("rtsp://"), QStringLiteral("rtsps://")};
    }
    if (source == QString::fromUtf8(videoSourceWebRTC)) {
        return {QStringLiteral("http://"), QStringLiteral("https://")};
    }
    if (source == QString::fromUtf8(videoSourceUDPH264)) {
        return {QStringLiteral("udp://")};
    }
    if (source == QString::fromUtf8(videoSourceUDPH265)) {
        return {QStringLiteral("udp265://"), QStringLiteral("udp://")};
    }
    if (source == QString::fromUtf8(videoSourceMPEGTS)) {
        return {QStringLiteral("mpegts://"), QStringLiteral("udp://")};
    }
    if (source == QString::fromUtf8(videoSourceTCP)) {
        return {QStringLiteral("tcp://")};
    }
    return {};
}

QString VideoSettings::_schemeOf(const QString &source, const QString &url)
{
    const QStringList schemes = _schemes(source);
    const auto found = std::find_if(schemes.cbegin(), schemes.cend(), [&url](const QString &scheme) {
        return url.startsWith(scheme, Qt::CaseInsensitive);
    });
    return (found == schemes.cend()) ? QString() : *found;
}

bool VideoSettings::_schemeAdded(const QString &source)
{
    return (source == QString::fromUtf8(videoSourceUDPH264)) || (source == QString::fromUtf8(videoSourceUDPH265))
        || (source == QString::fromUtf8(videoSourceMPEGTS)) || (source == QString::fromUtf8(videoSourceTCP));
}

QString VideoSettings::normalizedUrl(const QString &source, const QString &url)
{
    const QString scheme = _schemeOf(source, url);
    if (scheme.isEmpty()) {
        return url;
    }
    return _schemeAdded(source) ? url.mid(scheme.size()) : (scheme + url.mid(scheme.size()));
}

QString VideoSettings::urlHost(const QString &url)
{
    const qsizetype schemeEnd = url.indexOf(QStringLiteral("://"));
    const QString rest = (schemeEnd < 0) ? url : url.mid(schemeEnd + 3);
    const qsizetype pathStart = rest.indexOf(QRegularExpression(QStringLiteral("[/?]")));
    const QString authority = (pathStart < 0) ? rest : rest.left(pathStart);
    const QString hostPort = authority.section(QLatin1Char('@'), -1);
    const qsizetype colon = hostPort.lastIndexOf(QLatin1Char(':'));
    return (colon < 0) ? hostPort : hostPort.left(colon);
}

void VideoSettings::removeCamera(int index)
{
    std::optional<QJsonArray> list = cameraList();
    if (!list || (index < 0) || (index >= list->size())) {
        return;
    }
    list->removeAt(index);
    storeCameras(*list, activeAfterRemoval(activeVideoSource()->rawValue().toInt(), index));
}

void VideoSettings::moveCamera(int from, int to)
{
    std::optional<QJsonArray> list = cameraList();
    if (!list || (from < 0) || (from >= list->size()) || (to < 0) || (to >= list->size())) {
        return;
    }
    const QJsonValue moving = list->takeAt(from);
    list->insert(to, moving);
    storeCameras(*list, activeAfterMove(activeVideoSource()->rawValue().toInt(), from, to));
}

int VideoSettings::videoSourceCount()
{
    return _allCameras().size();
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
    if ((index < 0) || (index >= videoSourceCount())) {
        return false;
    }
    const QString source = videoSourceNameAt(index);
    return _sourceNeedsUrl(source) ? !videoUrlAt(index).isEmpty() : offeredSource(source);
}

bool VideoSettings::offeredSource(const QString &source)
{
    if (_sourceNeedsUrl(source)) {
        return true;
    }
    if ((source == QString::fromUtf8(videoSourceHerelinkAirUnit)) || (source == QString::fromUtf8(videoSourceHerelinkHotspot))) {
        return true;
    }
#ifndef QGC_HEADLESS_CORE
    return UVCReceiver::enabled() && UVCReceiver::deviceExists(source);
#else
    return false;
#endif
}

QStringList VideoSettings::offeredSources()
{
    QStringList offered;
    const QVariantList values = videoSource()->enumValues();
    for (const QVariant &value : values) {
        if (offeredSource(value.toString())) {
            offered.append(value.toString());
        }
    }
    return offered;
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

int VideoSettings::_nextAfterCurrent(const std::function<bool(int)> &eligible)
{
    const int count = videoSourceCount();
    const int current = currentIndex();
    const auto steps = std::views::iota(1, std::max(count, 1));
    const auto found = std::ranges::find_if(steps, [&eligible, count, current](int step) { return eligible((current + step) % count); });
    return (found == steps.end()) ? -1 : (current + *found) % count;
}

int VideoSettings::nextUsableIndex()
{
    return _nextAfterCurrent([this](int index) { return sourceUsable(index); });
}

int VideoSettings::pipCameraIndex()
{
    const QString main = streamUri(currentVideoSourceName(), currentVideoUrl());
    return _nextAfterCurrent([this, &main](int index) {
        const QString source = videoSourceNameAt(index);
        return sourceUsable(index) && pipCapable(source) && !sameStream(streamUri(source, videoUrlAt(index)), main);
    });
}

bool VideoSettings::pipCapable(const QString &source)
{
    return _sourceNeedsUrl(source) || (source == QString::fromUtf8(videoSourceHerelinkAirUnit)) || (source == QString::fromUtf8(videoSourceHerelinkHotspot));
}

QString VideoSettings::streamUri(const QString &source, const QString &url)
{
    if (source == QString::fromUtf8(videoSourceUDPH264)) {
        return QStringLiteral("udp://%1").arg(url);
    }
    if (source == QString::fromUtf8(videoSourceUDPH265)) {
        return QStringLiteral("udp265://%1").arg(url);
    }
    if (source == QString::fromUtf8(videoSourceMPEGTS)) {
        return QStringLiteral("mpegts://%1").arg(url);
    }
    if (source == QString::fromUtf8(videoSourceRTSP)) {
        return url;
    }
    if (source == QString::fromUtf8(videoSourceTCP)) {
        return QStringLiteral("tcp://%1").arg(url);
    }
    if (source == QString::fromUtf8(videoSourceWebRTC)) {
        const QString whepInput = url.trimmed();
        return whepInput.isEmpty() ? QString() : QUrl::fromUserInput(whepInput).toString();
    }
    if (source == QString::fromUtf8(videoSource3DRSolo)) {
        return QStringLiteral("udp://0.0.0.0:5600");
    }
    if (source == QString::fromUtf8(videoSourceParrotDiscovery)) {
        return QStringLiteral("udp://0.0.0.0:8888");
    }
    if (source == QString::fromUtf8(videoSourceYuneecMantisG)) {
        return QStringLiteral("rtsp://192.168.42.1:554/live");
    }
    if (source == QString::fromUtf8(videoSourceHerelinkAirUnit)) {
        return QStringLiteral("rtsp://192.168.0.10:8554/H264Video");
    }
    if (source == QString::fromUtf8(videoSourceHerelinkHotspot)) {
        return QStringLiteral("rtsp://192.168.43.1:8554/fpv_stream");
    }
    return QString();
}

QString VideoSettings::_streamIdentity(const QString &uri)
{
    const QString trimmed = uri.trimmed();
    const qsizetype schemeEnd = trimmed.indexOf(QStringLiteral("://"));
    if (schemeEnd < 0) {
        return trimmed;
    }
    const QString rest = trimmed.mid(schemeEnd + 3);
    const qsizetype pathStart = rest.indexOf(QRegularExpression(QStringLiteral("[/?]")));
    const QString authority = (pathStart < 0) ? rest : rest.left(pathStart);
    const QString path = (pathStart < 0) ? QString() : rest.mid(pathStart);
    return trimmed.left(schemeEnd).toLower() + QStringLiteral("://") + authority.section(QLatin1Char('@'), -1).toLower()
        + QString(path).remove(QRegularExpression(QStringLiteral("/+$")));
}

std::optional<QString> VideoSettings::_listenPort(const QString &uri)
{
    static const QStringList listening = {QStringLiteral("udp"), QStringLiteral("udp265"), QStringLiteral("mpegts")};
    const qsizetype schemeEnd = uri.indexOf(QStringLiteral("://"));
    if ((schemeEnd < 0) || !listening.contains(uri.left(schemeEnd))) {
        return std::nullopt;
    }
    const QString authority = uri.mid(schemeEnd + 3).section(QRegularExpression(QStringLiteral("[/?]")), 0, 0);
    const qsizetype colon = authority.lastIndexOf(QLatin1Char(':'));
    return (colon < 0) ? std::nullopt : std::optional<QString>(authority.mid(colon + 1));
}

bool VideoSettings::sameStream(const QString &a, const QString &b)
{
    const std::optional<QString> port = _listenPort(a);
    return (_streamIdentity(a) == _streamIdentity(b)) || (port && (port == _listenPort(b)));
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

QString VideoSettings::storedActiveSourceName()
{
    const int active = activeVideoSource()->rawValue().toInt();
    return (active < storedCameraCount()) ? videoSourceNameAt(active) : QString();
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
    return sourceUsable(currentIndex());
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
