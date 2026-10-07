#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QDateTime>
#include <QtCore/QJsonObject>
#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtCore/QTimer>

#include "LogEntry.h"

class QNetworkAccessManager;

// OtlpLogExporter ships this application's log to the same collector the
// aircraft's own daemon ships to, in the same shape: OTLP over HTTP, bearer
// token, batched. A ground station and the device it flies are one story, and
// until now only one half of it left the machine it happened on.
//
// JSON rather than protobuf: the collector accepts both, and the JSON encoding
// costs no dependency in a build that already carries Qt's network and JSON.
class OtlpLogExporter : public QObject
{
    Q_OBJECT

public:
    explicit OtlpLogExporter(QObject* parent = nullptr);
    ~OtlpLogExporter() override;

    void configure(const QString& endpoint, const QString& token, bool enabled);

    // setDeviceHost names the aircraft this ground station is paired with. A
    // ground station has no cloud account of its own, so it borrows the
    // device's: a short-lived token, asked for when the last one is running
    // out, instead of a key shipped inside the application where anyone
    // holding the binary could read it back out.
    void setDeviceHost(const QString& host);
    void setIdentity(const QString& serviceName, const QString& version, const QString& sessionId);

    [[nodiscard]] bool enabled() const { return _enabled; }

    [[nodiscard]] int queued() const { return _pending.size(); }

    [[nodiscard]] quint64 dropped() const { return _dropped; }

    void enqueue(const LogEntry& entry);
    void flush();

    // logsURL turns what an operator typed into the endpoint a collector
    // serves. Bare "host:port" means https, except on the plaintext OTLP
    // ports, and an endpoint that already carries a path keeps it.
    [[nodiscard]] static QString logsURL(const QString& endpoint);

    // defaultToken is the key this build was given at link time, offered when
    // this installation has never been given one of its own. Empty in a build
    // that was handed none.
    [[nodiscard]] bool hasCredential() const;
    [[nodiscard]] static int severityNumber(LogEntry::Level level);
    [[nodiscard]] static QString severityText(LogEntry::Level level);

    // Payload is exposed for tests: the body is the contract with the
    // collector, and a shape that drifts fails silently in production.
    [[nodiscard]] QByteArray buildPayload(const QList<LogEntry>& entries) const;

private:
    void _send();
    [[nodiscard]] bool _credentialUsable() const;
    void _requestToken();
    [[nodiscard]] QJsonObject _resource() const;

    QNetworkAccessManager* _network = nullptr;
    QTimer _timer;
    QList<LogEntry> _pending;
    QString _url;
    QByteArray _auth;
    QString _deviceHost;
    QDateTime _tokenExpiry;
    bool _tokenRequested = false;
    bool _storedToken = false;
    QString _serviceName = QStringLiteral("aircast-qgc");
    QString _version;
    QString _sessionId;
    bool _enabled = false;
    bool _inFlight = false;
    quint64 _dropped = 0;
};
