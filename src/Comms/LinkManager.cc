#include "LinkManager.h"
#include "LogReplayLink.h"
#include "QGCNetworkHelper.h"
#include "MAVLinkProtocol.h"
#include "MultiVehicleManager.h"
#include "AppMessages.h"
#include "QGCLoggingCategory.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "MavlinkSettings.h"
#include "AutoConnectSettings.h"
#include "TCPLink.h"
#include "AircastCloudLink.h"
#include "CoreLink.h"
#include "UDPLink.h"

#include "BluetoothLink.h"

#include "PositionManager.h"
#include "UdpIODevice.h"

#ifndef QGC_NO_SERIAL_LINK
#include "SerialLink.h"
#include "GPSManager.h"
#include "GPSRtk.h"
#ifdef Q_OS_ANDROID
#include "AndroidSerial.h"
#include "AppSettings.h"
#endif
#endif

#ifdef QT_DEBUG
#include "MockLink.h"
#endif

#include <QtCore/QApplicationStatic>
#include <QtCore/QTimer>

QGC_LOGGING_CATEGORY(LinkManagerLog, "Comms.LinkManager")
QGC_LOGGING_CATEGORY(LinkManagerVerboseLog, "Comms.LinkManager:verbose")

Q_APPLICATION_STATIC(LinkManager, _linkManagerInstance);

LinkManager::LinkManager(QObject *parent)
    : QObject(parent)
    , _portListTimer(new QTimer(this))
    , _connectingStallTimer(new QTimer(this))
    , _qmlConfigurations(new QmlObjectListModel(this))
    , _nmeaSocket(new UdpIODevice(this))
{
    _connectingStallTimer->setSingleShot(true);
    _connectingStallTimer->setInterval(_connectingStallMSecs);
    (void) connect(_connectingStallTimer, &QTimer::timeout, this, [this]() { _setConnectingStalled(true); });

    qCDebug(LinkManagerLog) << this;

    (void) qRegisterMetaType<QAbstractSocket::SocketError>("QAbstractSocket::SocketError");
    (void) qRegisterMetaType<LinkInterface*>("LinkInterface*");
#ifndef QGC_NO_SERIAL_LINK
    (void) qRegisterMetaType<QGCSerialPortInfo>("QGCSerialPortInfo");
#endif
}

LinkManager::~LinkManager()
{
    qCDebug(LinkManagerLog) << this;
}

LinkManager *LinkManager::instance()
{
    return _linkManagerInstance();
}

void LinkManager::init()
{
    _autoConnectSettings = SettingsManager::instance()->autoConnectSettings();

#if defined(Q_OS_ANDROID) && !defined(QGC_NO_SERIAL_LINK)
    AndroidSerial::setUsePosixSerial(
        SettingsManager::instance()->appSettings()->androidUsePosixSerial()->rawValue().toBool());
#endif

    if (!QGC::runningUnitTests()) {
        (void) connect(_portListTimer, &QTimer::timeout, this, &LinkManager::_updateAutoConnectLinks);
        _portListTimer->start(_autoconnectUpdateTimerMSecs);
    }
}

QList<SharedLinkInterfacePtr> LinkManager::links()
{
    QMutexLocker locker(&_linksMutex);
    return _rgLinks;
}

QmlObjectListModel *LinkManager::_qmlLinkConfigurations()
{
    return _qmlConfigurations;
}

void LinkManager::createConnectedLink(const LinkConfiguration *config)
{
    for (SharedLinkConfigurationPtr &sharedConfig : _rgLinkConfigs) {
        if (sharedConfig.get() == config) {
            sharedConfig->setAutoConnectStarted(true);
            sharedConfig->resetReconnectBackoff();
            createConnectedLink(sharedConfig);
        }
    }
}

void LinkManager::disconnectLink(LinkInterface *link)
{
    if (!link) {
        return;
    }

    const SharedLinkConfigurationPtr config = link->linkConfiguration();
    if (config) {
        config->setSuppressAutoReconnect(true);
    }

    link->disconnect();
}

void LinkManager::disconnectLinkConfiguration(LinkConfiguration *config)
{
    if (!config) {
        return;
    }

    config->setSuppressAutoReconnect(true);

    if (LinkInterface *const link = config->link()) {
        link->disconnect();
    }
}

