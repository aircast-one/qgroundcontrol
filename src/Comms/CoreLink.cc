#ifdef QGC_RUST_CORE
#include "CoreLink.h"

#include "QGCCoreC.h"
#include "QGCLoggingCategory.h"
#include "SettingsManager.h"
#include "AppSettings.h"
#include "SerialLink.h"
#include "TCPLink.h"
#include "UDPLink.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaObject>
#include <QtCore/QMutexLocker>

QGC_LOGGING_CATEGORY(CoreLinkLog, "qgc.comms.corelink")

QMutex CoreLink::_registryMutex;
QHash<uint32_t, CoreLink *> CoreLink::_links;
bool CoreLink::_sinksInstalled = false;

namespace {

QJsonObject takeJson(char *text)
{
    const QJsonObject json = QJsonDocument::fromJson(QByteArray(text ? text : "{}")).object();
    qgc_core_free(text);
    return json;
}

} // namespace

CoreLink::CoreLink(SharedLinkConfigurationPtr &config, QObject *parent)
    : LinkInterface(config, parent)
{
    _installSinks();
}

CoreLink::~CoreLink()
{
    CoreLink::disconnect();
    QMutexLocker locker(&_registryMutex);
    _links.remove(_id);
}

bool CoreLink::enabled()
{
    if (qEnvironmentVariableIntValue("QGC_CORE_LINKS") == 1) {
        return true;
    }
    AppSettings *const app = SettingsManager::instance() ? SettingsManager::instance()->appSettings() : nullptr;
    return app && app->coreLinks()->rawValue().toBool();
}

bool CoreLink::handles(LinkConfiguration::LinkType type)
{
    switch (type) {
#ifndef QGC_NO_SERIAL_LINK
    case LinkConfiguration::TypeSerial:
#endif
    case LinkConfiguration::TypeUdp:
    case LinkConfiguration::TypeTcp:
        return true;
    default:
        return false;
    }
}

bool CoreLink::isSecureConnection() const
{
#ifndef QGC_NO_SERIAL_LINK
    if (const SerialConfiguration *const serial = qobject_cast<const SerialConfiguration *>(_config.get())) {
        return serial->usbDirect();
    }
#endif
    return false;
}

QByteArray CoreLink::configJson(const LinkConfiguration *config)
{
    QJsonObject json {
        { QStringLiteral("name"), config->name() },
        { QStringLiteral("auto"), config->isAutoConnect() },
        { QStringLiteral("highLatency"), config->isHighLatency() },
        { QStringLiteral("viaLinkManager"), true },
    };
    if (const UDPConfiguration *const udp = qobject_cast<const UDPConfiguration *>(config)) {
        QJsonArray hosts;
        for (const std::shared_ptr<UDPClient> &target : udp->targetHosts()) {
            hosts.append(QJsonObject { { QStringLiteral("host"), target->address.toString() }, { QStringLiteral("port"), target->port } });
        }
        json.insert(QStringLiteral("kind"), QStringLiteral("udp"));
        json.insert(QStringLiteral("port"), udp->localPort());
        json.insert(QStringLiteral("hosts"), hosts);
    } else if (const TCPConfiguration *const tcp = qobject_cast<const TCPConfiguration *>(config)) {
        json.insert(QStringLiteral("kind"), QStringLiteral("tcp"));
        json.insert(QStringLiteral("host"), tcp->host());
        json.insert(QStringLiteral("port"), tcp->port());
#ifndef QGC_NO_SERIAL_LINK
    } else if (const SerialConfiguration *const serial = qobject_cast<const SerialConfiguration *>(config)) {
        json.insert(QStringLiteral("kind"), QStringLiteral("serial"));
        json.insert(QStringLiteral("baud"), serial->baud());
        json.insert(QStringLiteral("dataBits"), static_cast<int>(serial->dataBits()));
        json.insert(QStringLiteral("flowControl"), static_cast<int>(serial->flowControl()));
        json.insert(QStringLiteral("stopBits"), static_cast<int>(serial->stopBits()));
        json.insert(QStringLiteral("parity"), static_cast<int>(serial->parity()));
        json.insert(QStringLiteral("portName"), serial->portName());
        json.insert(QStringLiteral("portDisplayName"), serial->portDisplayName());
#endif
    }
    return QJsonDocument(json).toJson(QJsonDocument::Compact);
}

void CoreLink::_installSinks()
{
    QMutexLocker locker(&_registryMutex);
    if (_sinksInstalled) {
        return;
    }
    _sinksInstalled = true;
    qgc_core_set_link_bytes_sink(&CoreLink::_bytesArrived, nullptr);
    qgc_core_set_link_state_sink(&CoreLink::_stateChanged, nullptr);
}

void CoreLink::_bytesArrived(uint32_t id, const uint8_t *bytes, size_t len, void *user)
{
    Q_UNUSED(user);
    QMutexLocker locker(&_registryMutex);
    CoreLink *const link = _links.value(id, nullptr);
    if (!link) {
        return;
    }
    const QByteArray data(reinterpret_cast<const char *>(bytes), static_cast<qsizetype>(len));
    QMetaObject::invokeMethod(link, [link, data]() { emit link->bytesReceived(link, data); }, Qt::QueuedConnection);
}

void CoreLink::_stateChanged(uint32_t id, bool open, const char *reason, void *user)
{
    Q_UNUSED(user);
    if (open) {
        return;
    }
    const QString why = QString::fromUtf8(reason ? reason : "");
    QMutexLocker locker(&_registryMutex);
    CoreLink *const link = _links.value(id, nullptr);
    if (!link) {
        return;
    }
    QMetaObject::invokeMethod(link, [link, why]() {
        qCWarning(CoreLinkLog) << "core link closed:" << why;
        link->disconnect();
    }, Qt::QueuedConnection);
}

bool CoreLink::_connect()
{
    if (_id != 0) {
        return true;
    }
    const QJsonObject opened = takeJson(qgc_core_link_open(configJson(_config.get()).constData()));
    if (!opened.value(QStringLiteral("ok")).toBool(false)) {
        const QString reason = opened.value(QStringLiteral("reason")).toString();
        qCWarning(CoreLinkLog) << "core link open failed:" << reason;
        emit communicationError(tr("Link Error"), tr("Could not open %1: %2").arg(_config->name(), reason));
        return false;
    }
    _id = static_cast<uint32_t>(opened.value(QStringLiteral("id")).toInt());
    {
        QMutexLocker locker(&_registryMutex);
        _links.insert(_id, this);
    }
    emit connected();
    return true;
}

void CoreLink::disconnect()
{
    if (_id == 0) {
        return;
    }
    const uint32_t id = _id;
    _id = 0;
    {
        QMutexLocker locker(&_registryMutex);
        _links.remove(id);
    }
    (void) qgc_core_link_close(id, "disconnected by the link manager");
    emit disconnected();
}

void CoreLink::_writeBytes(const QByteArray &bytes)
{
    if (_id == 0 || bytes.isEmpty()) {
        return;
    }
    if (qgc_core_link_write(_id, reinterpret_cast<const uint8_t *>(bytes.constData()), static_cast<size_t>(bytes.size()))) {
        emit bytesSent(this, bytes);
    }
}

#endif
