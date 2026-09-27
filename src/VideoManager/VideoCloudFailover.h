#pragma once

#include <QtCore/QElapsedTimer>
#include <QtCore/QLoggingCategory>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtCore/QString>
#include <QtCore/QTimer>
#include <functional>
#include <memory>
#include <unordered_map>

class VideoReceiver;

Q_DECLARE_LOGGING_CATEGORY(VideoCloudFailoverLog)

class VideoCloudFailover : public QObject
{
    Q_OBJECT

public:
    struct Device
    {
        QString host;
        QString sfu;
        QString deviceId;
    };

    struct Timing
    {
        int stallMs = 5000;
        int probeMs = 5000;
        int probeSuccesses = 3;
        int holdMinMs = 15000;
        int holdMaxMs = 300000;
        int settleMs = 30000;
    };

    using TokenCallback = std::function<void(const QString& token)>;
    using FetchToken = std::function<void(const QString& deviceId, QObject* context, TokenCallback done)>;
    using Probe = std::function<void(const QString& host, QObject* context, std::function<void(bool ok)> done)>;
    using Restart = std::function<void(VideoReceiver* receiver)>;

    VideoCloudFailover(FetchToken fetchToken, Probe probe, Restart restart, Timing timing, QObject* parent = nullptr);

    void setDevice(const Device& device);

    const Device& device() const { return _device; }

    void watch(VideoReceiver* receiver);
    bool onCloud(const VideoReceiver* receiver) const;

    static QString cloudUrlFor(const Device& device, const QString& directUri);

signals:
    void switched(const QString& receiverName, bool toCloud);

private:
    struct State
    {
        QPointer<VideoReceiver> receiver;
        QTimer* stall = nullptr;
        QTimer* probe = nullptr;
        QString direct;
        bool decoding = false;
        bool onCloud = false;
        bool switching = false;
        bool probing = false;
        int probeOks = 0;
        int holdMs = 0;
        QElapsedTimer onCloudSince;
        QElapsedTimer failedBack;
    };

    void _armStall(State* state);
    void _stalled(State* state);
    void _useCloud(State* state, const QString& url, const QString& token);
    void _probeDirect(State* state);
    void _useDirect(State* state);
    void _setUri(State* state, const QString& uri, const QString& token);
    int _nextHold(const State& state) const;

    FetchToken _fetchToken;
    Probe _probe;
    Restart _restart;
    Timing _timing;
    Device _device;
    std::unordered_map<const VideoReceiver*, std::unique_ptr<State>> _states;
};