bool LinkManager::createConnectedLink(SharedLinkConfigurationPtr &config)
{
    if (config->link()) {
        qCDebug(LinkManagerLog) << Q_FUNC_INFO << config->name() << "is already connected";
        return true;
    }

    config->setSuppressAutoReconnect(false);

    SharedLinkInterfacePtr link = nullptr;
    if (CoreLink::enabled() && CoreLink::handles(config->type())) {
        link = std::make_shared<CoreLink>(config);
    }
    if (!link)
    switch(config->type()) {
#ifndef QGC_NO_SERIAL_LINK
    case LinkConfiguration::TypeSerial:
        link = std::make_shared<SerialLink>(config);
        break;
#endif
    case LinkConfiguration::TypeUdp:
        link = std::make_shared<UDPLink>(config);
        break;
    case LinkConfiguration::TypeTcp:
        link = std::make_shared<TCPLink>(config);
        break;
    case LinkConfiguration::TypeBluetooth:
        link = std::make_shared<BluetoothLink>(config);
        break;
    case LinkConfiguration::TypeLogReplay:
        link = std::make_shared<LogReplayLink>(config);
        break;
    case LinkConfiguration::TypeAircastCloud:
        link = std::make_shared<AircastCloudLink>(config);
        break;
#ifdef QT_DEBUG
    case LinkConfiguration::TypeMock:
        link = std::make_shared<MockLink>(config);
        break;
#endif
    case LinkConfiguration::TypeLast:
    default:
        break;
    }

    if (!link) {
        return false;
    }

    if (!link->_allocateMavlinkChannel()) {
        qCWarning(LinkManagerLog) << "Link failed to setup mavlink channels";
        return false;
    }

    (void) connect(link.get(), &LinkInterface::communicationError, this, &LinkManager::_communicationError);
    (void) connect(link.get(), &LinkInterface::bytesReceived, MAVLinkProtocol::instance(), &MAVLinkProtocol::receiveBytes);
    (void) connect(link.get(), &LinkInterface::bytesSent, MAVLinkProtocol::instance(), &MAVLinkProtocol::logSentBytes);
    (void) connect(link.get(), &LinkInterface::connected, this, &LinkManager::_linkConnected);
    (void) connect(link.get(), &LinkInterface::disconnected, this, &LinkManager::_linkDisconnected);

    MAVLinkProtocol::instance()->resetMetadataForLink(link.get());

    if (!link->_connect()) {
        (void) disconnect(link.get(), &LinkInterface::communicationError, this, &LinkManager::_communicationError);
        (void) disconnect(link.get(), &LinkInterface::bytesReceived, MAVLinkProtocol::instance(), &MAVLinkProtocol::receiveBytes);
        (void) disconnect(link.get(), &LinkInterface::bytesSent, MAVLinkProtocol::instance(), &MAVLinkProtocol::logSentBytes);
        (void) disconnect(link.get(), &LinkInterface::disconnected, this, &LinkManager::_linkDisconnected);
        link->_freeMavlinkChannel();
        config->setLink(nullptr);
        return false;
    }

    {
        QMutexLocker locker(&_linksMutex);
        _rgLinks.append(link);
    }
    config->setLink(link);
    emit connectingLinkNameChanged();
    _setFailedLink(nullptr);
    if (!config->isDynamic()) {
        _setConnectingStalled(false);
        _connectingStallTimer->start();
    }

    return true;
}

void LinkManager::_linkConnected()
{
    const LinkInterface *const link = qobject_cast<LinkInterface*>(sender());
    const SharedLinkConfigurationPtr config = link ? link->linkConfiguration() : nullptr;
    if (config) {
        config->noteConnected();
    }
}

void LinkManager::_communicationError(const QString &title, const QString &error, LinkConfiguration::ErrorRemedy remedy)
{
    const LinkInterface *const link = qobject_cast<LinkInterface*>(sender());
    const SharedLinkConfigurationPtr config = link ? link->linkConfiguration() : nullptr;

    if (config && config->isAutoConnect() && !config->suppressAutoReconnect()) {
        qCDebug(LinkManagerLog) << "Auto-connect link error (will retry):" << title << error;
        return;
    }

    if (!config || config->isDynamic() || link->isConnected()) {
        QGC::showAppMessage(error, title);
        return;
    }
    for (const SharedLinkConfigurationPtr &other : std::as_const(_rgLinkConfigs)) {
        if (other != config) {
            other->setLastError(QString());
        }
    }
    config->setLastError(error, remedy);
    _setFailedLink(config.get());
}

void LinkManager::_setFailedLink(LinkConfiguration *config)
{
    if (config != _failedLink) {
        _failedLink = config;
        emit failedLinkChanged();
    }
}

void LinkManager::setConnectingStallMSecs(int msecs)
{
    _connectingStallMSecs = msecs;
    _connectingStallTimer->setInterval(msecs);
}

void LinkManager::_setConnectingStalled(bool stalled)
{
    if (stalled != _connectingStalled) {
        _connectingStalled = stalled;
        emit connectingStalledChanged();
    }
}

QString LinkManager::connectingLinkName() const
{
    QMutexLocker locker(&_linksMutex);
    for (const SharedLinkInterfacePtr &link : _rgLinks) {
        const SharedLinkConfigurationPtr config = link->linkConfiguration();
        if (config && !config->isDynamic()) {
            return config->name();
        }
    }
    return QString();
}

SharedLinkInterfacePtr LinkManager::mavlinkForwardingLink()
{
    QMutexLocker locker(&_linksMutex);

    for (const SharedLinkInterfacePtr &link : _rgLinks) {
        const SharedLinkConfigurationPtr linkConfig = link->linkConfiguration();
        if (linkConfig && (linkConfig->type() == LinkConfiguration::TypeUdp) && (linkConfig->name() == _mavlinkForwardingLinkName)) {
            return link;
        }
    }

    return nullptr;
}

SharedLinkInterfacePtr LinkManager::mavlinkForwardingSupportLink()
{
    QMutexLocker locker(&_linksMutex);

    for (const SharedLinkInterfacePtr &link : _rgLinks) {
        const SharedLinkConfigurationPtr linkConfig = link->linkConfiguration();
        if (linkConfig && (linkConfig->type() == LinkConfiguration::TypeUdp) && (linkConfig->name() == _mavlinkForwardingSupportLinkName)) {
            return link;
        }
    }

    return nullptr;
}

void LinkManager::disconnectAll()
{
    QList<SharedLinkInterfacePtr> links;
    {
        QMutexLocker locker(&_linksMutex);
        links = _rgLinks;
    }

    for (const SharedLinkInterfacePtr &sharedLink: links) {
        sharedLink->disconnect();
    }
}

