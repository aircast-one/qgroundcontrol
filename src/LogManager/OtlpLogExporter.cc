#include "OtlpLogExporter.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QSysInfo>
#include <QtCore/QUrl>
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkReply>
#include <QtNetwork/QNetworkRequest>

#include "QGCLoggingCategory.h"

QGC_LOGGING_CATEGORY(OtlpLogExporterLog, "Utilities.OtlpLogExporter")

namespace {
constexpr int kFlushIntervalMs = 5000;
constexpr int kMaxQueued = 2000;
constexpr int kMaxPerRequest = 400;
constexpr int kRequestTimeoutMs = 15000;
constexpr int kTokenRenewSec = 300;
}  // namespace

OtlpLogExporter::OtlpLogExporter(QObject* parent) : QObject(parent)
{
    _timer.setInterval(kFlushIntervalMs);
    (void) connect(&_timer, &QTimer::timeout, this, &OtlpLogExporter::flush);
}

OtlpLogExporter::~OtlpLogExporter() = default;

void OtlpLogExporter::configure(const QString& endpoint, const QString& token, bool enabled)
{
    _url = logsURL(endpoint);
    const QString trimmedToken = token.trimmed();
    _storedToken = !trimmedToken.isEmpty();
    if (_storedToken) {
        // A collector of your own is reached with a key of your own.
        _auth = QByteArray("Bearer ") + trimmedToken.toUtf8();
        _tokenExpiry = QDateTime();
    } else {
        _auth.clear();
        _tokenExpiry = QDateTime();
    }
    _enabled = enabled && !_url.isEmpty();

    if (_enabled) {
        _timer.start();
    } else {
        _timer.stop();
        // Whatever was queued was for a collector this machine is no longer
        // talking to; holding it only grows.
        _pending.clear();
    }
}

void OtlpLogExporter::setIdentity(const QString& serviceName, const QString& version, const QString& sessionId)
{
    if (!serviceName.isEmpty()) {
        _serviceName = serviceName;
    }
    _version = version;
    _sessionId = sessionId;
}

void OtlpLogExporter::enqueue(const LogEntry& entry)
{
    if (!_enabled) {
        return;
    }
    // Debug chatter is the bulk of the log and the least of what support
    // needs; it stays on disk where the whole journal already is.
    if (entry.level < LogEntry::Info) {
        return;
    }
    if (_pending.size() >= kMaxQueued) {
        _pending.removeFirst();
        ++_dropped;
    }
    _pending.append(entry);
}

void OtlpLogExporter::setDeviceHost(const QString& host)
{
    _deviceHost = host.trimmed();
}

bool OtlpLogExporter::hasCredential() const
{
    return !_auth.isEmpty() || !_deviceHost.isEmpty();
}

// A minted token is short-lived on purpose. Asking again before it runs out
// keeps the log flowing; asking only after a 401 would lose whatever the
// operator was trying to capture.
bool OtlpLogExporter::_credentialUsable() const
{
    if (_auth.isEmpty()) {
        return false;
    }
    if (_storedToken || !_tokenExpiry.isValid()) {
        return true;
    }
    return QDateTime::currentDateTimeUtc().secsTo(_tokenExpiry) > kTokenRenewSec;
}

void OtlpLogExporter::flush()
{
    if (!_enabled || _inFlight || _pending.isEmpty() || _url.isEmpty()) {
        return;
    }
    if (!_credentialUsable()) {
        _requestToken();
        return;
    }
    _send();
}

