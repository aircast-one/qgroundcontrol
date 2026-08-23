#include "PacketRadioManager.h"

#include "Fact.h"
#include "QGCLoggingCategory.h"
#include "PacketRadioSettings.h"
#include "SettingsManager.h"
#include "VideoSettings.h"

#ifdef emit
#undef emit
#endif
#include "gui_interface.h"
#include "wfbng_link.h"

#include <QtCore/QByteArray>
#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QStandardPaths>

#include <algorithm>

QGC_LOGGING_CATEGORY(PacketRadioManagerLog, "qgc.packetradio.manager")

namespace {
constexpr int kVideoPort = 5600;
constexpr int kRetryIntervalMs = 3000;
constexpr int kPollIntervalMs = 1000;
constexpr auto kDefaultGsKeyHex =
    "bbb7ed6e83a46a8a9b8a12a0f98ece2bdc978705b8204701b2085fa28cac7b46"
    "0e05c48a6195fb70921c747a66e83c02e640bd6bbeb5b251537a98a27416a263";
}

Q_GLOBAL_STATIC(PacketRadioManager, _packetRadioManagerInstance)

PacketRadioManager *PacketRadioManager::instance()
{
    return _packetRadioManagerInstance();
}

const DeviceId *PacketRadioManager::selectAdapter(const std::vector<DeviceId> &devices, const std::string &savedName)
{
    const auto saved = std::find_if(devices.begin(), devices.end(), [&savedName](const DeviceId &device) {
        return device.known_adapter && device.matches_saved_name(savedName);
    });
    if (saved != devices.end()) {
        return &(*saved);
    }

    const auto first = std::find_if(devices.begin(), devices.end(), [](const DeviceId &device) {
        return device.known_adapter;
    });
    return (first != devices.end()) ? &(*first) : nullptr;
}

PacketRadioManager::PacketRadioManager(QObject *parent)
    : QObject(parent)
{
    _retryTimer.setInterval(kRetryIntervalMs);
    connect(&_retryTimer, &QTimer::timeout, this, &PacketRadioManager::_tryStart);

    _pollTimer.setInterval(kPollIntervalMs);
    connect(&_pollTimer, &QTimer::timeout, this, &PacketRadioManager::_poll);
}

PacketRadioManager::~PacketRadioManager()
{
    if (_callbacksInstalled) {
        GuiInterface &gui = GuiInterface::Instance();
        gui.onLog = nullptr;
        gui.onStats = nullptr;
        gui.onRtpStream = nullptr;
        gui.onWifiStopped = nullptr;
    }
    _releaseLink();
}

void PacketRadioManager::init()
{
    PacketRadioSettings *settings = SettingsManager::instance()->packetRadioSettings();

    connect(settings->enabled(), &Fact::rawValueChanged, this, [this](QVariant) { _evaluate(); });

    const auto restart = [this](QVariant) {
        if (_running || (_status != Disabled)) {
            _stop();
            _evaluate();
        }
    };
    connect(settings->channel(), &Fact::rawValueChanged, this, restart);
    connect(settings->channelWidth(), &Fact::rawValueChanged, this, restart);
    connect(settings->keyFile(), &Fact::rawValueChanged, this, restart);
    connect(settings->deviceName(), &Fact::rawValueChanged, this, restart);

    const auto adaptive = [this](QVariant) { _applyAdaptiveLink(); };
    connect(settings->alinkEnabled(), &Fact::rawValueChanged, this, adaptive);
    connect(settings->alinkTxPower(), &Fact::rawValueChanged, this, adaptive);

    connect(qApp, &QCoreApplication::aboutToQuit, this, [this]() { _stop(); });

    GuiInterface &gui = GuiInterface::Instance();
    _callbacksInstalled = true;
    gui.playerPort = kVideoPort;
    gui.onLog = [this](LogLevel level, std::string msg) {
        QMetaObject::invokeMethod(this, [level, text = QString::fromStdString(msg)]() {
            if ((level == LogLevel::Error) || (level == LogLevel::Warn)) {
                qCWarning(PacketRadioManagerLog) << text;
            } else {
                qCDebug(PacketRadioManagerLog) << text;
            }
        }, Qt::QueuedConnection);
    };
    gui.onStats = [this](long long, long long, long long rtp) {
        QMetaObject::invokeMethod(this, [this, rtp]() {
            _rtpPackets = rtp;
        }, Qt::QueuedConnection);
    };
    gui.onRtpStream = [this](int, std::string codec) {
        QMetaObject::invokeMethod(this, [this, name = QString::fromStdString(codec)]() {
            _applyVideoSettings(name);
        }, Qt::QueuedConnection);
    };
    gui.onWifiStopped = [this]() {
        QMetaObject::invokeMethod(this, [this]() {
            _releaseLink();
            _evaluate();
        }, Qt::QueuedConnection);
    };

    refreshAdapters();
    _evaluate();
}