void LinkManager::_linkDisconnected()
{
    LinkInterface* const link = qobject_cast<LinkInterface*>(sender());

    if (!link) {
        return;
    }

    SharedLinkInterfacePtr linkToCleanup;
    SharedLinkConfigurationPtr config;
    {
        QMutexLocker locker(&_linksMutex);

        for (auto it = _rgLinks.begin(); it != _rgLinks.end(); ++it) {
            if (it->get() == link) {
                config = it->get()->linkConfiguration();
                const QString linkName = config ? config->name() : QStringLiteral("<null config>");
                qCDebug(LinkManagerLog) << linkName << "use_count:" << it->use_count();
                linkToCleanup = *it;
                (void) _rgLinks.erase(it);
                break;
            }
        }
    }

    if (!linkToCleanup) {
        qCDebug(LinkManagerLog) << "link already removed";
        return;
    }

    if (config) {
        config->noteDisconnected();
        config->setLink(nullptr);
    }

    (void) disconnect(link, &LinkInterface::communicationError, this, &LinkManager::_communicationError);
    (void) disconnect(link, &LinkInterface::bytesReceived, MAVLinkProtocol::instance(), &MAVLinkProtocol::receiveBytes);
    (void) disconnect(link, &LinkInterface::bytesSent, MAVLinkProtocol::instance(), &MAVLinkProtocol::logSentBytes);
    (void) disconnect(link, &LinkInterface::connected, this, &LinkManager::_linkConnected);
    (void) disconnect(link, &LinkInterface::disconnected, this, &LinkManager::_linkDisconnected);

    link->_freeMavlinkChannel();

    if (config && (config->type() == LinkConfiguration::TypeUdp) && (config->name() == _mavlinkForwardingSupportLinkName)) {
        emit mavlinkSupportForwardingEnabledChanged();
    }
    emit connectingLinkNameChanged();
    if (connectingLinkName().isEmpty()) {
        _connectingStallTimer->stop();
        _setConnectingStalled(false);
    }
    if (config && config->isDynamic()) {
        (void) _removeConfiguration(config.get());
    }
}

SharedLinkInterfacePtr LinkManager::sharedLinkInterfacePointerForLink(const LinkInterface *link)
{
    QMutexLocker locker(&_linksMutex);

    for (const SharedLinkInterfacePtr &sharedLink: _rgLinks) {
        if (sharedLink.get() == link) {
            return sharedLink;
        }
    }

    qCDebug(LinkManagerLog) << "link not in list (likely disconnected)";
    return SharedLinkInterfacePtr(nullptr);
}

bool LinkManager::_connectionsSuspendedMsg() const
{
    if (_connectionsSuspended) {
        QGC::showAppMessage(tr("Connect not allowed: %1").arg(_connectionsSuspendedReason));
        return true;
    }

    return false;
}

void LinkManager::saveLinkConfigurationList()
{
    QSettings settings;
    settings.remove(LinkConfiguration::settingsRoot());

    int trueCount = 0;
    for (int i = 0; i < _rgLinkConfigs.count(); i++) {
        SharedLinkConfigurationPtr linkConfig = _rgLinkConfigs[i];
        if (!linkConfig) {
            qCWarning(LinkManagerLog) << "Internal error for link configuration in LinkManager";
            continue;
        }

        if (linkConfig->isDynamic()) {
            continue;
        }

        const QString root = LinkConfiguration::settingsRoot() + QStringLiteral("/Link%1").arg(trueCount++);
        settings.setValue(root + "/name", linkConfig->name());
        settings.setValue(root + "/type", linkConfig->type());
        settings.setValue(root + "/auto", linkConfig->isAutoConnect());
        settings.setValue(root + "/high_latency", linkConfig->isHighLatency());
        linkConfig->saveSettings(settings, root);
    }

    const QString root = QString(LinkConfiguration::settingsRoot());
    settings.setValue(root + "/count", trueCount);
}

void LinkManager::loadLinkConfigurationList()
{
    QSettings settings;
    if (settings.contains(LinkConfiguration::settingsRoot() + "/count")) {
        const int count = settings.value(LinkConfiguration::settingsRoot() + "/count").toInt();
        for (int i = 0; i < count; i++) {
            const QString root = LinkConfiguration::settingsRoot() + QStringLiteral("/Link%1").arg(i);
            if (!settings.contains(root + "/type")) {
                qCWarning(LinkManagerLog) << "Link Configuration" << root << "has no type.";
                continue;
            }

            LinkConfiguration::LinkType type = static_cast<LinkConfiguration::LinkType>(settings.value(root + "/type").toInt());
            if (type >= LinkConfiguration::TypeLast) {
                qCWarning(LinkManagerLog) << "Link Configuration" << root << "an invalid type:" << type;
                continue;
            }

            if (!settings.contains(root + "/name")) {
                qCWarning(LinkManagerLog) << "Link Configuration" << root << "has no name.";
                continue;
            }

            const QString name = settings.value(root + "/name").toString();
            if (name.isEmpty()) {
                qCWarning(LinkManagerLog) << "Link Configuration" << root << "has an empty name.";
                continue;
            }

            LinkConfiguration* link = nullptr;
            switch(type) {
#ifndef QGC_NO_SERIAL_LINK
            case LinkConfiguration::TypeSerial:
                link = new SerialConfiguration(name);
                break;
#endif
            case LinkConfiguration::TypeUdp:
                link = new UDPConfiguration(name);
                break;
            case LinkConfiguration::TypeTcp:
                link = new TCPConfiguration(name);
                break;
            case LinkConfiguration::TypeBluetooth:
                link = new BluetoothConfiguration(name);
                break;
            case LinkConfiguration::TypeLogReplay:
                link = new LogReplayConfiguration(name);
                break;
            case LinkConfiguration::TypeAircastCloud:
                link = new AircastCloudConfiguration(name);
                break;
#ifdef QT_DEBUG
            case LinkConfiguration::TypeMock:
                link = new MockConfiguration(name);
                break;
#endif
            case LinkConfiguration::TypeLast:
            default:
                break;
            }

            if (link) {
                const bool autoConnect = settings.value(root + "/auto").toBool();
                link->setAutoConnect(autoConnect);
                const bool highLatency = settings.value(root + "/high_latency").toBool();
                link->setHighLatency(highLatency);
                link->loadSettings(settings, root);
                addConfiguration(link);
            }
        }
    }

    _configurationsLoaded = true;
}