// The device mints this from the cloud account it is signed in to. Nothing
// secret is stored here and nothing secret ships in the application: the
// worst an extracted build yields is the address of a collector that will
// not talk to it.
void OtlpLogExporter::_requestToken()
{
    if (_tokenRequested || _deviceHost.isEmpty()) {
        return;
    }
    if (!_network) {
        _network = new QNetworkAccessManager(this);
    }
    _tokenRequested = true;

    QNetworkRequest request{QUrl(QStringLiteral("http://%1/api/diagnostics/token").arg(_deviceHost))};
    request.setTransferTimeout(kRequestTimeoutMs);
    QNetworkReply* reply = _network->get(request);
    (void) connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        reply->deleteLater();
        _tokenRequested = false;
        if (reply->error() != QNetworkReply::NoError) {
            qCDebug(OtlpLogExporterLog) << "no diagnostics token from" << _deviceHost << reply->errorString();
            return;
        }
        const QJsonObject body = QJsonDocument::fromJson(reply->readAll()).object();
        const QString token = body.value(QStringLiteral("token")).toString().trimmed();
        if (token.isEmpty()) {
            return;
        }
        _auth = QByteArray("Bearer ") + token.toUtf8();
        _tokenExpiry = QDateTime::fromString(body.value(QStringLiteral("expiresAt")).toString(), Qt::ISODate);
        // The device also says where it ships its own telemetry, which is
        // where this belongs too unless someone has said otherwise.
        const QString endpoint = body.value(QStringLiteral("endpoint")).toString();
        if (_url.isEmpty() && !endpoint.isEmpty()) {
            _url = logsURL(endpoint);
        }
        flush();
    });
}

void OtlpLogExporter::_send()
{
    const int count = qMin(_pending.size(), kMaxPerRequest);
    const QList<LogEntry> batch = _pending.mid(0, count);
    const QByteArray body = buildPayload(batch);

    if (!_network) {
        _network = new QNetworkAccessManager(this);
    }
    QNetworkRequest request{QUrl(_url)};
    request.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));
    request.setTransferTimeout(kRequestTimeoutMs);
    if (!_auth.isEmpty()) {
        request.setRawHeader("Authorization", _auth);
    }

    _inFlight = true;
    QNetworkReply* reply = _network->post(request, body);
    (void) connect(reply, &QNetworkReply::finished, this, [this, reply, count]() {
        reply->deleteLater();
        _inFlight = false;
        if (reply->error() != QNetworkReply::NoError) {
            // Kept, not dropped: a collector that is unreachable for a minute
            // should not cost the minute of log that explains why.
            qCDebug(OtlpLogExporterLog) << "OTLP log export failed" << reply->errorString();
            return;
        }
        _pending.remove(0, qMin(count, _pending.size()));
        if (!_pending.isEmpty()) {
            flush();
        }
    });
}

QJsonObject OtlpLogExporter::_resource() const
{
    const auto attr = [](const QString& key, const QString& value) {
        QJsonObject v;
        v.insert(QStringLiteral("stringValue"), value);
        QJsonObject kv;
        kv.insert(QStringLiteral("key"), key);
        kv.insert(QStringLiteral("value"), v);
        return kv;
    };

    QJsonArray attrs;
    attrs.append(attr(QStringLiteral("service.name"), _serviceName));
    if (!_version.isEmpty()) {
        attrs.append(attr(QStringLiteral("service.version"), _version));
    }
    if (!_sessionId.isEmpty()) {
        attrs.append(attr(QStringLiteral("session.id"), _sessionId));
    }
    attrs.append(attr(QStringLiteral("os.type"), QSysInfo::productType()));
    attrs.append(attr(QStringLiteral("host.arch"), QSysInfo::currentCpuArchitecture()));

    QJsonObject resource;
    resource.insert(QStringLiteral("attributes"), attrs);
    return resource;
}

