#include "VideoCloudFailover.h"

#include <QtCore/QUrl>
#include <algorithm>

#include "QGCLoggingCategory.h"
#include "VideoReceiver.h"

QGC_LOGGING_CATEGORY(VideoCloudFailoverLog, "Video.CloudFailover")

VideoCloudFailover::VideoCloudFailover(FetchToken fetchToken, Probe probe, Restart restart, Timing timing,
                                       QObject* parent)
    : QObject(parent),
      _fetchToken(std::move(fetchToken)),
      _probe(std::move(probe)),
      _restart(std::move(restart)),
      _timing(timing)
{}

void VideoCloudFailover::setDevice(const Device& device)
{
    _device = device;
    for (const auto& [receiver, state] : _states) {
        _armStall(state.get());
    }
}

QString VideoCloudFailover::cloudUrlFor(const Device& device, const QString& directUri)
{
    if (device.host.isEmpty() || device.sfu.isEmpty() || device.deviceId.isEmpty()) {
        return {};
    }
    const QUrl url(directUri);
    if (url.host() != device.host) {
        return {};
    }
    const QString scheme = url.scheme().toLower();
    const QStringList segments = url.path().split(QLatin1Char('/'), Qt::SkipEmptyParts);
    const bool rtsp = scheme.startsWith(QLatin1String("rtsp")) && segments.size() == 1;
    const bool whep =
        QStringList{QStringLiteral("http"), QStringLiteral("https"), QStringLiteral("whep"), QStringLiteral("wheps")}
            .contains(scheme) &&
        segments.size() == 2 && segments.at(1) == QLatin1String("whep");
    if (!rtsp && !whep) {
        return {};
    }
    QString base = device.sfu;
    while (base.endsWith(QLatin1Char('/'))) {
        base.chop(1);
    }
    return base + QStringLiteral("/api/v1/whep/") + QString::fromUtf8(QUrl::toPercentEncoding(device.deviceId)) +
           QLatin1Char('/') + QString::fromUtf8(QUrl::toPercentEncoding(segments.first()));
}

void VideoCloudFailover::watch(VideoReceiver* receiver)
{
    auto owned = std::make_unique<State>();
    State* const state = owned.get();
    state->receiver = receiver;
    state->stall = new QTimer(this);
    state->stall->setSingleShot(true);
    state->stall->setInterval(_timing.stallMs);
    state->probe = new QTimer(this);
    state->probe->setInterval(_timing.probeMs);
    (void) connect(state->stall, &QTimer::timeout, this, [this, state]() { _stalled(state); });
    (void) connect(state->probe, &QTimer::timeout, this, [this, state]() { _probeDirect(state); });

    (void) connect(receiver, &VideoReceiver::decodingChanged, this, [this, state](bool active) {
        state->decoding = active;
        if (active) {
            state->stall->stop();
        } else {
            _armStall(state);
        }
    });
    (void) connect(receiver, &VideoReceiver::uriChanged, this, [this, state]() {
        if (state->switching) {
            return;
        }
        if (state->onCloud) {
            state->onCloud = false;
            state->probe->stop();
            state->receiver->setAuthToken(QString());
        }
        _armStall(state);
    });
    (void) connect(receiver, &QObject::destroyed, this, [this, receiver]() {
        const auto it = _states.find(receiver);
        if (it != _states.end()) {
            delete it->second->stall;
            delete it->second->probe;
            _states.erase(it);
        }
    });

    _states[receiver] = std::move(owned);
    _armStall(state);
}

bool VideoCloudFailover::onCloud(const VideoReceiver* receiver) const
{
    const auto it = _states.find(receiver);
    return it != _states.end() && it->second->onCloud;
}

void VideoCloudFailover::_armStall(State* state)
{
    if (state->decoding || !state->receiver || state->receiver->uri().isEmpty() || state->stall->isActive()) {
        return;
    }
    if (!state->onCloud && cloudUrlFor(_device, state->receiver->uri()).isEmpty()) {
        return;
    }
    state->stall->start();
}

void VideoCloudFailover::_stalled(State* state)
{
    VideoReceiver* const receiver = state->receiver;
    if (!receiver || state->decoding) {
        return;
    }
    if (state->onCloud) {
        _fetchToken(_device.deviceId, this, [this, state](const QString& token) {
            if (!token.isEmpty() && state->onCloud && state->receiver) {
                state->receiver->setAuthToken(token);
            }
            _armStall(state);
        });
        return;
    }
    const QString direct = receiver->uri();
    const QString url = cloudUrlFor(_device, direct);
    if (url.isEmpty()) {
        return;
    }
    _fetchToken(_device.deviceId, this, [this, state, direct, url](const QString& token) {
        if (token.isEmpty() || !state->receiver || state->decoding || state->onCloud ||
            state->receiver->uri() != direct) {
            _armStall(state);
            return;
        }
        state->direct = direct;
        _useCloud(state, url, token);
    });
}

void VideoCloudFailover::_useCloud(State* state, const QString& url, const QString& token)
{
    state->onCloud = true;
    state->holdMs = _nextHold(*state);
    state->probeOks = 0;
    state->onCloudSince.start();
    qCInfo(VideoCloudFailoverLog) << state->receiver->name() << "can't reach" << state->direct
                                  << "- playing the cloud copy, trying the device again after" << state->holdMs << "ms";
    _setUri(state, url, token);
    state->probe->start();
    _armStall(state);
    emit switched(state->receiver->name(), true);
}

void VideoCloudFailover::_probeDirect(State* state)
{
    if (!state->onCloud || state->probing || state->onCloudSince.elapsed() < state->holdMs) {
        return;
    }
    state->probing = true;
    _probe(_device.host, this, [this, state](bool ok) {
        state->probing = false;
        if (!state->onCloud) {
            return;
        }
        state->probeOks = ok ? state->probeOks + 1 : 0;
        if (state->probeOks >= _timing.probeSuccesses) {
            _useDirect(state);
        }
    });
}

void VideoCloudFailover::_useDirect(State* state)
{
    state->onCloud = false;
    state->probe->stop();
    state->stall->stop();
    state->failedBack.start();
    qCInfo(VideoCloudFailoverLog) << state->receiver->name() << "device answers again - back to" << state->direct;
    _setUri(state, state->direct, QString());
    _armStall(state);
    emit switched(state->receiver->name(), false);
}

void VideoCloudFailover::_setUri(State* state, const QString& uri, const QString& token)
{
    state->switching = true;
    state->receiver->setAuthToken(token);
    state->receiver->setUri(uri);
    state->switching = false;
    _restart(state->receiver);
}

int VideoCloudFailover::_nextHold(const State& state) const
{
    if (state.failedBack.isValid() && state.failedBack.elapsed() < _timing.settleMs) {
        return std::clamp(state.holdMs * 2, _timing.holdMinMs, _timing.holdMaxMs);
    }
    return _timing.holdMinMs;
}