void LinkManager::_addUDPAutoConnectLink()
{
    if (!_autoConnectSettings->autoConnectUDP()->rawValue().toBool()) {
        return;
    }

    {
        QMutexLocker locker(&_linksMutex);
        for (const SharedLinkInterfacePtr &link : _rgLinks) {
            const SharedLinkConfigurationPtr linkConfig = link->linkConfiguration();
            if (linkConfig && (linkConfig->type() == LinkConfiguration::TypeUdp) && (linkConfig->name() == _defaultUDPLinkName)) {
                return;
            }
        }
    }

    qCDebug(LinkManagerLog) << "New auto-connect UDP port added";
    UDPConfiguration* const udpConfig = new UDPConfiguration(_defaultUDPLinkName);
    udpConfig->setDynamic(true);
    udpConfig->setAutoConnect(true);
    SharedLinkConfigurationPtr config = addConfiguration(udpConfig);
    createConnectedLink(config);
}

void LinkManager::_addMAVLinkForwardingLink()
{
    if (!SettingsManager::instance()->mavlinkSettings()->forwardMavlink()->rawValue().toBool()) {
        return;
    }

    {
        QMutexLocker locker(&_linksMutex);
        for (const SharedLinkInterfacePtr &link : _rgLinks) {
            const SharedLinkConfigurationPtr linkConfig = link->linkConfiguration();
            if (linkConfig && (linkConfig->type() == LinkConfiguration::TypeUdp) && (linkConfig->name() == _mavlinkForwardingLinkName)) {
                return;
            }
        }
    }

    const QString hostName = SettingsManager::instance()->mavlinkSettings()->forwardMavlinkHostName()->rawValue().toString();
    _createDynamicForwardLink(_mavlinkForwardingLinkName, hostName);
}

void LinkManager::_reconnectAutoConnectLinks()
{
    for (SharedLinkConfigurationPtr &config : _rgLinkConfigs) {
        if (!config || config->isDynamic() || !config->isAutoConnect()) {
            continue;
        }

        if (config->link() || config->suppressAutoReconnect() || !config->autoConnectStarted()) {
            continue;
        }

        if (!config->reconnectReady()) {
            continue;
        }

        qCDebug(LinkManagerLog) << "Reconnecting auto-connect link" << config->name();
        config->noteReconnectAttempt();
        createConnectedLink(config);
    }
}

void LinkManager::_updateAutoConnectLinks()
{
    if (_connectionsSuspended) {
        return;
    }

    _addUDPAutoConnectLink();
    _addMAVLinkForwardingLink();
    _reconnectAutoConnectLinks();

    const int nmeaSource = _autoConnectSettings->nmeaSource()->rawValue().toInt();
    if (nmeaSource == AutoConnectSettings::NmeaSourceUdp) {
        if ((_nmeaSocket->localPort() != _autoConnectSettings->nmeaUdpPort()->rawValue().toUInt()) || (_nmeaSocket->state() != UdpIODevice::BoundState)) {
            qCDebug(LinkManagerLog) << "Changing port for UDP NMEA stream";
            _nmeaSocket->close();
            _nmeaSocket->bind(QHostAddress::AnyIPv4, _autoConnectSettings->nmeaUdpPort()->rawValue().toUInt());
            QGCPositionManager::instance()->setNmeaSourceDevice(_nmeaSocket);
        }
    } else {
        _nmeaSocket->close();

        if (nmeaSource == AutoConnectSettings::NmeaSourceDisabled) {
            QGCPositionManager::instance()->resetNmeaSourceDevice();
        }
    }

#ifndef QGC_NO_SERIAL_LINK
    if ((nmeaSource != AutoConnectSettings::NmeaSourceSerial) && _nmeaPort) {
        _nmeaPort->close();
        delete _nmeaPort;
        _nmeaPort = nullptr;
        _nmeaDeviceName = "";
    }

    _addSerialAutoConnectLink();
#endif
}

void LinkManager::shutdown()
{
    setConnectionsSuspended(tr("Shutdown"));
    disconnectAll();

    while (MultiVehicleManager::instance()->vehicles()->count()) {
        QCoreApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
    }
}

