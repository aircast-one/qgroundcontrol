/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "DebugApiServer.h"
#include <QtGui/QPointingDevice>
#include <QtTest/QTest>
#include <QtCore/QCoreApplication>
#include <QtCore/QScopeGuard>
#include <QtCore/QEventLoop>
#include "LinkManager.h"
#include "MockLink.h"
#include "MultiVehicleManager.h"
#include "ParameterManager.h"
#include "PlanMasterController.h"
#include "QmlObjectListModel.h"
#include "TCPLink.h"
#include "QGCApplication.h"
#include "QGCLoggingCategory.h"
#include "QGroundControlQmlGlobal.h"
#include "SettingsManager.h"
#include "Vehicle.h"
#include "VideoManager.h"
#include "VideoSettings.h"
#include "Fact.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QSettings>
#include <QtCore/QStandardPaths>
#include <QtCore/QUrl>
#include <QtCore/QUrlQuery>
#include <QtCore/QFile>
#include <QtCore/QLoggingCategory>
#include <QtCore/QMetaProperty>
#include <QtCore/QTextStream>
#include <QtGui/QKeySequence>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QScreen>
#include <QtNetwork/QHostAddress>
#include <QtNetwork/QTcpServer>
#include <QtNetwork/QTcpSocket>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <qpa/qwindowsysteminterface.h>

#include <algorithm>
#include <functional>
#include <optional>

QGC_LOGGING_CATEGORY(DebugApiServerLog, "qgc.debugapi.debugapiserver")

// Leading marker byte on a handler error body. It cannot occur in JSON text output, so
// _handleConnection maps a marked body to HTTP 400 unambiguously (rather than sniffing the
// JSON shape) and strips the byte before writing the response.
static constexpr char kErrorMarker = '\x01';

static constexpr int kMinWindowWidth      = 320;
static constexpr int kMinWindowHeight     = 240;
static constexpr int kDefaultGestureSteps = 12;
static constexpr int kMinGestureSteps     = 2;
static constexpr int kMaxGestureSteps     = 100;
static constexpr int kEventSliceMSecs     = 5;
static constexpr int kResizeSettleMSecs   = 50;

class TouchCounter : public QObject
{
public:
    int begin = 0, update = 0, end = 0;

protected:
    bool eventFilter(QObject *watched, QEvent *event) override
    {
        switch (event->type()) {
        case QEvent::TouchBegin:  begin++;  break;
        case QEvent::TouchUpdate: update++; break;
        case QEvent::TouchEnd:    end++;    break;
        default: break;
        }
        return QObject::eventFilter(watched, event);
    }
};

static QPointingDevice *_touchDevice()
{
    static QPointingDevice *device = QTest::createTouchDevice();
    return device;
}

DebugApiServer *DebugApiServer::_instance = nullptr;

void DebugApiServer::startIfConfigured(QObject *parent)
{
    bool ok = false;
    const uint port = qEnvironmentVariableIntValue("QGC_DEBUG_API_PORT", &ok);
    if (!ok || port == 0 || port > 65535) {
        return;
    }
    start(static_cast<quint16>(port), parent);
}

void DebugApiServer::start(quint16 port, QObject *parent)
{
    if (_instance) {
        qCDebug(DebugApiServerLog) << "debug api already running on port" << _instance->serverPort();
        return;
    }
    _instance = new DebugApiServer(port, parent);
}

DebugApiServer::DebugApiServer(quint16 port, QObject *parent)
    : QObject(parent)
    , _server(new QTcpServer(this))
{
    if (!_server->listen(QHostAddress::LocalHost, port)) {
        qCWarning(DebugApiServerLog) << "listen failed on port" << port << _server->errorString();
        return;
    }
    qCDebug(DebugApiServerLog) << "debug api listening on 127.0.0.1:" << port;

    (void) connect(MultiVehicleManager::instance(), &MultiVehicleManager::activeVehicleChanged, this, [this](Vehicle *vehicle) {
        // Re-activating a vehicle must not stack duplicate connections.
        QObject::disconnect(_messageConnection);
        QObject::disconnect(_rcConnection);
        if (!vehicle) {
            return;
        }
        _messageConnection = connect(vehicle, &Vehicle::textMessageReceived, this, [this](int, int, int severity, const QString &text, const QString &) {
            _messages.append(QStringLiteral("[sev%1] %2").arg(severity).arg(text));
            while (_messages.size() > 200) {
                _messages.removeFirst();
            }
        });
        _rcConnection = connect(vehicle, &Vehicle::rcChannelsChanged, this, [this](int channelCount, int *pwmValues) {
            _rcValues.clear();
            for (int i = 0; i < channelCount; ++i) {
                _rcValues.append(pwmValues[i]);
            }
        });
    });

    (void) connect(_server, &QTcpServer::newConnection, this, [this]() {
        while (QTcpSocket *socket = _server->nextPendingConnection()) {
            _handleConnection(socket);
        }
    });
}

quint16 DebugApiServer::serverPort() const
{
    return _server->serverPort();
}

void DebugApiServer::_handleConnection(QTcpSocket *socket)
{
    (void) connect(socket, &QTcpSocket::disconnected, socket, &QObject::deleteLater);
    (void) connect(socket, &QTcpSocket::readyRead, this, [this, socket]() {
        socket->setProperty("request", socket->property("request").toByteArray() + socket->readAll());
        const QByteArray request = socket->property("request").toByteArray();
        constexpr int kMaxRequestBytes = 8192;
        if (request.size() > kMaxRequestBytes) {
            socket->abort();
            return;
        }
        if (!request.contains("\r\n\r\n")) {
            return;
        }

        const QList<QByteArray> requestLine = request.left(request.indexOf("\r\n")).split(' ');
        QByteArray body = QByteArrayLiteral("{\"error\":\"bad request\"}");
        QByteArray statusLine = QByteArrayLiteral("HTTP/1.1 400 Bad Request");
        const QByteArray headers = request.left(request.indexOf("\r\n\r\n")).toLower();
        if (!headers.contains("\r\nx-qgc-debug-api:")) {
            // Browsers cannot attach custom headers without a CORS preflight (which this
            // server never grants), so requiring one kills drive-by requests from web pages.
            body = QByteArrayLiteral("{\"error\":\"missing X-QGC-Debug-Api header\"}");
            statusLine = QByteArrayLiteral("HTTP/1.1 403 Forbidden");
        } else if (requestLine.size() >= 2 && requestLine.at(0) == "GET") {
            const QUrl url = QUrl::fromEncoded(requestLine.at(1));
            body = _route(url.path(), QUrlQuery(url));
            if (body.isEmpty()) {
                statusLine = QByteArrayLiteral("HTTP/1.1 404 Not Found");
                body = QByteArrayLiteral("{\"error\":\"not found\"}");
            } else if (body.startsWith(kErrorMarker)) {
                statusLine = QByteArrayLiteral("HTTP/1.1 400 Bad Request");
                body.remove(0, 1);
            } else {
                statusLine = QByteArrayLiteral("HTTP/1.1 200 OK");
            }
        }

        QByteArray response = statusLine + "\r\nContent-Type: application/json\r\nContent-Length: " +
                              QByteArray::number(body.size()) + "\r\nConnection: close\r\n\r\n" + body;
        socket->write(response);
        socket->disconnectFromHost();
    });
}