QString PacketRadioManager::statusText() const
{
    switch (_status) {
    case Disabled:
        return tr("Off");
    case NoAdapter:
        return tr("No supported Wi-Fi adapter found");
    case AdapterUnavailable:
        return tr("%1 found but cannot be opened — is another app using it?").arg(_adapterName);
    case InvalidKey:
        return tr("Key file cannot be read — check the path in Radio settings");
    case Listening:
        return tr("Listening on %1 — no video yet").arg(_adapterName);
    case Receiving:
        return tr("Receiving on %1").arg(_adapterName);
    }
    return QString();
}

void PacketRadioManager::_updateAdapters(const std::vector<DeviceId> &devices)
{
    QStringList names;
    for (const DeviceId &device : devices) {
        if (device.known_adapter) {
            names.append(QString::fromStdString(device.display_name));
        }
    }
    if (names != _adapters) {
        _adapters = names;
        Q_EMIT adaptersChanged();
    }
}

void PacketRadioManager::refreshAdapters()
{
    _updateAdapters(WfbngLink::get_device_list());
}

void PacketRadioManager::_setStatus(Status status)
{
    if (_status == status) {
        return;
    }
    const bool videoFlapped = linkActive() && ((status == Listening) || (status == Receiving));
    _status = status;

    if (videoFlapped) {
        Q_EMIT statusChanged();
        return;
    }

    PacketRadioSettings *settings = SettingsManager::instance()->packetRadioSettings();
    qCDebug(PacketRadioManagerLog) << "status" << status
                                   << "adapter" << (_adapterName.isEmpty() ? QStringLiteral("none") : _adapterName)
                                   << "channel" << settings->channel()->rawValue().toUInt()
                                   << "width" << settings->channelWidth()->rawValue().toInt()
                                   << "key" << (settings->keyFile()->rawValue().toString().isEmpty()
                                                    ? QStringLiteral("built-in default")
                                                    : settings->keyFile()->rawValue().toString())
                                   << "adapters" << _adapters.size();

    Q_EMIT statusChanged();
}

void PacketRadioManager::_evaluate()
{
    PacketRadioSettings *settings = SettingsManager::instance()->packetRadioSettings();
    if (!settings->enabled()->rawValue().toBool()) {
        _stop();
        return;
    }
    if (_running) {
        return;
    }
    _tryStart();
}

void PacketRadioManager::_tryStart()
{
    if (_running) {
        return;
    }

    const std::vector<DeviceId> devices = WfbngLink::get_device_list();
    _updateAdapters(devices);

    PacketRadioSettings *settings = SettingsManager::instance()->packetRadioSettings();
    const DeviceId *device = selectAdapter(devices, settings->deviceName()->rawValue().toString().toStdString());
    if (!device) {
        _adapterName.clear();
        _setStatus(NoAdapter);
        _retryTimer.start();
        return;
    }

    _adapterName = QString::fromStdString(device->display_name);

    const QString keyPath = _resolveKeyPath();
    if (keyPath.isEmpty()) {
        _setStatus(InvalidKey);
        _retryTimer.stop();
        return;
    }

    const uint8_t channel = static_cast<uint8_t>(settings->channel()->rawValue().toUInt());
    const int channelWidth = settings->channelWidth()->rawValue().toInt();

    _link = std::make_unique<WfbngLink>();
    if (!_link->start(*device, channel, channelWidth, keyPath.toStdString())) {
        _link.reset();
        _setStatus(AdapterUnavailable);
        _retryTimer.start();
        return;
    }

    _running = true;
    _retryTimer.stop();
    _rtpPackets = 0;
    _lastRtpPackets = 0;
    _pollTimer.start();
    _applyAdaptiveLink();
    _setStatus(Listening);
    Q_EMIT statusChanged();
}