namespace {

const QList<QPair<QString, QString>> &linkTypeTable()
{
    static QList<QPair<QString, QString>> table;
    if (!table.isEmpty()) {
        return table;
    }

#ifndef QGC_NO_SERIAL_LINK
    table += qMakePair(QStringLiteral("serial"), LinkManager::tr("Serial"));
#endif
    table += qMakePair(QStringLiteral("udp"), LinkManager::tr("UDP"));
    table += qMakePair(QStringLiteral("tcp"), LinkManager::tr("TCP"));
    table += qMakePair(QStringLiteral("bluetooth"), LinkManager::tr("Bluetooth"));
#ifdef QT_DEBUG
    table += qMakePair(QStringLiteral("mock"), LinkManager::tr("Mock Link"));
#endif
    table += qMakePair(QStringLiteral("logReplay"), LinkManager::tr("Log Replay"));
    table += qMakePair(QStringLiteral("aircastCloud"), LinkManager::tr("Aircast Cloud"));

    return table;
}

}

QStringList LinkManager::linkTypeStrings() const
{
    QStringList list;
    for (const auto &entry: linkTypeTable()) {
        list += entry.second;
    }

    if (list.size() != static_cast<int>(LinkConfiguration::TypeLast)) {
        qCWarning(LinkManagerLog) << "Internal error";
    }

    return list;
}

QStringList LinkManager::linkTypeIds() const
{
    QStringList ids;
    for (const auto &entry: linkTypeTable()) {
        ids += entry.first;
    }

    return ids;
}

void LinkManager::endConfigurationEditing(LinkConfiguration *config, LinkConfiguration *editedConfig)
{
    if (!config || !editedConfig) {
        qCWarning(LinkManagerLog) << "Internal error";
        return;
    }

    config->copyFrom(editedConfig);
    saveLinkConfigurationList();
    emit config->nameChanged(config->name());
    delete editedConfig;
}

void LinkManager::endCreateConfiguration(LinkConfiguration *config)
{
    if (!config) {
        qCWarning(LinkManagerLog) << "Internal error";
        return;
    }

    addConfiguration(config);
    saveLinkConfigurationList();
}

LinkConfiguration *LinkManager::createConfiguration(int type, const QString &name)
{
#ifndef QGC_NO_SERIAL_LINK
    if (static_cast<LinkConfiguration::LinkType>(type) == LinkConfiguration::TypeSerial) {
        _updateSerialPorts();
    }
#endif

    return LinkConfiguration::createSettings(type, name);
}

LinkConfiguration *LinkManager::startConfigurationEditing(LinkConfiguration *config)
{
    if (!config) {
        qCWarning(LinkManagerLog) << "Internal error";
        return nullptr;
    }

#ifndef QGC_NO_SERIAL_LINK
    if (config->type() == LinkConfiguration::TypeSerial) {
        _updateSerialPorts();
    }
#endif

    return LinkConfiguration::duplicateSettings(config);
}

void LinkManager::removeConfiguration(LinkConfiguration *config)
{
    if (!config) {
        qCWarning(LinkManagerLog) << "Internal error";
        return;
    }

    LinkInterface* const link = config->link();
    if (link) {
        link->disconnect();
    }

    if (!_removeConfiguration(config)) {
        qCWarning(LinkManagerLog) << "called with unknown config";
    }
    saveLinkConfigurationList();
}

bool LinkManager::createAndConnectLink(const QString &type, const QString &name, const QString &host, int port)
{
    if (name.isEmpty() || (port <= 0) || (port > 65535)) {
        qCWarning(LinkManagerLog) << "createAndConnectLink: bad name or port" << name << port;
        return false;
    }

    for (const SharedLinkConfigurationPtr &existing : std::as_const(_rgLinkConfigs)) {
        if (existing->name() == name) {
            qCWarning(LinkManagerLog) << "createAndConnectLink: name already in use" << name;
            return false;
        }
    }

    LinkConfiguration *config = nullptr;
    if (type.compare(QStringLiteral("udp"), Qt::CaseInsensitive) == 0) {
        UDPConfiguration *const udpConfig = new UDPConfiguration(name);
        udpConfig->setLocalPort(static_cast<quint16>(port));
        if (!host.isEmpty()) {
            udpConfig->addHost(host, static_cast<quint16>(port));
        }
        config = udpConfig;
    } else if (type.compare(QStringLiteral("tcp"), Qt::CaseInsensitive) == 0) {
        if (host.isEmpty()) {
            qCWarning(LinkManagerLog) << "createAndConnectLink: TCP needs a host";
            return false;
        }
        TCPConfiguration *const tcpConfig = new TCPConfiguration(name);
        tcpConfig->setHost(host);
        tcpConfig->setPort(static_cast<quint16>(port));
        config = tcpConfig;
    } else {
        qCWarning(LinkManagerLog) << "createAndConnectLink: unsupported type" << type;
        return false;
    }

    SharedLinkConfigurationPtr shared = addConfiguration(config);
    saveLinkConfigurationList();
    return createConnectedLink(shared);
}

