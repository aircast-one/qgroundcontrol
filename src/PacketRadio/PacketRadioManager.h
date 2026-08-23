#pragma once

#include <QtCore/QLoggingCategory>
#include <QtCore/QObject>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtCore/QVariant>
#include <QtQmlIntegration/QtQmlIntegration>

#include <memory>
#include <string>
#include <vector>

struct DeviceId;
class WfbngLink;
class PacketRadioTest;

Q_DECLARE_LOGGING_CATEGORY(PacketRadioManagerLog)

class PacketRadioManager : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("Reference only")

    Q_PROPERTY(Status status                READ status             NOTIFY statusChanged)
    Q_PROPERTY(QString statusText           READ statusText         NOTIFY statusChanged)
    Q_PROPERTY(bool linkActive              READ linkActive         NOTIFY statusChanged)
    Q_PROPERTY(QString adapterName          READ adapterName        NOTIFY statusChanged)
    Q_PROPERTY(QStringList adapters         READ adapters           NOTIFY adaptersChanged)
    Q_PROPERTY(QVariantList antennaRssi     READ antennaRssi        NOTIFY statsChanged)
    Q_PROPERTY(QVariantList antennaSnr      READ antennaSnr         NOTIFY statsChanged)
    Q_PROPERTY(bool haveSignal              READ haveSignal         NOTIFY statsChanged)
    Q_PROPERTY(int linkScore                READ linkScore          NOTIFY statsChanged)
    Q_PROPERTY(int packetLoss               READ packetLoss         NOTIFY statsChanged)
    Q_PROPERTY(qlonglong videoPackets       READ videoPackets       NOTIFY statsChanged)

public:
    explicit PacketRadioManager(QObject *parent = nullptr);
    ~PacketRadioManager() override;

    enum Status {
        Disabled,
        NoAdapter,
        AdapterUnavailable,
        InvalidKey,
        Listening,
        Receiving
    };
    Q_ENUM(Status)

    static PacketRadioManager *instance();

    static const DeviceId *selectAdapter(const std::vector<DeviceId> &devices, const std::string &savedName);

    void init();

    Status status() const { return _status; }
    QString statusText() const;
    bool linkActive() const { return (_status == Listening) || (_status == Receiving); }
    QString adapterName() const { return _adapterName; }
    QStringList adapters() const { return _adapters; }
    QVariantList antennaRssi() const { return _antennaRssi; }
    QVariantList antennaSnr() const { return _antennaSnr; }
    bool haveSignal() const { return _haveSignal; }
    int linkScore() const { return _linkScore; }
    int packetLoss() const { return _packetLoss; }
    qlonglong videoPackets() const { return _rtpPackets; }

    Q_INVOKABLE void refreshAdapters();

signals:
    void statusChanged();
    void adaptersChanged();
    void statsChanged();

private:
    friend class PacketRadioTest;

    void _evaluate();
    void _tryStart();
    void _stop();
    void _releaseLink();
    void _poll();
    void _applyAdaptiveLink();
    void _applyVideoSettings(const QString &codec);
    void _restoreVideoSettings();
    void _setStatus(Status status);
    void _updateAdapters(const std::vector<DeviceId> &devices);
    static std::vector<DeviceId> _enumerate();
    QString _resolveKeyPath();

    std::unique_ptr<WfbngLink> _link;
    QTimer _retryTimer;
    QTimer _pollTimer;
    bool _running = false;
    bool _callbacksInstalled = false;

    Status _status = Disabled;
    QString _adapterName;
    QString _startError;
    QStringList _adapters;
    QVariantList _antennaRssi;
    QVariantList _antennaSnr;
    bool _haveSignal = false;
    int _linkScore = 0;
    int _packetLoss = 0;
    qlonglong _rtpPackets = 0;
    qlonglong _lastRtpPackets = 0;

    bool _videoOverridden = false;
    QVariant _savedVideoSource;
    QVariant _savedUdpUrl;
    QVariant _savedLowLatency;
};
