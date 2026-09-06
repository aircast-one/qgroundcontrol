/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include <QtCore/QEvent>
#include <QtCore/QObject>
#include <QtCore/QStringList>

class PlanMasterController;
class QQuickWindow;
class QTcpServer;
class QTcpSocket;
class QUrlQuery;

/// Localhost-only JSON debug API for driving a running instance from external tooling
/// (tests, MCP servers). Disabled unless the QGC_DEBUG_API_PORT environment variable
/// holds a port number; binds to 127.0.0.1 only.
class DebugApiServer : public QObject
{
    Q_OBJECT

public:
    static void startIfConfigured(QObject *parent);
    static void start(quint16 port, QObject *parent);

    /// Direct construction is for unit tests; production goes through startIfConfigured.
    /// Pass port 0 to listen on an ephemeral port (see serverPort()).
    DebugApiServer(quint16 port, QObject *parent = nullptr);

    quint16 serverPort() const;

    /// Points the /ui/* endpoints at a window the caller owns. Unit tests have no QML application
    /// engine, so mainRootWindow() is null and every endpoint that reads the scene can only be
    /// exercised on its failure path. Pass nullptr to go back to the application's own window.
    static void setWindowForTesting(QQuickWindow *window);

private:
    void _handleConnection(QTcpSocket *socket);
    QByteArray _route(const QString &path, const QUrlQuery &query);
#ifdef Q_OS_MACOS
    QByteArray _nativeJson(const QString &path, const QUrlQuery &query);
#endif
    static QByteArray _statusJson();
    static QByteArray _screenshotJson();
    static QByteArray _uiMouseStepJson(const QUrlQuery &query, QEvent::Type type);
    static QByteArray _uiPinchJson(const QUrlQuery &query);
    static QByteArray _uiTapJson(const QUrlQuery &query);
    static QByteArray _uiPropJson(const QUrlQuery &query);
    static QByteArray _uiPropSetJson(const QUrlQuery &query);
    static QByteArray _uiAtJson(const QUrlQuery &query);

    /// Streams one NDJSON sample per animation tick until the frame budget runs out or the peer
    /// hangs up. Polling for values that change every frame reads them a round trip apart;
    /// sampling on the GUI thread's own animation tick reads them where they actually are.
    static bool _startWatch(QTcpSocket *socket, const QUrlQuery &query);
    static QByteArray _uiResizeJson(const QUrlQuery &query);
    static QByteArray _mockLinkJson(const QUrlQuery &query);
    static QByteArray _uiDismissJson();
    static QByteArray _vehicleJson();
    static QByteArray _paramsJson(const QUrlQuery &query);
    static QByteArray _paramSetJson(const QUrlQuery &query);
    static QByteArray _paramsFileJson(const QUrlQuery &query, bool save);
    static QByteArray _calibrateJson(const QUrlQuery &query);
    static QByteArray _motorTestJson(const QUrlQuery &query);
    QByteArray _messagesJson() const;
    QByteArray _rcJson() const;
    QByteArray _missionJson(const QUrlQuery &query, bool upload);
    PlanMasterController *_planController();
    static QByteArray _uiTreeJson(const QUrlQuery &query);
    static QByteArray _uiClickJson(const QUrlQuery &query, bool doubleClick);
    static QByteArray _uiDragJson(const QUrlQuery &query);
    static QByteArray _uiHoverJson(const QUrlQuery &query);
    static QByteArray _uiTypeJson(const QUrlQuery &query);
    static QByteArray _uiKeyJson(const QUrlQuery &query);
    static QByteArray _loggingJson(const QUrlQuery &query);
    static QByteArray _linksJson();
    static QByteArray _linkConnectJson(const QUrlQuery &query);
    static QByteArray _linkDisconnectJson(const QUrlQuery &query);
    static QByteArray _videoSettingJson(const QUrlQuery &query);

    /// The window the /ui/* endpoints act on: the test override when one is set, otherwise the
    /// application's root window.
    static QQuickWindow *_targetWindow();

    static DebugApiServer *_instance;
    static QQuickWindow *_testWindow;

    QTcpServer *_server = nullptr;
    QMetaObject::Connection _messageConnection;
    QMetaObject::Connection _rcConnection;
    QMetaObject::Connection _pendingMissionDownload;
    QStringList _messages;
    QList<int> _rcValues;
    PlanMasterController *_plan = nullptr;
};