bool LinkManager::createSerialConfiguration(const QString &name, const QString &portName, int baud)
{
#ifdef QGC_NO_SERIAL_LINK
    Q_UNUSED(name); Q_UNUSED(portName); Q_UNUSED(baud);
    qCWarning(LinkManagerLog) << "createSerialConfiguration: this build has no serial support";
    return false;
#else
    if (name.isEmpty() || portName.isEmpty() || baud <= 0) {
        qCWarning(LinkManagerLog) << "createSerialConfiguration: bad name, port or baud" << name << portName << baud;
        return false;
    }

    for (const SharedLinkConfigurationPtr &existing : std::as_const(_rgLinkConfigs)) {
        if (existing->name() == name) {
            qCWarning(LinkManagerLog) << "createSerialConfiguration: name already in use" << name;
            return false;
        }
    }

    SerialConfiguration *const serialConfig = new SerialConfiguration(name);
    serialConfig->setPortName(portName);
    serialConfig->setBaud(baud);

    addConfiguration(serialConfig);
    saveLinkConfigurationList();
    return true;
#endif
}

void LinkManager::createMavlinkForwardingSupportLink()
{
    const QString hostName = SettingsManager::instance()->mavlinkSettings()->forwardMavlinkAPMSupportHostName()->rawValue().toString();
    _createDynamicForwardLink(_mavlinkForwardingSupportLinkName, hostName);
    emit mavlinkSupportForwardingEnabledChanged();
}

void LinkManager::endMavlinkForwardingSupportLink()
{
    const SharedLinkInterfacePtr link = mavlinkForwardingSupportLink();
    if (link) {
        link->disconnect();
    }
}

bool LinkManager::_removeConfiguration(const LinkConfiguration *config)
{
    if (config == _failedLink) {
        _setFailedLink(nullptr);
    }
    (void) _qmlConfigurations->removeOne(config);

    return _rgLinkConfigs.removeIf([config](const SharedLinkConfigurationPtr &candidate) {
        return candidate.get() == config;
    }) > 0;
}

bool LinkManager::isBluetoothAvailable()
{
    return QGCNetworkHelper::isBluetoothAvailable();
}

bool LinkManager::containsLink(const LinkInterface *link)
{
    QMutexLocker locker(&_linksMutex);

    for (const SharedLinkInterfacePtr &sharedLink : _rgLinks) {
        if (sharedLink.get() == link) {
            return true;
        }
    }

    return false;
}

SharedLinkConfigurationPtr LinkManager::addConfiguration(LinkConfiguration *config)
{
    (void) _qmlConfigurations->append(config);
    (void) _rgLinkConfigs.append(SharedLinkConfigurationPtr(config));

    return _rgLinkConfigs.last();
}

void LinkManager::startAutoConnectedLinks()
{
    for (SharedLinkConfigurationPtr &sharedConfig : _rgLinkConfigs) {
        if (sharedConfig->isAutoConnect()) {
            sharedConfig->setAutoConnectStarted(true);
            createConnectedLink(sharedConfig);
        }
    }
}

uint8_t LinkManager::allocateMavlinkChannel()
{
    for (uint8_t mavlinkChannel = 0; mavlinkChannel < MAVLINK_COMM_NUM_BUFFERS; mavlinkChannel++) {
        if (_mavlinkChannelsUsedBitMask & (1 << mavlinkChannel)) {
            continue;
        }

        mavlink_reset_channel_status(mavlinkChannel);
        mavlink_status_t* const mavlinkStatus = mavlink_get_channel_status(mavlinkChannel);
        mavlinkStatus->flags |= MAVLINK_STATUS_FLAG_OUT_MAVLINK1;
        _mavlinkChannelsUsedBitMask |= (1 << mavlinkChannel);
        qCDebug(LinkManagerLog) << "allocateMavlinkChannel" << mavlinkChannel;
        return mavlinkChannel;
    }

    qCWarning(LinkManagerLog) << "allocateMavlinkChannel: all channels reserved!";
    return invalidMavlinkChannel();
}

void LinkManager::freeMavlinkChannel(uint8_t channel)
{
    qCDebug(LinkManagerLog) << "freeMavlinkChannel" << channel;

    if (invalidMavlinkChannel() == channel) {
        return;
    }

    _mavlinkChannelsUsedBitMask &= ~(1 << channel);
}

LogReplayLink *LinkManager::startLogReplay(const QString &logFile)
{
    LogReplayConfiguration* const linkConfig = new LogReplayConfiguration(tr("Log Replay"));
    linkConfig->setLogFilename(logFile);
    linkConfig->setName(linkConfig->logFilenameShort());

    SharedLinkConfigurationPtr sharedConfig = addConfiguration(linkConfig);
    if (createConnectedLink(sharedConfig)) {
        return qobject_cast<LogReplayLink*>(sharedConfig->link());
    }

    return nullptr;
}

void LinkManager::_createDynamicForwardLink(const char *linkName, const QString &hostName)
{
    UDPConfiguration* const udpConfig = new UDPConfiguration(linkName);

    udpConfig->setDynamic(true);
    udpConfig->setForwarding(true);
    udpConfig->addHost(hostName);

    SharedLinkConfigurationPtr config = addConfiguration(udpConfig);
    createConnectedLink(config);

    qCDebug(LinkManagerLog) << "New dynamic MAVLink forwarding port added:" << linkName << " hostname:" << hostName;
}

bool LinkManager::isLinkUSBDirect([[maybe_unused]] const LinkInterface *link)
{
#ifndef QGC_NO_SERIAL_LINK
    const SerialLink* const serialLink = qobject_cast<const SerialLink*>(link);
    if (!serialLink) {
        return false;
    }

    const SharedLinkConfigurationPtr config = serialLink->linkConfiguration();
    if (!config) {
        return false;
    }

    const SerialConfiguration* const serialConfig = qobject_cast<const SerialConfiguration*>(config.get());
    if (serialConfig && serialConfig->usbDirect()) {
        return link;
    }
#endif

    return false;
}