static QByteArray _errorJson(const QString &message)
{
    return kErrorMarker + QJsonDocument(QJsonObject{{"error", message}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_route(const QString &path, const QUrlQuery &query)
{
    static bool routeBusy = false;
    if (routeBusy) {
        return _errorJson(QStringLiteral("busy: another request is still being processed"));
    }
    routeBusy = true;
    const auto busyGuard = qScopeGuard([] { routeBusy = false; });

    VideoManager *videoManager = VideoManager::instance();

    if (path == QStringLiteral("/status")) {
        return _statusJson();
    }
    if (path == QStringLiteral("/switch")) {
        if (query.hasQueryItem(QStringLiteral("index"))) {
            bool ok = false;
            const int index = query.queryItemValue(QStringLiteral("index")).toInt(&ok);
            if (!ok) {
                return _errorJson(QStringLiteral("index must be a number"));
            }
            videoManager->setActiveVideoSource(index);
        } else {
            videoManager->switchActiveVideoSource();
        }
        return _statusJson();
    }
    if (path == QStringLiteral("/grab")) {
        videoManager->grabImage();
        return QJsonDocument(QJsonObject{{"imageFile", videoManager->imageFile()}}).toJson(QJsonDocument::Compact);
    }
    if (path == QStringLiteral("/record")) {
        if (query.queryItemValue(QStringLiteral("on")) == QStringLiteral("1")) {
            videoManager->startRecording();
        } else {
            videoManager->stopRecording();
        }
        return _statusJson();
    }
    if (path == QStringLiteral("/screenshot")) {
        return _screenshotJson();
    }
    if (path == QStringLiteral("/vehicle")) {
        return _vehicleJson();
    }
    if (path == QStringLiteral("/vehicle/params")) {
        return _paramsJson(query);
    }
    if (path == QStringLiteral("/vehicle/params/set")) {
        return _paramSetJson(query);
    }
    if (path == QStringLiteral("/vehicle/params/save")) {
        return _paramsFileJson(query, true);
    }
    if (path == QStringLiteral("/vehicle/params/load")) {
        return _paramsFileJson(query, false);
    }
    if (path == QStringLiteral("/vehicle/calibrate")) {
        return _calibrateJson(query);
    }
    if (path == QStringLiteral("/vehicle/motortest")) {
        return _motorTestJson(query);
    }
    if (path == QStringLiteral("/vehicle/messages")) {
        return _messagesJson();
    }
    if (path == QStringLiteral("/vehicle/rc")) {
        return _rcJson();
    }
    if (path == QStringLiteral("/mission/upload")) {
        return _missionJson(query, true);
    }
    if (path == QStringLiteral("/mission/download")) {
        return _missionJson(query, false);
    }
    if (path == QStringLiteral("/ui/tree")) {
        return _uiTreeJson(query);
    }
    if (path == QStringLiteral("/ui/click")) {
        return _uiClickJson(query, false);
    }
    if (path == QStringLiteral("/ui/doubleclick")) {
        return _uiClickJson(query, true);
    }
    if (path == QStringLiteral("/ui/drag")) {
        return _uiDragJson(query);
    }
    if (path == QStringLiteral("/ui/hover")) {
        return _uiHoverJson(query);
    }
    if (path == QStringLiteral("/ui/type")) {
        return _uiTypeJson(query);
    }
    if (path == QStringLiteral("/ui/key")) {
        return _uiKeyJson(query);
    }
    if (path == QStringLiteral("/logging")) {
        return _loggingJson(query);
    }
    if (path == QStringLiteral("/links")) {
        return _linksJson();
    }
    if (path == QStringLiteral("/links/connect")) {
        return _linkConnectJson(query);
    }
    if (path == QStringLiteral("/links/disconnect")) {
        return _linkDisconnectJson(query);
    }
    if (path == QStringLiteral("/links/mocklink")) {
        return _mockLinkJson(query);
    }
    if (path == QStringLiteral("/ui/dismiss")) {
        return _uiDismissJson();
    }
    if (path == QStringLiteral("/ui/press")) {
        return _uiMouseStepJson(query, QEvent::MouseButtonPress);
    }
    if (path == QStringLiteral("/ui/move")) {
        return _uiMouseStepJson(query, QEvent::MouseMove);
    }
    if (path == QStringLiteral("/ui/release")) {
        return _uiMouseStepJson(query, QEvent::MouseButtonRelease);
    }
    if (path == QStringLiteral("/ui/pinch")) {
        return _uiPinchJson(query);
    }
    if (path == QStringLiteral("/ui/tap")) {
        return _uiTapJson(query);
    }
    if (path == QStringLiteral("/ui/prop")) {
        return _uiPropJson(query);
    }
    if (path == QStringLiteral("/ui/resize")) {
        return _uiResizeJson(query);
    }
    if (path == QStringLiteral("/video/setting")) {
        return _videoSettingJson(query);
    }
    return QByteArray();
}

// Walks the visual (childItems) hierarchy: QML reparenting (pip swaps, Repeater delegates)
// detaches the QObject parent chain, so findChildren() misses items the user can see.
static QQuickItem *_findVisibleItem(QQuickWindow *window, const QString &objectName)
{
    QQuickItem *fallback = nullptr;
    std::function<QQuickItem*(QQuickItem*)> walk = [&](QQuickItem *item) -> QQuickItem* {
        if (item->objectName() == objectName) {
            if (item->isVisible()) {
                return item;
            }
            if (!fallback) {
                fallback = item;
            }
        }
        const QList<QQuickItem*> children = item->childItems();
        for (QQuickItem *child : children) {
            if (QQuickItem *found = walk(child)) {
                return found;
            }
        }
        return nullptr;
    };
    QQuickItem *found = walk(window->contentItem());
    return found ? found : fallback;
}

QByteArray DebugApiServer::_uiTreeJson(const QUrlQuery &query)
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return QJsonDocument(QJsonObject{{"error", "no main window"}}).toJson(QJsonDocument::Compact);
    }

    const QString filter = query.queryItemValue(QStringLiteral("name"));
    constexpr int kMaxItems = 300;
    QJsonArray items;

    QStringList namedAncestors;
    std::function<void(QQuickItem*)> walk = [&](QQuickItem *item) {
        if (items.size() >= kMaxItems) {
            return;
        }
        const QString name = item->objectName();
        const bool named = !name.isEmpty();
        if (named && (filter.isEmpty() || name.contains(filter, Qt::CaseInsensitive))) {
            const QPointF scenePos = item->mapToScene(QPointF(0, 0));
            items.append(QJsonObject{
                {"objectName", name},
                {"namedAncestors", QJsonArray::fromStringList(namedAncestors)},
                {"class", QString::fromLatin1(item->metaObject()->className())},
                {"x", scenePos.x()},
                {"y", scenePos.y()},
                {"width", item->width()},
                {"height", item->height()},
                {"visible", item->isVisible()},
                {"enabled", item->isEnabled()},
            });
        }
        if (named) {
            namedAncestors.append(name);
        }
        const QList<QQuickItem*> children = item->childItems();
        for (QQuickItem *child : children) {
            walk(child);
        }
        if (named) {
            namedAncestors.removeLast();
        }
    };
    walk(window->contentItem());

    return QJsonDocument(QJsonObject{{"items", items}, {"truncated", items.size() >= kMaxItems}}).toJson(QJsonDocument::Compact);
}

// Resolves `<prefix>name` (visible item center) or `<prefix>x`/`<prefix>y` to scene coordinates.
static bool _resolvePoint(QQuickWindow *window, const QUrlQuery &query, const QString &prefix, QPointF &scenePos, QByteArray &error)
{
    const QString nameKey = prefix.isEmpty() ? QStringLiteral("name") : (prefix + QStringLiteral("Name"));
    const QString xKey = prefix.isEmpty() ? QStringLiteral("x") : (prefix + QStringLiteral("X"));
    const QString yKey = prefix.isEmpty() ? QStringLiteral("y") : (prefix + QStringLiteral("Y"));

    const QString name = query.queryItemValue(nameKey);
    if (!name.isEmpty()) {
        QQuickItem *item = _findVisibleItem(window, name);
        if (!item) {
            error = _errorJson(QStringLiteral("item not found: %1").arg(name));
            return false;
        }
        if (!item->isVisible()) {
            error = _errorJson(QStringLiteral("item exists but is not visible: %1").arg(name));
            return false;
        }
        scenePos = item->mapToScene(QPointF(item->width() / 2, item->height() / 2));
        return true;
    }
    if (query.hasQueryItem(xKey) && query.hasQueryItem(yKey)) {
        scenePos = QPointF(query.queryItemValue(xKey).toDouble(), query.queryItemValue(yKey).toDouble());
        return true;
    }
    error = _errorJson(QStringLiteral("%1 or %2/%3 required").arg(nameKey, xKey, yKey));
    return false;
}

// Injection goes through the QPA layer (what QTest uses) so Quick's pointer delivery treats
// it exactly like real input; directly posted QMouseEvents are ignored by it.
static void _mouse(QQuickWindow *window, const QPointF &scenePos, Qt::MouseButtons buttons, Qt::MouseButton button, QEvent::Type type)
{
    QWindowSystemInterface::handleMouseEvent(window, scenePos, window->mapToGlobal(scenePos), buttons, button, type);
}

QByteArray DebugApiServer::_uiClickJson(const QUrlQuery &query, bool doubleClick)
{
    const QString buttonName = query.queryItemValue(QStringLiteral("button"));
    Qt::MouseButton button = Qt::LeftButton;
    if (!buttonName.isEmpty()) {
        if (buttonName == QStringLiteral("right")) {
            button = Qt::RightButton;
        } else if (buttonName != QStringLiteral("left")) {
            return _errorJson(QStringLiteral("button must be left or right, got: %1").arg(buttonName));
        }
    }

    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    QPointF scenePos;
    QByteArray error;
    if (!_resolvePoint(window, query, QString(), scenePos, error)) {
        return error;
    }

    const int clicks = doubleClick ? 2 : 1;
    for (int i = 0; i < clicks; ++i) {
        _mouse(window, scenePos, button, button, QEvent::MouseButtonPress);
        _mouse(window, scenePos, Qt::NoButton, button, QEvent::MouseButtonRelease);
    }
    QWindowSystemInterface::flushWindowSystemEvents();

    return QJsonDocument(QJsonObject{{"clicked", clicks}, {"x", scenePos.x()}, {"y", scenePos.y()}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_uiDragJson(const QUrlQuery &query)
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    QPointF from;
    QPointF to;
    QByteArray error;
    if (!_resolvePoint(window, query, QString(), from, error)) {
        return error;
    }
    if (!_resolvePoint(window, query, QStringLiteral("to"), to, error)) {
        return error;
    }

    int steps = 10;
    if (query.hasQueryItem(QStringLiteral("steps"))) {
        bool ok = false;
        steps = query.queryItemValue(QStringLiteral("steps")).toInt(&ok);
        if (!ok) {
            return _errorJson(QStringLiteral("steps must be a number"));
        }
    }
    steps = qBound(2, steps, 100);
    _mouse(window, from, Qt::LeftButton, Qt::LeftButton, QEvent::MouseButtonPress);
    for (int i = 1; i <= steps; ++i) {
        const QPointF pos = from + (to - from) * i / steps;
        _mouse(window, pos, Qt::LeftButton, Qt::NoButton, QEvent::MouseMove);
    }
    _mouse(window, to, Qt::NoButton, Qt::LeftButton, QEvent::MouseButtonRelease);

    return QJsonDocument(QJsonObject{
        {"dragged", true},
        {"fromX", from.x()}, {"fromY", from.y()},
        {"toX", to.x()}, {"toY", to.y()},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_uiHoverJson(const QUrlQuery &query)
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    QPointF scenePos;
    QByteArray error;
    if (!_resolvePoint(window, query, QString(), scenePos, error)) {
        return error;
    }

    _mouse(window, scenePos, Qt::NoButton, Qt::NoButton, QEvent::MouseMove);
    QWindowSystemInterface::flushWindowSystemEvents();

    return QJsonDocument(QJsonObject{{"hovering", true}, {"x", scenePos.x()}, {"y", scenePos.y()}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_uiTypeJson(const QUrlQuery &query)
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    const QString text = query.queryItemValue(QStringLiteral("text"), QUrl::FullyDecoded);
    if (text.isEmpty()) {
        return _errorJson(QStringLiteral("text required"));
    }
    for (const QChar &character : text) {
        QWindowSystemInterface::handleKeyEvent(window, QEvent::KeyPress, 0, Qt::NoModifier, QString(character));
        QWindowSystemInterface::handleKeyEvent(window, QEvent::KeyRelease, 0, Qt::NoModifier, QString(character));
    }
    QWindowSystemInterface::flushWindowSystemEvents();

    return QJsonDocument(QJsonObject{{"typed", text}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_uiKeyJson(const QUrlQuery &query)
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    const QString keyName = query.queryItemValue(QStringLiteral("key"), QUrl::FullyDecoded);
    const QKeySequence sequence = QKeySequence::fromString(keyName);
    if (sequence.count() != 1) {
        return _errorJson(QStringLiteral("unknown key: %1").arg(keyName));
    }
    const QKeyCombination combination = sequence[0];
    QWindowSystemInterface::handleKeyEvent(window, QEvent::KeyPress, combination.key(), combination.keyboardModifiers());
    QWindowSystemInterface::handleKeyEvent(window, QEvent::KeyRelease, combination.key(), combination.keyboardModifiers());
    QWindowSystemInterface::flushWindowSystemEvents();

    return QJsonDocument(QJsonObject{{"key", keyName}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_loggingJson(const QUrlQuery &query)
{
    const QString rules = query.queryItemValue(QStringLiteral("rules"), QUrl::FullyDecoded);
    if (rules.isEmpty()) {
        return _errorJson(QStringLiteral("rules required, e.g. qgc.videomanager.*.debug=true"));
    }
    QLoggingCategory::setFilterRules(QString(rules).replace(QLatin1Char(';'), QLatin1Char('\n')));
    return QJsonDocument(QJsonObject{{"rules", rules}}).toJson(QJsonDocument::Compact);
}

static LinkConfiguration *_findLinkConfiguration(const QString &name)
{
    QmlObjectListModel *configurations = LinkManager::instance()->linkConfigurations();
    for (int i = 0; configurations && i < configurations->count(); ++i) {
        LinkConfiguration *config = qobject_cast<LinkConfiguration*>(configurations->get(i));
        if (config && config->name() == name) {
            return config;
        }
    }
    return nullptr;
}

QByteArray DebugApiServer::_linksJson()
{
    QJsonArray links;
    QmlObjectListModel *configurations = LinkManager::instance()->linkConfigurations();
    for (int i = 0; configurations && i < configurations->count(); ++i) {
        LinkConfiguration *config = qobject_cast<LinkConfiguration*>(configurations->get(i));
        if (config) {
            links.append(QJsonObject{
                {"name", config->name()},
                {"type", config->type()},
                {"connected", config->link() != nullptr},
            });
        }
    }
    return QJsonDocument(QJsonObject{{"links", links}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_linkConnectJson(const QUrlQuery &query)
{
    const QString host = query.queryItemValue(QStringLiteral("host"), QUrl::FullyDecoded);
    const quint16 port = static_cast<quint16>(query.queryItemValue(QStringLiteral("port")).toUInt());
    if (host.isEmpty() || port == 0) {
        return _errorJson(QStringLiteral("host and port required"));
    }
    QString name = query.queryItemValue(QStringLiteral("name"));
    if (name.isEmpty()) {
        name = QStringLiteral("debug-api %1:%2").arg(host).arg(port);
    }

    const QHostAddress address(host);
    if (address.isNull()) {
        // No DNS here: a blocking lookup would freeze the GUI thread. Clients resolve first.
        return _errorJson(QStringLiteral("host must be an IP address (resolve %1 client-side)").arg(host));
    }

    LinkManager *linkManager = LinkManager::instance();
    LinkConfiguration *config = _findLinkConfiguration(name);
    if (!config) {
        config = linkManager->createConfiguration(LinkConfiguration::TypeTcp, name);
        if (!config) {
            return _errorJson(QStringLiteral("createConfiguration failed"));
        }
        linkManager->endCreateConfiguration(config);
    }
    TCPConfiguration *tcpConfig = qobject_cast<TCPConfiguration*>(config);
    if (!tcpConfig) {
        return _errorJson(QStringLiteral("link %1 exists but is not tcp").arg(name));
    }
    tcpConfig->setHost(address.toString());
    tcpConfig->setPort(port);

    if (!config->link()) {
        linkManager->createConnectedLink(config);
    }
    return QJsonDocument(QJsonObject{
        {"name", name},
        {"host", address.toString()},
        {"port", port},
        {"connected", config->link() != nullptr},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_linkDisconnectJson(const QUrlQuery &query)
{
    const QString name = query.queryItemValue(QStringLiteral("name"));
    if (name.isEmpty()) {
        LinkManager::instance()->disconnectAll();
        return QJsonDocument(QJsonObject{{"disconnected", "all"}}).toJson(QJsonDocument::Compact);
    }
    LinkConfiguration *config = _findLinkConfiguration(name);
    if (!config || !config->link()) {
        return _errorJson(QStringLiteral("no connected link named %1").arg(name));
    }
    config->link()->disconnect();
    return QJsonDocument(QJsonObject{{"disconnected", name}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_videoSettingJson(const QUrlQuery &query)
{
    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    const QString factName = query.queryItemValue(QStringLiteral("fact"));

    if (factName.isEmpty()) {
        // SettingsGroup facts are lazily-created Q_PROPERTYs (DEFINE_SETTINGFACT) with no
        // enumeration API, so the meta-object is the only way to dump them generically.
        QJsonObject facts;
        const QMetaObject *meta = videoSettings->metaObject();
        for (int i = 0; i < meta->propertyCount(); ++i) {
            const QMetaProperty property = meta->property(i);
            Fact *fact = qvariant_cast<Fact*>(property.read(videoSettings));
            if (fact) {
                facts.insert(QString::fromLatin1(property.name()), QJsonValue::fromVariant(fact->rawValue()));
            }
        }
        return QJsonDocument(facts).toJson(QJsonDocument::Compact);
    }

    Fact *fact = qvariant_cast<Fact*>(videoSettings->property(factName.toLatin1().constData()));
    if (!fact) {
        return QJsonDocument(QJsonObject{{"error", QStringLiteral("unknown fact: %1").arg(factName)}}).toJson(QJsonDocument::Compact);
    }
    if (query.hasQueryItem(QStringLiteral("value"))) {
        fact->setRawValue(query.queryItemValue(QStringLiteral("value")));
    }
    return QJsonDocument(QJsonObject{{factName, QJsonValue::fromVariant(fact->rawValue())}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_uiResizeJson(const QUrlQuery &query)
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    bool okWidth = false;
    bool okHeight = false;
    const int width = query.queryItemValue(QStringLiteral("width")).toInt(&okWidth);
    const int height = query.queryItemValue(QStringLiteral("height")).toInt(&okHeight);
    if (!okWidth || !okHeight || width < kMinWindowWidth || height < kMinWindowHeight) {
        return _errorJson(QStringLiteral("width and height required (minimum %1x%2)")
                              .arg(kMinWindowWidth).arg(kMinWindowHeight));
    }

    const QSize maxSize = window->screen() ? window->screen()->size() : QSize(width, height);
    window->resize(qMin(width, maxSize.width()), qMin(height, maxSize.height()));
    QCoreApplication::processEvents(QEventLoop::AllEvents, kResizeSettleMSecs);

    return QJsonDocument(QJsonObject{
        {"width", window->width()},
        {"height", window->height()},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_uiPropJson(const QUrlQuery &query)
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    const QString name = query.queryItemValue(QStringLiteral("name"));
    const QString property = query.queryItemValue(QStringLiteral("property"));
    if (name.isEmpty() || property.isEmpty()) {
        return _errorJson(QStringLiteral("name and property required"));
    }

    QObject *item = _findVisibleItem(window, name);
    if (!item) {
        item = window->findChild<QObject*>(name, Qt::FindChildrenRecursively);
    }
    if (!item) {
        return _errorJson(QStringLiteral("item not found: %1").arg(name));
    }

    const QVariant value = item->property(property.toLatin1().constData());
    if (!value.isValid()) {
        return _errorJson(QStringLiteral("no such property: %1").arg(property));
    }

    return QJsonDocument(QJsonObject{
        {"name", name},
        {"property", property},
        {"value", QJsonValue::fromVariant(value)},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_uiTapJson(const QUrlQuery &query)
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    QPointF point;
    QByteArray error;
    if (!_resolvePoint(window, query, QString(), point, error)) {
        return error;
    }

    QTest::touchEvent(window, _touchDevice(), false).press(0, point.toPoint()).commit();
    QCoreApplication::processEvents(QEventLoop::AllEvents, kEventSliceMSecs);
    QTest::touchEvent(window, _touchDevice(), false).release(0, point.toPoint()).commit();
    QCoreApplication::processEvents(QEventLoop::AllEvents, kEventSliceMSecs);
    return QJsonDocument(QJsonObject{{"tapped", true}, {"x", point.x()}, {"y", point.y()}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_uiPinchJson(const QUrlQuery &query)
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    QPointF centre;
    QByteArray error;
    if (!_resolvePoint(window, query, QString(), centre, error)) {
        return error;
    }

    bool ok = false;
    const qreal fromSpan = query.queryItemValue(QStringLiteral("from")).toDouble(&ok);
    if (!ok || fromSpan <= 0) {
        return _errorJson(QStringLiteral("from (starting finger separation in pixels) required"));
    }
    const qreal toSpan = query.queryItemValue(QStringLiteral("to")).toDouble(&ok);
    if (!ok || toSpan <= 0) {
        return _errorJson(QStringLiteral("to (ending finger separation in pixels) required"));
    }

    int steps = kDefaultGestureSteps;
    if (query.hasQueryItem(QStringLiteral("steps"))) {
        steps = qBound(kMinGestureSteps, query.queryItemValue(QStringLiteral("steps")).toInt(), kMaxGestureSteps);
    }

    TouchCounter counter;

    const auto sendSpan = [&](qreal span, int phase) {
        const QPointF offset(span / 2, 0);
        const QPoint first  = (centre - offset).toPoint();
        const QPoint second = (centre + offset).toPoint();
        QTest::QTouchEventSequence sequence = QTest::touchEvent(window, _touchDevice(), false);
        if (phase == 0) {
            sequence.press(0, first).press(1, second);
        } else if (phase == 1) {
            sequence.move(0, first).move(1, second);
        } else {
            sequence.release(0, first).release(1, second);
        }
        sequence.commit();
        QCoreApplication::processEvents(QEventLoop::AllEvents, kEventSliceMSecs);
    };

    window->installEventFilter(&counter);

    sendSpan(fromSpan, 0);
    for (int i = 1; i <= steps; ++i) {
        sendSpan(fromSpan + (toSpan - fromSpan) * i / steps, 1);
    }
    sendSpan(toSpan, 2);

    window->removeEventFilter(&counter);

    return QJsonDocument(QJsonObject{
        {"sent", true},
        {"touchBeginReceived", counter.begin},
        {"touchUpdateReceived", counter.update},
        {"touchEndReceived", counter.end},
        {"x", centre.x()}, {"y", centre.y()},
        {"from", fromSpan}, {"to", toSpan},
    }).toJson(QJsonDocument::Compact);
}

static std::optional<QPointF> s_uiPressActiveAt;

QByteArray DebugApiServer::_uiMouseStepJson(const QUrlQuery &query, QEvent::Type type)
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    QPointF scenePos;
    QByteArray error;
    if (!_resolvePoint(window, query, QString(), scenePos, error)) {
        return error;
    }

    if (type == QEvent::MouseButtonPress && s_uiPressActiveAt) {
        _mouse(window, *s_uiPressActiveAt, Qt::NoButton, Qt::LeftButton, QEvent::MouseButtonRelease);
        s_uiPressActiveAt.reset();
    }

    const Qt::MouseButtons buttons = (type == QEvent::MouseButtonRelease) ? Qt::NoButton : Qt::LeftButton;
    const Qt::MouseButton button = (type == QEvent::MouseMove) ? Qt::NoButton : Qt::LeftButton;
    _mouse(window, scenePos, buttons, button, type);

    if (type == QEvent::MouseButtonPress) {
        s_uiPressActiveAt = scenePos;
    } else if (type == QEvent::MouseButtonRelease) {
        s_uiPressActiveAt.reset();
    } else if (s_uiPressActiveAt) {
        s_uiPressActiveAt = scenePos;
    }

    return QJsonDocument(QJsonObject{
        {"ok", true},
        {"x", scenePos.x()},
        {"y", scenePos.y()},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_mockLinkJson(const QUrlQuery &query)
{
    const QString autopilot = query.queryItemValue(QStringLiteral("autopilot")).toLower();

    if (autopilot.isEmpty()) {
        return _errorJson(QStringLiteral("autopilot must be px4 or apm"));
    }

    if (!query.hasQueryItem(QStringLiteral("add"))) {
        const QList<SharedLinkInterfacePtr> links = LinkManager::instance()->links();
        const bool mockExists = std::any_of(links.cbegin(), links.cend(), [](const SharedLinkInterfacePtr &link) {
            return link->linkConfiguration()->type() == LinkConfiguration::TypeMock;
        });
        if (mockExists) {
            return QJsonDocument(QJsonObject{
                {"started", false},
                {"existing", true},
                {"autopilot", autopilot},
            }).toJson(QJsonDocument::Compact);
        }
    }

    MockConfiguration *config = new MockConfiguration("MockLink");
    if (autopilot == QStringLiteral("px4")) {
        config->setFirmwareType(MAV_AUTOPILOT_PX4);
        config->setVehicleType(MAV_TYPE_QUADROTOR);
    } else if (autopilot == QStringLiteral("apm") || autopilot == QStringLiteral("arducopter")) {
        config->setFirmwareType(MAV_AUTOPILOT_ARDUPILOTMEGA);
        config->setVehicleType(MAV_TYPE_QUADROTOR);
    } else {
        delete config;
        return _errorJson(QStringLiteral("autopilot must be px4 or apm, got: %1").arg(autopilot));
    }
    config->setDynamic(true);

    SharedLinkConfigurationPtr sharedConfig(config);
    if (!LinkManager::instance()->createConnectedLink(sharedConfig)) {
        return _errorJson(QStringLiteral("could not start mock link"));
    }

    return QJsonDocument(QJsonObject{
        {"started", true},
        {"autopilot", autopilot},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_uiDismissJson()
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return _errorJson(QStringLiteral("no main window"));
    }

    int closed = 0;
    int examined = 0;
    const QList<QQuickItem*> items = window->contentItem()->findChildren<QQuickItem*>();
    for (QQuickItem *item : items) {
        if (!item->inherits("QQuickPopupItem")) {
            continue;
        }
        examined++;
        if (QObject *popup = item->parent()) {
            if (popup->inherits("QQuickPopup") && popup->property("visible").toBool()) {
                QMetaObject::invokeMethod(popup, "close");
                closed++;
            }
        }
    }

    return QJsonDocument(QJsonObject{
        {"closed", closed},
        {"popupItemsExamined", examined},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_screenshotJson()
{
    QQuickWindow *window = qgcApp()->mainRootWindow();
    if (!window) {
        return QJsonDocument(QJsonObject{{"error", "no main window"}}).toJson(QJsonDocument::Compact);
    }
    QImage image = window->grabWindow();
    // Full retina grabs are multi-megabyte; tooling consumers only need enough to read the UI.
    constexpr int kMaxWidth = 1280;
    if (image.width() > kMaxWidth) {
        image = image.scaledToWidth(kMaxWidth, Qt::SmoothTransformation);
    }
    const QString file = QStandardPaths::writableLocation(QStandardPaths::TempLocation) + QStringLiteral("/qgc-debug-screenshot-%1.jpg").arg(QCoreApplication::applicationPid());
    if (!image.save(file, "JPEG", 80)) {
        return QJsonDocument(QJsonObject{{"error", "save failed"}}).toJson(QJsonDocument::Compact);
    }
    const qreal sceneWidth  = window->width();
    const qreal sceneHeight = window->height();
    return QJsonDocument(QJsonObject{
        {"imageFile", file},
        {"size", QStringLiteral("%1x%2").arg(image.width()).arg(image.height())},
        {"imageWidth", image.width()},
        {"imageHeight", image.height()},
        {"sceneWidth", sceneWidth},
        {"sceneHeight", sceneHeight},
        {"imageToScene", image.width() > 0 ? sceneWidth / image.width() : 1.0},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_vehicleJson()
{
    Vehicle *vehicle = MultiVehicleManager::instance()->activeVehicle();
    if (!vehicle) {
        return QJsonDocument(QJsonObject{{"connected", false}}).toJson(QJsonDocument::Compact);
    }

    QJsonObject json{
        {"connected", true},
        {"id", vehicle->id()},
        {"armed", vehicle->armed()},
        {"flightMode", vehicle->flightMode()},
        {"latitude", vehicle->coordinate().latitude()},
        {"longitude", vehicle->coordinate().longitude()},
    };

    if (FactGroup *group = vehicle->vehicleFactGroup()) {
        if (Fact *fact = group->getFact(QStringLiteral("altitudeRelative"))) {
            json.insert(QStringLiteral("altitudeRelative"), fact->rawValue().toDouble());
        }
        if (Fact *fact = group->getFact(QStringLiteral("groundSpeed"))) {
            json.insert(QStringLiteral("groundSpeed"), fact->rawValue().toDouble());
        }
    }
    if (FactGroup *gps = vehicle->gpsFactGroup()) {
        if (Fact *fact = gps->getFact(QStringLiteral("count"))) {
            json.insert(QStringLiteral("gpsCount"), fact->rawValue().toInt());
        }
        if (Fact *fact = gps->getFact(QStringLiteral("lock"))) {
            json.insert(QStringLiteral("gpsLock"), fact->rawValue().toInt());
        }
    }
    if (vehicle->batteries()->count() > 0) {
        if (FactGroup *battery = qobject_cast<FactGroup*>(vehicle->batteries()->get(0))) {
            if (Fact *fact = battery->getFact(QStringLiteral("percentRemaining"))) {
                json.insert(QStringLiteral("batteryPercent"), fact->rawValue().toDouble());
            }
        }
    }
    return QJsonDocument(json).toJson(QJsonDocument::Compact);
}

static ParameterManager *_parameterManager(QByteArray &error)
{
    Vehicle *vehicle = MultiVehicleManager::instance()->activeVehicle();
    if (!vehicle) {
        error = _errorJson(QStringLiteral("no vehicle connected"));
        return nullptr;
    }
    ParameterManager *parameterManager = vehicle->parameterManager();
    if (!parameterManager || !parameterManager->parametersReady()) {
        error = _errorJson(QStringLiteral("parameters not ready yet"));
        return nullptr;
    }
    return parameterManager;
}

QByteArray DebugApiServer::_paramsJson(const QUrlQuery &query)
{
    QByteArray error;
    ParameterManager *parameterManager = _parameterManager(error);
    if (!parameterManager) {
        return error;
    }

    const QString name = query.queryItemValue(QStringLiteral("name"), QUrl::FullyDecoded);
    if (!name.isEmpty()) {
        if (!parameterManager->parameterExists(ParameterManager::defaultComponentId, name)) {
            return _errorJson(QStringLiteral("unknown parameter: %1").arg(name));
        }
        Fact *fact = parameterManager->getParameter(ParameterManager::defaultComponentId, name);
        return QJsonDocument(QJsonObject{
            {"name", name},
            {"value", QJsonValue::fromVariant(fact->rawValue())},
            {"units", fact->cookedUnits()},
            {"min", QJsonValue::fromVariant(fact->cookedMin())},
            {"max", QJsonValue::fromVariant(fact->cookedMax())},
            {"description", fact->shortDescription()},
        }).toJson(QJsonDocument::Compact);
    }

    const QString filter = query.queryItemValue(QStringLiteral("filter"), QUrl::FullyDecoded);
    const int limitArg = query.queryItemValue(QStringLiteral("limit")).toInt();
    const int limit = qBound(1, limitArg ? limitArg : 100, 10000);

    QJsonObject params;
    int total = 0;
    const QStringList names = parameterManager->parameterNames(ParameterManager::defaultComponentId);
    for (const QString &paramName : names) {
        if (!filter.isEmpty() && !paramName.contains(filter, Qt::CaseInsensitive)) {
            continue;
        }
        ++total;
        if (params.size() < limit) {
            params.insert(paramName, QJsonValue::fromVariant(parameterManager->getParameter(ParameterManager::defaultComponentId, paramName)->rawValue()));
        }
    }
    return QJsonDocument(QJsonObject{
        {"params", params},
        {"matched", total},
        {"truncated", total > params.size()},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_paramSetJson(const QUrlQuery &query)
{
    QByteArray error;
    ParameterManager *parameterManager = _parameterManager(error);
    if (!parameterManager) {
        return error;
    }

    const QString name = query.queryItemValue(QStringLiteral("name"), QUrl::FullyDecoded);
    if (name.isEmpty() || !query.hasQueryItem(QStringLiteral("value"))) {
        return _errorJson(QStringLiteral("name and value required"));
    }
    if (!parameterManager->parameterExists(ParameterManager::defaultComponentId, name)) {
        return _errorJson(QStringLiteral("unknown parameter: %1").arg(name));
    }

    Fact *fact = parameterManager->getParameter(ParameterManager::defaultComponentId, name);
    fact->setRawValue(query.queryItemValue(QStringLiteral("value"), QUrl::FullyDecoded));
    return QJsonDocument(QJsonObject{
        {"name", name},
        {"value", QJsonValue::fromVariant(fact->rawValue())},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_paramsFileJson(const QUrlQuery &query, bool save)
{
    QByteArray error;
    ParameterManager *parameterManager = _parameterManager(error);
    if (!parameterManager) {
        return error;
    }

    QString file = query.queryItemValue(QStringLiteral("file"), QUrl::FullyDecoded);
    if (file.isEmpty()) {
        if (!save) {
            return _errorJson(QStringLiteral("file required"));
        }
        file = QStandardPaths::writableLocation(QStandardPaths::TempLocation) + QStringLiteral("/qgc-params-%1.params").arg(QCoreApplication::applicationPid());
    }

    QFile paramFile(file);
    if (save) {
        if (!paramFile.open(QIODevice::WriteOnly | QIODevice::Text)) {
            return _errorJson(QStringLiteral("cannot write %1").arg(file));
        }
        QTextStream stream(&paramFile);
        parameterManager->writeParametersToStream(stream);
        return QJsonDocument(QJsonObject{{"saved", file}}).toJson(QJsonDocument::Compact);
    }

    if (!paramFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return _errorJson(QStringLiteral("cannot read %1").arg(file));
    }
    QTextStream stream(&paramFile);
    const QString result = parameterManager->readParametersFromStream(stream);
    return QJsonDocument(QJsonObject{
        {"loaded", file},
        {"notes", result},
    }).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_calibrateJson(const QUrlQuery &query)
{
    Vehicle *vehicle = MultiVehicleManager::instance()->activeVehicle();
    if (!vehicle) {
        return _errorJson(QStringLiteral("no vehicle connected"));
    }
    if (query.queryItemValue(QStringLiteral("stop")) == QStringLiteral("1")) {
        vehicle->stopCalibration(false);
        return QJsonDocument(QJsonObject{{"calibration", "stopped"}}).toJson(QJsonDocument::Compact);
    }

    const QString type = query.queryItemValue(QStringLiteral("type"));
    static const QHash<QString, QGCMAVLink::CalibrationType> kTypes{
        {QStringLiteral("accel"), QGCMAVLink::CalibrationAccel},
        {QStringLiteral("mag"), QGCMAVLink::CalibrationMag},
        {QStringLiteral("compass"), QGCMAVLink::CalibrationMag},
        {QStringLiteral("gyro"), QGCMAVLink::CalibrationGyro},
        {QStringLiteral("level"), QGCMAVLink::CalibrationLevel},
        {QStringLiteral("radio"), QGCMAVLink::CalibrationRadio},
        {QStringLiteral("esc"), QGCMAVLink::CalibrationEsc},
    };
    if (!kTypes.contains(type)) {
        return _errorJson(QStringLiteral("type must be one of: accel, mag, gyro, level, radio, esc"));
    }
    vehicle->startCalibration(kTypes.value(type));
    return QJsonDocument(QJsonObject{{"calibration", type}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_motorTestJson(const QUrlQuery &query)
{
    if (!qEnvironmentVariableIsSet("QGC_DEBUG_API_ALLOW_ACTUATORS")) {
        return _errorJson(QStringLiteral("motor test disabled; set QGC_DEBUG_API_ALLOW_ACTUATORS=1 (props off!)"));
    }
    Vehicle *vehicle = MultiVehicleManager::instance()->activeVehicle();
    if (!vehicle) {
        return _errorJson(QStringLiteral("no vehicle connected"));
    }
    const int motor = query.queryItemValue(QStringLiteral("motor")).toInt();
    const int percent = qBound(0, query.queryItemValue(QStringLiteral("percent")).toInt(), 100);
    const int timeout = qBound(1, query.queryItemValue(QStringLiteral("timeout")).toInt(), 10);
    if (motor < 1) {
        return _errorJson(QStringLiteral("motor (1-based) required"));
    }
    vehicle->motorTest(motor, percent, timeout, false);
    return QJsonDocument(QJsonObject{{"motor", motor}, {"percent", percent}, {"timeout", timeout}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_messagesJson() const
{
    return QJsonDocument(QJsonObject{{"messages", QJsonArray::fromStringList(_messages)}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_rcJson() const
{
    QJsonArray channels;
    for (int value : _rcValues) {
        channels.append(value);
    }
    return QJsonDocument(QJsonObject{{"channels", channels}}).toJson(QJsonDocument::Compact);
}

PlanMasterController *DebugApiServer::_planController()
{
    if (!_plan) {
        _plan = new PlanMasterController(this);
        _plan->setFlyView(false);
        _plan->start();
    }
    return _plan;
}

QByteArray DebugApiServer::_missionJson(const QUrlQuery &query, bool upload)
{
    if (!MultiVehicleManager::instance()->activeVehicle()) {
        return _errorJson(QStringLiteral("no vehicle connected"));
    }
    const QString file = query.queryItemValue(QStringLiteral("file"), QUrl::FullyDecoded);
    if (file.isEmpty()) {
        return _errorJson(QStringLiteral("file required"));
    }

    PlanMasterController *plan = _planController();
    if (upload) {
        if (!QFile::exists(file)) {
            return _errorJson(QStringLiteral("no such file: %1").arg(file));
        }
        plan->loadFromFile(file);
        plan->sendToVehicle();
        return QJsonDocument(QJsonObject{{"uploading", file}}).toJson(QJsonDocument::Compact);
    }

    QObject::disconnect(_pendingMissionDownload);
    _pendingMissionDownload = connect(plan, &PlanMasterController::syncInProgressChanged, this, [this, plan, file]() {
        if (!plan->syncInProgress()) {
            QObject::disconnect(_pendingMissionDownload);
            // A failed or empty download must not produce a file: clients poll for the
            // file's existence as the success signal.
            if (plan->containsItems()) {
                plan->saveToFile(file);
            } else {
                qCWarning(DebugApiServerLog) << "mission download finished empty, not writing" << file;
            }
        }
    });
    plan->loadFromVehicle();
    return QJsonDocument(QJsonObject{{"downloading", file}}).toJson(QJsonDocument::Compact);
}

QByteArray DebugApiServer::_statusJson()
{
    VideoManager *videoManager = VideoManager::instance();
    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();

    const QStringList statuses = videoManager->cameraStatuses();
    QJsonArray cameras;
    for (int i = 0; i < statuses.size(); ++i) {
        cameras.append(QJsonObject{
            {"index", i},
            {"name", videoManager->cameraName(i)},
            {"source", videoSettings->videoSourceNameAt(i)},
            {"status", statuses.at(i)},
            {"receiving", statuses.at(i).isEmpty()},
            {"framesDecoded", static_cast<qint64>(videoManager->cameraFramesDecoded(i))},
            {"bytesReceived", static_cast<qint64>(videoManager->cameraBytesReceived(i))},
            {"secondsSinceLastFrame", videoManager->cameraSecondsSinceLastFrame(i)},
        });
    }

    QSettings layoutSettings;
    layoutSettings.beginGroup(QLatin1String(QGroundControlQmlGlobal::kQmlGlobalKeyName));
    const QJsonObject layout{
        {"mainIsMap", layoutSettings.value(QStringLiteral("MainFlyWindowIsMap"), true).toBool()},
        {"pipExpanded", layoutSettings.value(QStringLiteral("IsPIPVisible"), true).toBool()},
        {"pipCustomPosition", layoutSettings.value(QStringLiteral("PIPCustomPosition"), false).toBool()},
        {"pipSize", layoutSettings.value(QStringLiteral("PIPSize"), 0).toDouble()},
        {"tileSize", layoutSettings.value(QStringLiteral("VideoTileSize"), 0).toDouble()},
    };

    const QJsonObject status{
        {"activeVideoSource", videoManager->activeVideoSource()},
        {"multiViewEnabled", videoSettings->multiViewEnabled()->rawValue().toBool()},
        {"decoding", videoManager->decoding()},
        {"streaming", videoManager->streaming()},
        {"recording", videoManager->recording()},
        {"videoSize", QStringLiteral("%1x%2").arg(videoManager->videoSize().width()).arg(videoManager->videoSize().height())},
        {"cameras", cameras},
        {"layout", layout},
        {"uiPressActive", s_uiPressActiveAt.has_value()},
        {"uiPressX", s_uiPressActiveAt ? s_uiPressActiveAt->x() : 0.0},
        {"uiPressY", s_uiPressActiveAt ? s_uiPressActiveAt->y() : 0.0},
    };
    return QJsonDocument(status).toJson(QJsonDocument::Compact);
}
