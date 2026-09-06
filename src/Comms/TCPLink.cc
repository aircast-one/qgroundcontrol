/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "TCPLink.h"
#include "DeviceInfo.h"
#include "QGCLoggingCategory.h"

#include <QtCore/QThread>
#include <QtCore/QTimer>
#include <QtNetwork/QTcpSocket>

QGC_LOGGING_CATEGORY(TCPLinkLog, "qgc.comms.tcplink")

namespace {
    constexpr int CONNECT_TIMEOUT_MS = 1000;
    constexpr int TYPE_OF_SERVICE = 32;
}

TCPConfiguration::TCPConfiguration(const QString &name, QObject *parent)
    : LinkConfiguration(name, parent)
{
}

TCPConfiguration::TCPConfiguration(const TCPConfiguration *copy, QObject *parent)
    : LinkConfiguration(copy, parent)
    , _host(copy->host())
    , _port(copy->port())
{
}

TCPConfiguration::~TCPConfiguration()
{
}

void TCPConfiguration::copyFrom(const LinkConfiguration *source)
{
    Q_ASSERT(source);
    LinkConfiguration::copyFrom(source);

    const TCPConfiguration* const tcpSource = qobject_cast<const TCPConfiguration*>(source);
    Q_ASSERT(tcpSource);

    setHost(tcpSource->host());
    setPort(tcpSource->port());
}

void TCPConfiguration::loadSettings(QSettings &settings, const QString &root)
{
    settings.beginGroup(root);

    setHost(settings.value(QStringLiteral("host"), host()).toString());
    setPort(static_cast<quint16>(settings.value(QStringLiteral("port"), port()).toUInt()));

    settings.endGroup();
}

void TCPConfiguration::saveSettings(QSettings &settings, const QString &root) const
{
    settings.beginGroup(root);

    settings.setValue(QStringLiteral("host"), host());
    settings.setValue(QStringLiteral("port"), port());

    settings.endGroup();
}

TCPWorker::TCPWorker(const TCPConfiguration *config, QObject *parent)
    : QObject(parent)
    , _config(config)
{
}

TCPWorker::~TCPWorker()
{
    disconnectFromHost();

}

bool TCPWorker::isConnected() const
{
    return (_socket && _socket->isOpen() && (_socket->state() == QAbstractSocket::ConnectedState));
}

void TCPWorker::setupSocket()
{
    Q_ASSERT(!_socket);
    _socket = new QTcpSocket(this);

    _socket->setSocketOption(QAbstractSocket::LowDelayOption, 1);
    _socket->setSocketOption(QAbstractSocket::KeepAliveOption, 1);
    _socket->setSocketOption(QAbstractSocket::TypeOfServiceOption, TYPE_OF_SERVICE);

    (void) connect(_socket, &QTcpSocket::connected, this, &TCPWorker::_onSocketConnected);
    (void) connect(_socket, &QTcpSocket::disconnected, this, &TCPWorker::_onSocketDisconnected);
    (void) connect(_socket, &QTcpSocket::readyRead, this, &TCPWorker::_onSocketReadyRead);
    (void) connect(_socket, &QTcpSocket::errorOccurred, this, &TCPWorker::_onSocketErrorOccurred);

    if (TCPLinkLog().isDebugEnabled()) {

        (void) connect(_socket, &QTcpSocket::stateChanged, this, [](QTcpSocket::SocketState state) {
            qCDebug(TCPLinkLog) << "TCP State Changed:" << state;
        });

        (void) connect(_socket, &QTcpSocket::hostFound, this, []() {
            qCDebug(TCPLinkLog) << "TCP Host Found";
        });
    }
}

void TCPWorker::connectToHost()
{
    if (isConnected()) {
        qCWarning(TCPLinkLog) << "Already connected to" << _config->host() << ":" << _config->port();
        return;
    }

    _errorEmitted = false;

    qCDebug(TCPLinkLog) << "Attempting to connect to host:" << _config->host() << "port:" << _config->port();
    _socket->connectToHost(_config->host(), _config->port());

    if (!_socket->waitForConnected(CONNECT_TIMEOUT_MS)) {
        qCWarning(TCPLinkLog) << "Connection to" << _config->host() << ":" << _config->port() << "failed:" << _socket->errorString();

        if (!_errorEmitted) {
            emit errorOccurred(_socket->errorString(), _socket->error());
            _errorEmitted = true;
        }

        _onSocketDisconnected();
    } else {
        qCDebug(TCPLinkLog) << "Successfully connected to host:" << _config->host() << "port:" << _config->port();
    }
}

void TCPWorker::disconnectFromHost()
{
    if (!isConnected()) {
        qCDebug(TCPLinkLog) << "Already disconnected from host:" << _config->host() << "port:" << _config->port();
        return;
    }

    qCDebug(TCPLinkLog) << "Attempting to disconnect from host:" << _config->host() << "port:" << _config->port();

    _socket->disconnectFromHost();
}

void TCPWorker::writeData(const QByteArray &data)
{
    if (data.isEmpty()) {
        emit errorOccurred(tr("Data to Send is Empty"));
        return;
    }

    if (!isConnected()) {
        emit errorOccurred(tr("Socket is not connected"));
        return;
    }

    qint64 totalBytesWritten = 0;
    while (totalBytesWritten < data.size()) {
        const qint64 bytesWritten = _socket->write(data.constData() + totalBytesWritten, data.size() - totalBytesWritten);
        if (bytesWritten == -1) {
            emit errorOccurred(tr("Could Not Send Data - Write Failed: %1").arg(_socket->errorString()));
            return;
        } else if (bytesWritten == 0) {
            emit errorOccurred(tr("Could Not Send Data - Write Returned 0 Bytes"));
            return;
        }
        totalBytesWritten += bytesWritten;
    }

    emit dataSent(data.first(totalBytesWritten));
}