#ifndef QGC_NO_SERIAL_LINK // Serial Only Functions

void LinkManager::_filterCompositePorts(QList<QGCSerialPortInfo> &portList)
{
    typedef QPair<quint16, quint16> VidPidPair_t;

    QMap<VidPidPair_t, QStringList> seenSerialNumbers;

    for (auto it = portList.begin(); it != portList.end();) {
        const QGCSerialPortInfo &portInfo = *it;
        if (portInfo.hasVendorIdentifier() && portInfo.hasProductIdentifier() && !portInfo.serialNumber().isEmpty() && portInfo.serialNumber() != "0") {
            VidPidPair_t vidPid(portInfo.vendorIdentifier(), portInfo.productIdentifier());
            if (seenSerialNumbers.contains(vidPid) && seenSerialNumbers[vidPid].contains(portInfo.serialNumber())) {
                if(!portInfo.description().contains("NMEA")) {
                    qCDebug(LinkManagerVerboseLog) << QStringLiteral("Removing secondary port on same device - port:%1 vid:%2 pid%3 sn:%4").arg(portInfo.portName()).arg(portInfo.vendorIdentifier()).arg(portInfo.productIdentifier()).arg(portInfo.serialNumber());
                    it = portList.erase(it);
                    continue;
                }
            }
            seenSerialNumbers[vidPid].append(portInfo.serialNumber());
        }
        it++;
    }
}

void LinkManager::_addSerialAutoConnectLink()
{
    QList<QGCSerialPortInfo> portList;
#ifdef Q_OS_ANDROID
    if (AndroidSerial::usePosixSerial() || !_isSerialPortConnected()) {
        portList = QGCSerialPortInfo::availablePorts();
    }
#else
    portList = QGCSerialPortInfo::availablePorts();
#endif

    _filterCompositePorts(portList);

    QStringList currentPorts;
    for (const QGCSerialPortInfo &portInfo: portList) {
        qCDebug(LinkManagerVerboseLog) << "-----------------------------------------------------";
        qCDebug(LinkManagerVerboseLog) << "portName:          " << portInfo.portName();
        qCDebug(LinkManagerVerboseLog) << "systemLocation:    " << portInfo.systemLocation();
        qCDebug(LinkManagerVerboseLog) << "description:       " << portInfo.description();
        qCDebug(LinkManagerVerboseLog) << "manufacturer:      " << portInfo.manufacturer();
        qCDebug(LinkManagerVerboseLog) << "serialNumber:      " << portInfo.serialNumber();
        qCDebug(LinkManagerVerboseLog) << "vendorIdentifier:  " << portInfo.vendorIdentifier();
        qCDebug(LinkManagerVerboseLog) << "productIdentifier: " << portInfo.productIdentifier();

        currentPorts << portInfo.systemLocation();

        QGCSerialPortInfo::BoardType_t boardType;
        QString boardName;

        if ((_autoConnectSettings->nmeaSource()->rawValue().toInt() == AutoConnectSettings::NmeaSourceSerial) &&
                (portInfo.systemLocation().trimmed() == _autoConnectSettings->autoConnectNmeaPort()->cookedValueString())) {
            if (portInfo.systemLocation().trimmed() != _nmeaDeviceName) {
                _nmeaDeviceName = portInfo.systemLocation().trimmed();
                qCDebug(LinkManagerLog) << "Configuring nmea port" << _nmeaDeviceName;
                QSerialPort* newPort = new QSerialPort(portInfo, this);
                _nmeaBaud = _autoConnectSettings->autoConnectNmeaBaud()->cookedValue().toUInt();
                newPort->setBaudRate(static_cast<qint32>(_nmeaBaud));
                qCDebug(LinkManagerLog) << "Configuring nmea baudrate" << _nmeaBaud;
                QGCPositionManager::instance()->setNmeaSourceDevice(newPort);
                if (_nmeaPort) {
                    delete _nmeaPort;
                }
                _nmeaPort = newPort;
            } else if (_autoConnectSettings->autoConnectNmeaBaud()->cookedValue().toUInt() != _nmeaBaud) {
                _nmeaBaud = _autoConnectSettings->autoConnectNmeaBaud()->cookedValue().toUInt();
                _nmeaPort->setBaudRate(static_cast<qint32>(_nmeaBaud));
                qCDebug(LinkManagerLog) << "Configuring nmea baudrate" << _nmeaBaud;
            }
        } else if (portInfo.getBoardInfo(boardType, boardName)) {
            if (!_allowAutoConnectToBoard(boardType)) {
                continue;
            }

            if (portInfo.isBootloader()) {
                qCDebug(LinkManagerLog) << "Waiting for bootloader to finish" << portInfo.systemLocation();
                continue;
            }
            if (_portAlreadyConnected(portInfo.systemLocation()) || (_autoConnectRTKPort == portInfo.systemLocation())) {
                qCDebug(LinkManagerVerboseLog) << "Skipping existing autoconnect" << portInfo.systemLocation();
            } else if (!_autoconnectPortWaitList.contains(portInfo.systemLocation())) {
                qCDebug(LinkManagerLog) << "Waiting for next autoconnect pass" << portInfo.systemLocation() << boardName;
                _autoconnectPortWaitList[portInfo.systemLocation()] = 1;
            } else if ((++_autoconnectPortWaitList[portInfo.systemLocation()] * _autoconnectUpdateTimerMSecs) > _autoconnectConnectDelayMSecs) {
                SerialConfiguration* pSerialConfig = nullptr;
                _autoconnectPortWaitList.remove(portInfo.systemLocation());
                switch (boardType) {
                case QGCSerialPortInfo::BoardTypePixhawk:
                    pSerialConfig = new SerialConfiguration(tr("%1 on %2 (AutoConnect)").arg(boardName, portInfo.portName().trimmed()));
                    pSerialConfig->setUsbDirect(true);
                    break;
                case QGCSerialPortInfo::BoardTypeSiKRadio:
                    pSerialConfig = new SerialConfiguration(tr("%1 on %2 (AutoConnect)").arg(boardName, portInfo.portName().trimmed()));
                    break;
                case QGCSerialPortInfo::BoardTypeOpenPilot:
                    pSerialConfig = new SerialConfiguration(tr("%1 on %2 (AutoConnect)").arg(boardName, portInfo.portName().trimmed()));
                    break;
                case QGCSerialPortInfo::BoardTypeRTKGPS:
                    qCDebug(LinkManagerLog) << "RTK GPS auto-connected" << portInfo.portName().trimmed();
                    _autoConnectRTKPort = portInfo.systemLocation();
                    GPSManager::instance()->gpsRtk()->connectGPS(portInfo.systemLocation(), boardName);
                    break;
                default:
                    qCWarning(LinkManagerLog) << "Internal error: Unknown board type" << boardType;
                    continue;
                }

                if (pSerialConfig) {
                    qCDebug(LinkManagerLog) << "New auto-connect port added: " << pSerialConfig->name() << portInfo.systemLocation();
                    pSerialConfig->setBaud((boardType == QGCSerialPortInfo::BoardTypeSiKRadio) ? 57600 : 115200);
                    pSerialConfig->setDynamic(true);
                    pSerialConfig->setPortName(portInfo.systemLocation());
                    pSerialConfig->setAutoConnect(true);

                    SharedLinkConfigurationPtr sharedConfig(pSerialConfig);
                    createConnectedLink(sharedConfig);
                }
            }
        }
    }

    if (!_autoConnectRTKPort.isEmpty() && !currentPorts.contains(_autoConnectRTKPort)) {
        qCDebug(LinkManagerLog) << "RTK GPS disconnected" << _autoConnectRTKPort;
        GPSManager::instance()->gpsRtk()->disconnectGPS();
        _autoConnectRTKPort.clear();
    }
}