void PacketRadioManager::_releaseLink()
{
    _retryTimer.stop();
    _pollTimer.stop();
    if (_link) {
        _link->stop();
        _link.reset();
    }
    _running = false;
}

void PacketRadioManager::_stop()
{
    _releaseLink();
    _adapterName.clear();
    _antennaRssi.clear();
    _antennaSnr.clear();
    _haveSignal = false;
    _linkScore = 0;
    _packetLoss = 0;
    _restoreVideoSettings();
    _setStatus(Disabled);
    Q_EMIT statsChanged();
}

void PacketRadioManager::_poll()
{
    if (!_link) {
        return;
    }

    const std::array<int, ANTENNA_COUNT> rssi = _link->get_rssi_dbm();
    const std::array<int, ANTENNA_COUNT> snr = _link->get_snr_db();
    const std::array<int, ANTENNA_COUNT> scores = _link->get_link_score();

    QVariantList rssiList;
    QVariantList snrList;
    for (int value : rssi) {
        rssiList.append(value);
    }
    for (int value : snr) {
        snrList.append(value);
    }

    _antennaRssi = rssiList;
    _antennaSnr = snrList;
    _haveSignal = std::any_of(rssi.begin(), rssi.end(), [](int value) { return value != 0; });
    _linkScore = *std::max_element(scores.begin(), scores.end());
    _packetLoss = _link->get_packet_loss();

    _setStatus(_rtpPackets > _lastRtpPackets ? Receiving : Listening);
    _lastRtpPackets = _rtpPackets;

    Q_EMIT statsChanged();
}

void PacketRadioManager::_applyAdaptiveLink()
{
    if (!_link) {
        return;
    }
    PacketRadioSettings *settings = SettingsManager::instance()->packetRadioSettings();
    _link->enable_alink(settings->alinkEnabled()->rawValue().toBool());
    _link->set_alink_tx_power(settings->alinkTxPower()->rawValue().toInt());
}

void PacketRadioManager::_applyVideoSettings(const QString &codec)
{
    VideoSettings *video = SettingsManager::instance()->videoSettings();

    if (!_videoOverridden) {
        _savedVideoSource = video->videoSource()->rawValue();
        _savedUdpUrl = video->udpUrl()->rawValue();
        _savedLowLatency = video->lowLatencyMode()->rawValue();
        _videoOverridden = true;
    }

    const QString source = (codec == QStringLiteral("H265")) ? VideoSettings::videoSourceUDPH265
                                                             : VideoSettings::videoSourceUDPH264;
    if (video->videoSource()->rawValue().toString() != source) {
        video->videoSource()->setRawValue(source);
    }
    const QString url = QStringLiteral("0.0.0.0:%1").arg(kVideoPort);
    if (video->udpUrl()->rawValue().toString() != url) {
        video->udpUrl()->setRawValue(url);
    }
    if (!video->lowLatencyMode()->rawValue().toBool()) {
        video->lowLatencyMode()->setRawValue(true);
    }
}

void PacketRadioManager::_restoreVideoSettings()
{
    if (!_videoOverridden) {
        return;
    }
    VideoSettings *video = SettingsManager::instance()->videoSettings();
    video->videoSource()->setRawValue(_savedVideoSource);
    video->udpUrl()->setRawValue(_savedUdpUrl);
    video->lowLatencyMode()->setRawValue(_savedLowLatency);
    _videoOverridden = false;
}

QString PacketRadioManager::_resolveKeyPath()
{
    PacketRadioSettings *settings = SettingsManager::instance()->packetRadioSettings();
    const QString configured = settings->keyFile()->rawValue().toString().trimmed();
    if (!configured.isEmpty()) {
        return QFile::exists(configured) ? configured : QString();
    }

    const QString fallback = QDir(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation))
                                 .filePath(QStringLiteral("openipc-default-gs.key"));
    if (!QFile::exists(fallback)) {
        QDir().mkpath(QFileInfo(fallback).absolutePath());
        QFile file(fallback);
        if (!file.open(QIODevice::WriteOnly)) {
            return QString();
        }
        file.write(QByteArray::fromHex(kDefaultGsKeyHex));
    }
    return fallback;
}