void TCPWorker::_onSocketConnected()
{
    qCDebug(TCPLinkLog) << "Socket connected:" << _config->host() << _config->port();
    _errorEmitted = false;
    emit connected();
}

void TCPWorker::_onSocketDisconnected()
{
    qCDebug(TCPLinkLog) << "Socket disconnected:" << _config->host() << _config->port();
    emit disconnected();
}

void TCPWorker::_onSocketReadyRead()
{
    const QByteArray data = _socket->readAll();
    emit dataReceived(data);
}

void TCPWorker::_onSocketBytesWritten(qint64 bytes)
{
    qCDebug(TCPLinkLog) << _config->host() << "Wrote" << bytes << "bytes";
}

void TCPWorker::_onSocketErrorOccurred(QAbstractSocket::SocketError socketError)
{
    const QString errorString = _socket->errorString();

    qCWarning(TCPLinkLog) << "Socket error:" << errorString;

    if (!_errorEmitted) {
        emit errorOccurred(errorString, socketError);
        _errorEmitted = true;
    }
}

TCPLink::TCPLink(SharedLinkConfigurationPtr &config, QObject *parent)
    : LinkInterface(config, parent)
    , _tcpConfig(qobject_cast<const TCPConfiguration*>(config.get()))
    , _worker(new TCPWorker(_tcpConfig))
    , _workerThread(new QThread(this))
{

    _workerThread->setObjectName(QStringLiteral("TCP_%1").arg(_tcpConfig->name()));

    _worker->moveToThread(_workerThread);

    (void) connect(_workerThread, &QThread::started, _worker, &TCPWorker::setupSocket);
    (void) connect(_workerThread, &QThread::finished, _worker, &QObject::deleteLater);

    (void) connect(_worker, &TCPWorker::connected, this, &TCPLink::_onConnected, Qt::QueuedConnection);
    (void) connect(_worker, &TCPWorker::disconnected, this, &TCPLink::_onDisconnected, Qt::QueuedConnection);
    (void) connect(_worker, &TCPWorker::errorOccurred, this, &TCPLink::_onErrorOccurred, Qt::QueuedConnection);
    (void) connect(_worker, &TCPWorker::dataReceived, this, &TCPLink::_onDataReceived, Qt::QueuedConnection);
    (void) connect(_worker, &TCPWorker::dataSent, this, &TCPLink::_onDataSent, Qt::QueuedConnection);

    _workerThread->start();
}

TCPLink::~TCPLink()
{
    TCPLink::disconnect();

    _workerThread->quit();
    if (!_workerThread->wait()) {
        qCWarning(TCPLinkLog) << "Failed to wait for TCP Thread to close";
    }

}

bool TCPLink::isConnected() const
{
    return _worker->isConnected();
}

bool TCPLink::_connect()
{
    return QMetaObject::invokeMethod(_worker, "connectToHost", Qt::QueuedConnection);
}

void TCPLink::disconnect()
{
    (void) QMetaObject::invokeMethod(_worker, "disconnectFromHost", Qt::QueuedConnection);
}

void TCPLink::_onConnected()
{
    emit connected();
}

void TCPLink::_onDisconnected()
{
    emit disconnected();
}

void TCPLink::_onErrorOccurred(const QString &errorString, QAbstractSocket::SocketError socketError)
{
    qCWarning(TCPLinkLog) << "Communication error:" << socketError << errorString;
    const QString title = tr("Couldn't connect to %1").arg(_tcpConfig->name());
    if (_tcpConfig->host().isEmpty()) {
        emit communicationError(title, tr("%1 has no address.").arg(_tcpConfig->name()), LinkConfiguration::RemedyEditAddress);
        return;
    }
    switch (socketError) {
    case QAbstractSocket::HostNotFoundError:
        emit communicationError(title, tr("Can't find %1 on this network.").arg(_tcpConfig->host()), LinkConfiguration::RemedyEditAddress);
        return;
    case QAbstractSocket::ConnectionRefusedError:
        emit communicationError(title, tr("Reached %1 but nothing is listening on port %2.").arg(_tcpConfig->host()).arg(_tcpConfig->port()), LinkConfiguration::RemedyEditAddress);
        return;
    default:
        emit communicationError(title, tr("No answer from %1:%2 (%3).").arg(_tcpConfig->host()).arg(_tcpConfig->port()).arg(errorString), LinkConfiguration::RemedyRetry);
        return;
    }
}

void TCPLink::_onDataReceived(const QByteArray &data)
{
    emit bytesReceived(this, data);
}

void TCPLink::_onDataSent(const QByteArray &data)
{
    emit bytesSent(this, data);
}

void TCPLink::_writeBytes(const QByteArray& bytes)
{
    (void) QMetaObject::invokeMethod(_worker, "writeData", Qt::QueuedConnection, Q_ARG(QByteArray, bytes));
}

bool TCPLink::isSecureConnection() const
{
    return QGCDeviceInfo::isNetworkEthernet();
}