bool LinkManager::_allowAutoConnectToBoard(QGCSerialPortInfo::BoardType_t boardType) const
{
    switch (boardType) {
    case QGCSerialPortInfo::BoardTypePixhawk:
        if (_autoConnectSettings->autoConnectPixhawk()->rawValue().toBool()) {
            return true;
        }
        break;
    case QGCSerialPortInfo::BoardTypeSiKRadio:
        if (_autoConnectSettings->autoConnectSiKRadio()->rawValue().toBool()) {
            return true;
        }
        break;
    case QGCSerialPortInfo::BoardTypeOpenPilot:
        if (_autoConnectSettings->autoConnectLibrePilot()->rawValue().toBool()) {
            return true;
        }
        break;
    case QGCSerialPortInfo::BoardTypeRTKGPS:
        if (_autoConnectSettings->autoConnectRTKGPS()->rawValue().toBool() && !GPSManager::instance()->gpsRtk()->connected()) {
            return true;
        }
        break;
    default:
        qCWarning(LinkManagerLog) << "Internal error: Unknown board type" << boardType;
        return false;
    }

    return false;
}

bool LinkManager::_portAlreadyConnected(const QString &portName)
{
    QMutexLocker locker(&_linksMutex);

    const QString searchPort = portName.trimmed();
    for (const SharedLinkInterfacePtr &linkInterface : _rgLinks) {
        const SharedLinkConfigurationPtr linkConfig = linkInterface->linkConfiguration();
        const SerialConfiguration* const serialConfig = qobject_cast<const SerialConfiguration*>(linkConfig.get());
        if (serialConfig && (serialConfig->portName() == searchPort)) {
            return true;
        }
    }

    return false;
}

void LinkManager::_updateSerialPorts()
{
    _commPortList.clear();
    _commPortDisplayList.clear();
    const QList<QGCSerialPortInfo> portList = QGCSerialPortInfo::availablePorts();
    for (const QGCSerialPortInfo &info: portList) {
        const QString port = info.systemLocation().trimmed();
        _commPortList += port;
        _commPortDisplayList += SerialConfiguration::cleanPortDisplayName(port);
    }
}

QStringList LinkManager::serialPortStrings()
{
    if (_commPortDisplayList.isEmpty()) {
        _updateSerialPorts();
    }

    return _commPortDisplayList;
}

QStringList LinkManager::serialPorts()
{
    if (_commPortList.isEmpty()) {
        _updateSerialPorts();
    }

    return _commPortList;
}

QStringList LinkManager::serialBaudRates()
{
    return SerialConfiguration::supportedBaudRates();
}

bool LinkManager::_isSerialPortConnected()
{
    QMutexLocker locker(&_linksMutex);

    for (const SharedLinkInterfacePtr &link: _rgLinks) {
        if (qobject_cast<const SerialLink*>(link.get())) {
            return true;
        }
    }

    return false;
}

#endif // QGC_NO_SERIAL_LINK