QByteArray OtlpLogExporter::buildPayload(const QList<LogEntry>& entries) const
{
    const auto stringAttr = [](const QString& key, const QString& value) {
        QJsonObject v;
        v.insert(QStringLiteral("stringValue"), value);
        QJsonObject kv;
        kv.insert(QStringLiteral("key"), key);
        kv.insert(QStringLiteral("value"), v);
        return kv;
    };
    const auto intAttr = [](const QString& key, qint64 value) {
        QJsonObject v;
        v.insert(QStringLiteral("intValue"), QString::number(value));
        QJsonObject kv;
        kv.insert(QStringLiteral("key"), key);
        kv.insert(QStringLiteral("value"), v);
        return kv;
    };

    QJsonArray records;
    for (const LogEntry& entry : entries) {
        QJsonArray attrs;
        if (!entry.category.isEmpty()) {
            attrs.append(stringAttr(QStringLiteral("log.category"), entry.category));
        }
        if (!entry.file.isEmpty()) {
            attrs.append(stringAttr(QStringLiteral("code.filepath"), entry.file));
        }
        if (!entry.function.isEmpty()) {
            attrs.append(stringAttr(QStringLiteral("code.function"), entry.function));
        }
        if (entry.line > 0) {
            attrs.append(intAttr(QStringLiteral("code.lineno"), entry.line));
        }

        QJsonObject body;
        body.insert(QStringLiteral("stringValue"), entry.message);

        QJsonObject record;
        const qint64 unixMs =
            entry.timestamp.isValid() ? entry.timestamp.toMSecsSinceEpoch() : QDateTime::currentMSecsSinceEpoch();
        record.insert(QStringLiteral("timeUnixNano"), QString::number(unixMs * 1000000LL));
        record.insert(QStringLiteral("severityNumber"), severityNumber(entry.level));
        record.insert(QStringLiteral("severityText"), severityText(entry.level));
        record.insert(QStringLiteral("body"), body);
        record.insert(QStringLiteral("attributes"), attrs);
        records.append(record);
    }

    QJsonObject scope;
    scope.insert(QStringLiteral("name"), _serviceName);

    QJsonObject scopeLogs;
    scopeLogs.insert(QStringLiteral("scope"), scope);
    scopeLogs.insert(QStringLiteral("logRecords"), records);

    QJsonObject resourceLogs;
    resourceLogs.insert(QStringLiteral("resource"), _resource());
    resourceLogs.insert(QStringLiteral("scopeLogs"), QJsonArray{scopeLogs});

    QJsonObject root;
    root.insert(QStringLiteral("resourceLogs"), QJsonArray{resourceLogs});
    return QJsonDocument(root).toJson(QJsonDocument::Compact);
}

QString OtlpLogExporter::logsURL(const QString& endpoint)
{
    QString rest = endpoint.trimmed();
    if (rest.isEmpty()) {
        return {};
    }

    QString scheme;
    const int schemeEnd = rest.indexOf(QStringLiteral("://"));
    if (schemeEnd >= 0) {
        scheme = rest.left(schemeEnd).toLower();
        rest = rest.mid(schemeEnd + 3);
    } else {
        // The plaintext OTLP ports are the collector saying it speaks no TLS;
        // everything else is assumed to.
        const int colon = rest.lastIndexOf(QLatin1Char(':'));
        const QString port = (colon >= 0) ? rest.mid(colon + 1).section(QLatin1Char('/'), 0, 0) : QString();
        scheme = (port == QLatin1String("4317") || port == QLatin1String("4318")) ? QStringLiteral("http")
                                                                                  : QStringLiteral("https");
    }

    QString host = rest;
    QString path = QStringLiteral("/v1/logs");
    const int slash = rest.indexOf(QLatin1Char('/'));
    if (slash >= 0) {
        host = rest.left(slash);
        path = rest.mid(slash);
    }
    if (host.isEmpty()) {
        return {};
    }
    return scheme + QStringLiteral("://") + host + path;
}

int OtlpLogExporter::severityNumber(LogEntry::Level level)
{
    switch (level) {
        case LogEntry::Debug:
            return 5;
        case LogEntry::Info:
            return 9;
        case LogEntry::Warning:
            return 13;
        case LogEntry::Critical:
            return 17;
        case LogEntry::Fatal:
            return 21;
    }
    return 9;
}

QString OtlpLogExporter::severityText(LogEntry::Level level)
{
    switch (level) {
        case LogEntry::Debug:
            return QStringLiteral("DEBUG");
        case LogEntry::Info:
            return QStringLiteral("INFO");
        case LogEntry::Warning:
            return QStringLiteral("WARN");
        case LogEntry::Critical:
            return QStringLiteral("ERROR");
        case LogEntry::Fatal:
            return QStringLiteral("FATAL");
    }
    return QStringLiteral("INFO");
}
