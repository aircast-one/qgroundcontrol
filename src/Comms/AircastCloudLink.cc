#include "AircastCloudLink.h"

#include <QtCore/QTimer>
#include <QtNetwork/QNetworkRequest>
#include <QtWebSockets/QWebSocket>

#include "AircastAccount.h"
#include "QGCLoggingCategory.h"

QGC_LOGGING_CATEGORY(AircastCloudLinkLog, "Comms.AircastCloudLink")

namespace {
constexpr int kInitialBackoffMs = 2000;
constexpr int kMaxBackoffMs = 60000;
}  // namespace

AircastCloudConfiguration::AircastCloudConfiguration(const QString& name, QObject* parent)
    : LinkConfiguration(name, parent)
{}

AircastCloudConfiguration::AircastCloudConfiguration(const AircastCloudConfiguration* copy, QObject* parent)
    : LinkConfiguration(copy, parent), _apiBase(copy->apiBase()), _deviceId(copy->deviceId())
{}

AircastCloudConfiguration::~AircastCloudConfiguration() = default;

void AircastCloudConfiguration::setApiBase(const QString& apiBase)
{
    const QString clean = apiBase.trimmed();
    if (clean != _apiBase) {
        _apiBase = clean;
        emit apiBaseChanged();
        emit summaryChanged();
    }
}

void AircastCloudConfiguration::setDeviceId(const QString& deviceId)
{
    const QString clean = deviceId.trimmed();
    if (clean != _deviceId) {
        _deviceId = clean;
        emit deviceIdChanged();
        emit summaryChanged();
    }
}

QUrl AircastCloudConfiguration::relayUrl() const
{
    QUrl url(_apiBase);
    if (!url.isValid() || url.host().isEmpty() || _deviceId.isEmpty()) {
        return {};
    }
    url.setScheme(url.scheme() == QStringLiteral("http") ? QStringLiteral("ws") : QStringLiteral("wss"));
    QString path = url.path();
    if (path.endsWith(QLatin1Char('/'))) {
        path.chop(1);
    }
    url.setPath(path + QStringLiteral("/v1/mavlink/web/%1/ws").arg(_deviceId));
    return url;
}

void AircastCloudConfiguration::copyFrom(const LinkConfiguration* source)
{
    LinkConfiguration::copyFrom(source);
    const auto* cloudSource = qobject_cast<const AircastCloudConfiguration*>(source);
    if (!cloudSource) {
        return;
    }
    setApiBase(cloudSource->apiBase());
    setDeviceId(cloudSource->deviceId());
}

void AircastCloudConfiguration::loadSettings(QSettings& settings, const QString& root)
{
    settings.beginGroup(root);
    setApiBase(settings.value(QStringLiteral("apiBase"), apiBase()).toString());
    setDeviceId(settings.value(QStringLiteral("deviceId"), deviceId()).toString());
    settings.endGroup();
}

void AircastCloudConfiguration::saveSettings(QSettings& settings, const QString& root) const
{
    settings.beginGroup(root);
    settings.setValue(QStringLiteral("apiBase"), apiBase());
    settings.setValue(QStringLiteral("deviceId"), deviceId());
    settings.endGroup();
}

AircastCloudLink::AircastCloudLink(SharedLinkConfigurationPtr& config, QObject* parent)
    : LinkInterface(config, parent),
      _cloudConfig(qobject_cast<const AircastCloudConfiguration*>(config.get())),
      _socket(new QWebSocket(QString(), QWebSocketProtocol::VersionLatest, this)),
      _reconnectTimer(new QTimer(this))
{
    _reconnectTimer->setSingleShot(true);
    (void) connect(_reconnectTimer, &QTimer::timeout, this, &AircastCloudLink::_open);
    (void) connect(_socket, &QWebSocket::connected, this, &AircastCloudLink::_onConnected);
    (void) connect(_socket, &QWebSocket::disconnected, this, &AircastCloudLink::_onDisconnected);
    (void) connect(_socket, &QWebSocket::binaryMessageReceived, this, &AircastCloudLink::_onBinaryMessage);
    (void) connect(_socket, &QWebSocket::errorOccurred, this, &AircastCloudLink::_onError);
    (void) connect(AircastAccount::instance(), &AircastAccount::tokensChanged, this, [this]() {
        if (_wanted && !isConnected()) {
            _failures = 0;
            _open();
        }
    });
}

