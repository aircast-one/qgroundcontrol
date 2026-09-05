/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "AircastDeviceSetupTest.h"
#include "LinkManager.h"
#include "QGCApplication.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "UDPLink.h"
#include "VideoSettings.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QTimer>
#include <QtNetwork/QTcpServer>
#include <QtNetwork/QTcpSocket>
#include <QtTest/QTest>

namespace {

/// Minimal in-process aircastd: serves canned JSON for the device API paths.
class FakeAircastd : public QObject
{
public:
    FakeAircastd()
    {
        _server.listen(QHostAddress::LocalHost, 0);
        connect(&_server, &QTcpServer::newConnection, this, [this]() {
            QTcpSocket *socket = _server.nextPendingConnection();
            connect(socket, &QTcpSocket::readyRead, socket, [this, socket]() {
                const QString path = QString::fromUtf8(socket->readAll()).section(QLatin1Char(' '), 1, 1);
                const QByteArray body = routes.value(path);
                const QByteArray response = body.isNull()
                    ? QByteArrayLiteral("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    : QByteArrayLiteral("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: ")
                        + QByteArray::number(body.size()) + QByteArrayLiteral("\r\nConnection: close\r\n\r\n") + body;
                const auto respond = [socket, response]() {
                    socket->write(response);
                    socket->disconnectFromHost();
                };
                if (responseDelayMs > 0) {
                    QTimer::singleShot(responseDelayMs, socket, respond);
                } else {
                    respond();
                }
            });
            connect(socket, &QTcpSocket::disconnected, socket, &QObject::deleteLater);
        });
    }

    QString hostWithPort() const { return QStringLiteral("127.0.0.1:%1").arg(_server.serverPort()); }

    /// Delays every HTTP response by this many milliseconds, to deterministically make this device
    /// the "slow, superseded" one in a race against another FakeAircastd.
    int responseDelayMs = 0;

    void setDevice(const QStringList &cameras, const QStringList &telemetryEndpoints)
    {
        QJsonObject paths{{QStringLiteral("no-source"), QJsonObject{}}};
        for (const QString &camera : cameras) {
            paths.insert(camera, QJsonObject{{QStringLiteral("source"), QStringLiteral("rtsp://192.168.144.25:8554/main")}});
        }
        routes.insert(QStringLiteral("/api/stream/config"),
                      QJsonDocument(QJsonObject{{QStringLiteral("paths"), paths}}).toJson(QJsonDocument::Compact));
        routes.insert(QStringLiteral("/api/telemetry/config"),
                      QJsonDocument(QJsonObject{{QStringLiteral("baud"), 115200},
                                                {QStringLiteral("endpoints"), QJsonArray::fromStringList(telemetryEndpoints)}})
                          .toJson(QJsonDocument::Compact));
    }

    QHash<QString, QByteArray> routes;

private:
    QTcpServer _server;
};

const QString kLinkName = QStringLiteral("Aircast 127.0.0.1");

QList<LinkConfiguration*> _aircastLinkConfigs()
{
    QList<LinkConfiguration*> found;
    QmlObjectListModel *configs = LinkManager::instance()->linkConfigurations();
    for (int i = 0; i < configs->count(); i++) {
        LinkConfiguration *config = qobject_cast<LinkConfiguration*>(configs->get(i));
        if (config && config->name() == kLinkName) {
            found.append(config);
        }
    }
    return found;
}

void _removeAircastLinkConfigs()
{
    for (LinkConfiguration *config : _aircastLinkConfigs()) {
        LinkManager::instance()->removeConfiguration(config);
    }
}

void _applySetupDeepLink(const FakeAircastd &device)
{
    qgcApp()->handleDeepLink(QUrl(QStringLiteral("aircast-qgc://setup?host=%1").arg(device.hostWithPort())));
}

} // namespace

void AircastDeviceSetupTest::_configuresCamerasAndTelemetryFromDevice()
{
    FakeAircastd device;
    device.setDevice({QStringLiteral("cam1"), QStringLiteral("cam2")}, {QStringLiteral("udps:0.0.0.0:14550")});

    _applySetupDeepLink(device);

    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    QTRY_COMPARE_WITH_TIMEOUT(videoSettings->rtspUrl()->rawValue().toString(), QStringLiteral("rtsp://127.0.0.1:8554/cam1"), 5000);
    QCOMPARE(videoSettings->videoSource()->rawValue().toString(), QString::fromUtf8(VideoSettings::videoSourceRTSP));
    QCOMPARE(videoSettings->primaryCameraName()->rawValue().toString(), QStringLiteral("cam1 (127.0.0.1)"));

    const QJsonArray extras = QJsonDocument::fromJson(videoSettings->extraVideoSources()->rawValue().toString().toUtf8()).array();
    QCOMPARE(extras.size(), 1);
    const QJsonObject extra = extras.first().toObject();
    QCOMPARE(extra.value(QStringLiteral("name")).toString(), QStringLiteral("cam2 (127.0.0.1)"));
    QCOMPARE(extra.value(QStringLiteral("source")).toString(), QString::fromUtf8(VideoSettings::videoSourceWebRTC));
    QCOMPARE(extra.value(QStringLiteral("url")).toString(), QStringLiteral("http://127.0.0.1:8889/cam2/whep"));

    QTRY_COMPARE_WITH_TIMEOUT(_aircastLinkConfigs().size(), 1, 5000);
    LinkConfiguration *linkConfig = _aircastLinkConfigs().first();
    QCOMPARE(linkConfig->type(), LinkConfiguration::TypeUdp);
    QVERIFY(linkConfig->isAutoConnect());
    QVERIFY(linkConfig->link());
    const UDPConfiguration *udpConfig = qobject_cast<UDPConfiguration*>(linkConfig);
    QVERIFY(udpConfig);
    QCOMPARE(udpConfig->hostList(), QStringList{QStringLiteral("127.0.0.1:14550")});

    _removeAircastLinkConfigs();
}

void AircastDeviceSetupTest::_reapplyReplacesExistingLink()
{
    FakeAircastd device;
    device.setDevice({QStringLiteral("cam1")}, {QStringLiteral("udps:0.0.0.0:14550")});
    _applySetupDeepLink(device);
    QTRY_COMPARE_WITH_TIMEOUT(_aircastLinkConfigs().size(), 1, 5000);

    device.setDevice({QStringLiteral("front")}, {QStringLiteral("udps:0.0.0.0:14551")});
    _applySetupDeepLink(device);

    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    QTRY_COMPARE_WITH_TIMEOUT(videoSettings->rtspUrl()->rawValue().toString(), QStringLiteral("rtsp://127.0.0.1:8554/front"), 5000);
    QTRY_COMPARE_WITH_TIMEOUT(_aircastLinkConfigs().size(), 1, 5000);
    const UDPConfiguration *udpConfig = qobject_cast<UDPConfiguration*>(_aircastLinkConfigs().first());
    QVERIFY(udpConfig);
    QTRY_COMPARE_WITH_TIMEOUT(udpConfig->hostList(), QStringList{QStringLiteral("127.0.0.1:14551")}, 5000);

    _removeAircastLinkConfigs();
}

void AircastDeviceSetupTest::_clientOnlyTelemetryEndpointCreatesNoLink()
{
    FakeAircastd device;
    device.setDevice({QStringLiteral("cam1")}, {QStringLiteral("udpc:10.0.0.5:14550")});

    _applySetupDeepLink(device);

    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    QTRY_COMPARE_WITH_TIMEOUT(videoSettings->rtspUrl()->rawValue().toString(), QStringLiteral("rtsp://127.0.0.1:8554/cam1"), 5000);

    // The camera and telemetry replies land independently; give the telemetry
    // handler a bounded window to (wrongly) create a link before asserting.
    QTest::qWait(200);
    QCOMPARE(_aircastLinkConfigs().size(), 0);
}

void AircastDeviceSetupTest::_staleReplyFromSupersededSetupIsIgnored()
{
    // deviceA is slow to respond; deviceB responds immediately. Firing deviceA's
    // deep link and then immediately superseding it with deviceB's must leave
    // deviceB's config in place even once deviceA's late reply finally arrives.
    FakeAircastd deviceA;
    deviceA.responseDelayMs = 300;
    deviceA.setDevice({QStringLiteral("stale")}, {QStringLiteral("udps:0.0.0.0:14550")});

    FakeAircastd deviceB;
    deviceB.setDevice({QStringLiteral("fresh")}, {QStringLiteral("udps:0.0.0.0:14551")});

    _applySetupDeepLink(deviceA);
    _applySetupDeepLink(deviceB);

    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    QTRY_COMPARE_WITH_TIMEOUT(videoSettings->rtspUrl()->rawValue().toString(), QStringLiteral("rtsp://127.0.0.1:8554/fresh"), 5000);

    // Let deviceA's delayed reply land, then confirm it did not clobber deviceB's config.
    QTest::qWait(deviceA.responseDelayMs + 200);
    QCOMPARE(videoSettings->rtspUrl()->rawValue().toString(), QStringLiteral("rtsp://127.0.0.1:8554/fresh"));
    QCOMPARE(videoSettings->primaryCameraName()->rawValue().toString(), QStringLiteral("fresh (127.0.0.1)"));

    QTRY_COMPARE_WITH_TIMEOUT(_aircastLinkConfigs().size(), 1, 5000);
    const UDPConfiguration *udpConfig = qobject_cast<UDPConfiguration*>(_aircastLinkConfigs().first());
    QVERIFY(udpConfig);
    QCOMPARE(udpConfig->hostList(), QStringList{QStringLiteral("127.0.0.1:14551")});

    _removeAircastLinkConfigs();
}