AircastCloudLink::~AircastCloudLink()
{
    _wanted = false;
    _socket->abort();
}

bool AircastCloudLink::isConnected() const
{
    return _socket->state() == QAbstractSocket::ConnectedState;
}

bool AircastCloudLink::_connect()
{
    _wanted = true;
    _failures = 0;
    return QMetaObject::invokeMethod(this, &AircastCloudLink::_open, Qt::QueuedConnection);
}

void AircastCloudLink::_open()
{
    if (!_wanted || _socket->state() != QAbstractSocket::UnconnectedState) {
        return;
    }
    const QUrl url = _cloudConfig->relayUrl();
    if (!url.isValid()) {
        emit communicationError(tr("Aircast cloud link"), tr("%1 has no device to reach.").arg(_cloudConfig->name()),
                                LinkConfiguration::RemedyEditAddress);
        return;
    }
    const QString token = AircastAccount::instance()->token(_cloudConfig->apiBase());
    if (token.isEmpty()) {
        if (!_reportedError) {
            _reportedError = true;
            emit communicationError(
                tr("Aircast cloud link"),
                tr("Sign in to your Aircast account in this link's settings to use the cloud backup link."),
                LinkConfiguration::RemedyEditAddress);
        }
        return;
    }

    QNetworkRequest request(url);
    request.setRawHeader(QByteArrayLiteral("Authorization"), QByteArrayLiteral("Bearer ") + token.toUtf8());
    qCDebug(AircastCloudLinkLog) << "opening" << url;
    _socket->open(request);
}

void AircastCloudLink::disconnect()
{
    _wanted = false;
    _reconnectTimer->stop();
    if (_socket->state() != QAbstractSocket::UnconnectedState) {
        _socket->close();
        return;
    }
    emit disconnected();
}

void AircastCloudLink::_onConnected()
{
    qCDebug(AircastCloudLinkLog) << "connected to the relay";
    _failures = 0;
    _reportedError = false;
    emit connected();
}

void AircastCloudLink::_onDisconnected()
{
    if (!_wanted) {
        (void) QMetaObject::invokeMethod(this, [this]() { emit disconnected(); }, Qt::QueuedConnection);
        return;
    }
    qCDebug(AircastCloudLinkLog) << "relay connection closed" << _socket->closeCode() << _socket->closeReason();
    _scheduleReconnect();
}

void AircastCloudLink::_scheduleReconnect()
{
    if (_reconnectTimer->isActive()) {
        return;
    }
    const int backoff = std::min(kInitialBackoffMs << std::min(_failures, 5), kMaxBackoffMs);
    ++_failures;
    _reconnectTimer->start(backoff);
}

void AircastCloudLink::_onBinaryMessage(const QByteArray& message)
{
    if (message.isEmpty()) {
        return;
    }
    if (message.at(0) == kControlPrefix) {
        qCDebug(AircastCloudLinkLog) << "relay status" << message.mid(2);
        return;
    }
    emit bytesReceived(this, message);
}

void AircastCloudLink::_writeBytes(const QByteArray& bytes)
{
    if (!isConnected()) {
        return;
    }
    (void) _socket->sendBinaryMessage(bytes);
    emit bytesSent(this, bytes);
}

void AircastCloudLink::_onError(QAbstractSocket::SocketError error)
{
    if (!_wanted) {
        return;
    }
    qCWarning(AircastCloudLinkLog) << "relay error" << error << _socket->errorString();
    if (!_reportedError) {
        _reportedError = true;
        emit communicationError(tr("Aircast cloud link"),
                                tr("Can't reach the Aircast relay (%1). Retrying.").arg(_socket->errorString()),
                                LinkConfiguration::RemedyRetry);
    }
    if (_socket->state() == QAbstractSocket::UnconnectedState) {
        _scheduleReconnect();
    }
}
