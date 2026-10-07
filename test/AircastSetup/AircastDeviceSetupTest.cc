/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "AircastDeviceSetupTest.h"
#include "AircastAccount.h"
#include "AircastCloudLink.h"
#include "LinkInterface.h"
#include "LinkManager.h"
#include "LogManager.h"
#include "QGCLoggingCategoryManager.h"
#include "QGCApplication.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "UDPLink.h"
#include "VideoCloudFailover.h"
#include "VideoManager.h"
#include "VideoSettings.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QScopeGuard>
#include <QtCore/QTimer>
#include <QtNetwork/QTcpServer>
#include <QtNetwork/QTcpSocket>
#include <QtCore/QRegularExpression>
#include <QtTest/QTest>

#include <algorithm>

namespace {

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

    int responseDelayMs = 0;

    void setDevice(const QStringList &cameras, const QStringList &telemetryEndpoints)
    {
        QJsonObject paths{{QStringLiteral("no-source"), QJsonObject{}}};
        for (const QString &camera : cameras) {
            paths.insert(camera, QJsonObject{{QStringLiteral("source"), QStringLiteral("rtsp://192.168.144.25:8554/main")}});
        }
        routes.insert(QStringLiteral("/api/stream/config"),
                      QJsonDocument(QJsonObject{{QStringLiteral("paths"), paths}}).toJson(QJsonDocument::Compact));
        QJsonObject telemetry{{QStringLiteral("baud"), 115200},
                              {QStringLiteral("endpoints"), QJsonArray::fromStringList(telemetryEndpoints)}};
        if (!cloud.isEmpty()) {
            telemetry.insert(QStringLiteral("cloud"), cloud);
        }
        routes.insert(QStringLiteral("/api/telemetry/config"), QJsonDocument(telemetry).toJson(QJsonDocument::Compact));
    }

    QJsonObject cloud;

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
        if (config && config->name().startsWith(kLinkName)) {
            found.append(config);
        }
    }
    return found;
}

void _removeAircastLinkConfigs()
{
    for (LinkConfiguration *config : _aircastLinkConfigs()) {
        if (LinkInterface *const link = config->link()) {
            link->disconnect();
        }
        LinkManager::instance()->removeConfiguration(config);
    }

    const auto aircastLinkOpen = []() {
        const QList<SharedLinkInterfacePtr> links = LinkManager::instance()->links();
        return std::any_of(links.begin(), links.end(), [](const SharedLinkInterfacePtr &link) {
            return link->linkConfiguration() && link->linkConfiguration()->name().startsWith(kLinkName);
        });
    };
    QTRY_VERIFY_WITH_TIMEOUT(!aircastLinkOpen(), 1000);
}

void _applySetupDeepLink(const FakeAircastd &device)
{
    qgcApp()->handleDeepLink(QUrl(QStringLiteral("aircast-qgc://setup?host=%1").arg(device.hostWithPort())));
}

}

void AircastDeviceSetupTest::_configuresCamerasAndTelemetryFromDevice()
{
    FakeAircastd device;
    device.setDevice({QStringLiteral("cam1"), QStringLiteral("cam2")}, {QStringLiteral("udps:0.0.0.0:14550")});

    _applySetupDeepLink(device);

    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    QTRY_COMPARE_WITH_TIMEOUT(videoSettings->currentVideoUrl(), QStringLiteral("rtsp://127.0.0.1:8554/cam1"), 5000);
    const int first = videoSettings->currentIndex();
    QCOMPARE(videoSettings->videoSourceNameAt(first), QString::fromUtf8(VideoSettings::videoSourceRTSP));
    QCOMPARE(videoSettings->cameraName(first), QStringLiteral("cam1 (127.0.0.1)"));
    QCOMPARE(videoSettings->videoSourceCount(), first + 2);
    QCOMPARE(videoSettings->cameraName(first + 1), QStringLiteral("cam2 (127.0.0.1)"));
    QCOMPARE(videoSettings->videoSourceNameAt(first + 1), QString::fromUtf8(VideoSettings::videoSourceRTSP));
    QCOMPARE(videoSettings->videoUrlAt(first + 1), QStringLiteral("rtsp://127.0.0.1:8554/cam2"));

    QTRY_COMPARE_WITH_TIMEOUT(_aircastLinkConfigs().size(), 1, 5000);
    LinkConfiguration *linkConfig = _aircastLinkConfigs().first();
    QCOMPARE(linkConfig->type(), LinkConfiguration::TypeUdp);
    QVERIFY(linkConfig->isAutoConnect());
    QVERIFY(linkConfig->link());
    const UDPConfiguration *udpConfig = qobject_cast<UDPConfiguration*>(linkConfig);
    QVERIFY(udpConfig);
    QCOMPARE(udpConfig->hostList(), QStringList{QStringLiteral("127.0.0.1:14550")});
    QCOMPARE(udpConfig->localPort(), quint16(0));

    _removeAircastLinkConfigs();
}

void AircastDeviceSetupTest::_camerasTheDeviceSteersToCloudflareAreWatchedThroughIt()
{
    FakeAircastd device;
    device.setDevice({QStringLiteral("cam1"), QStringLiteral("cam2"), QStringLiteral("cam3")}, {QStringLiteral("udps:0.0.0.0:14550")});
    device.routes.insert(QStringLiteral("/api/watch/via"),
                         QJsonDocument(QJsonObject{{QStringLiteral("cam1"), QStringLiteral("cloudflare")},
                                                   {QStringLiteral("cam3"), QStringLiteral("cloudflare")}}).toJson(QJsonDocument::Compact));

    _applySetupDeepLink(device);

    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    const QString cloudflare = QStringLiteral("http://%1/whep/cloudflare/").arg(device.hostWithPort());
    QTRY_COMPARE_WITH_TIMEOUT(videoSettings->currentVideoUrl(), cloudflare + QStringLiteral("cam1"), 5000);
    const int first = videoSettings->currentIndex();
    QCOMPARE(videoSettings->videoSourceNameAt(first), QString::fromUtf8(VideoSettings::videoSourceWebRTC));
    QCOMPARE(videoSettings->videoSourceCount(), first + 3);
    QCOMPARE(videoSettings->videoUrlAt(first + 1), QStringLiteral("rtsp://127.0.0.1:8554/cam2"));
    QCOMPARE(videoSettings->videoUrlAt(first + 2), cloudflare + QStringLiteral("cam3"));

    _removeAircastLinkConfigs();
}

void AircastDeviceSetupTest::_aDeviceWithACloudAccountAlsoGetsTheCloudLink()
{
    FakeAircastd device;
    device.cloud = QJsonObject{{QStringLiteral("api"), QStringLiteral("https://api.dev.aircast.one")},
                               {QStringLiteral("deviceId"), QStringLiteral("d-42")},
                               {QStringLiteral("sfu"), QStringLiteral("https://sfu.dev.aircast.one")},
                               {QStringLiteral("configured"), true}};
    device.setDevice({QStringLiteral("cam1")}, {QStringLiteral("udps:0.0.0.0:14550")});
    ignoreLogMessage("API.QGCApplication.AppMessage", QtDebugMsg, QRegularExpression(QStringLiteral("Aircast cloud link")));

    _applySetupDeepLink(device);

    QTRY_COMPARE_WITH_TIMEOUT(_aircastLinkConfigs().size(), 2, 5000);
    const QList<LinkConfiguration*> configs = _aircastLinkConfigs();
    const auto cloudIt = std::find_if(configs.cbegin(), configs.cend(), [](LinkConfiguration *config) {
        return config->type() == LinkConfiguration::TypeAircastCloud;
    });
    QVERIFY(cloudIt != configs.cend());
    const AircastCloudConfiguration *cloudConfig = qobject_cast<AircastCloudConfiguration*>(*cloudIt);
    QVERIFY(cloudConfig);
    QCOMPARE(cloudConfig->name(), QStringLiteral("Aircast 127.0.0.1 (cloud)"));
    QCOMPARE(cloudConfig->apiBase(), QStringLiteral("https://api.dev.aircast.one"));
    QCOMPARE(cloudConfig->deviceId(), QStringLiteral("d-42"));
    QCOMPARE(cloudConfig->relayUrl(), QUrl(QStringLiteral("wss://api.dev.aircast.one/v1/mavlink/web/d-42/ws")));
    QVERIFY(cloudConfig->isAutoConnect());
    QCOMPARE(AircastAccount::instance()->apiBase(), QStringLiteral("https://api.dev.aircast.one"));
    QCOMPARE(VideoCloudFailover::cloudUrlFor(VideoManager::instance()->cloudFailover()->device(),
                                             QStringLiteral("rtsp://127.0.0.1:8554/cam1")),
             QStringLiteral("https://sfu.dev.aircast.one/api/v1/whep/d-42/cam1"));

    _removeAircastLinkConfigs();
}

void AircastDeviceSetupTest::_aDeviceWithoutACloudAccountGetsNoCloudLink()
{
    FakeAircastd device;
    device.setDevice({QStringLiteral("cam1")}, {QStringLiteral("udps:0.0.0.0:14550")});

    _applySetupDeepLink(device);

    QTRY_COMPARE_WITH_TIMEOUT(_aircastLinkConfigs().size(), 1, 5000);
    QCOMPARE(_aircastLinkConfigs().first()->type(), LinkConfiguration::TypeUdp);

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
    QTRY_COMPARE_WITH_TIMEOUT(videoSettings->currentVideoUrl(), QStringLiteral("rtsp://127.0.0.1:8554/front"), 5000);
    const auto reappliedHost = []() {
        const QList<LinkConfiguration*> configs = _aircastLinkConfigs();
        const UDPConfiguration *const udpConfig = configs.size() == 1 ? qobject_cast<UDPConfiguration*>(configs.first()) : nullptr;
        return udpConfig ? udpConfig->hostList() : QStringList();
    };
    QTRY_COMPARE_WITH_TIMEOUT(reappliedHost(), QStringList{QStringLiteral("127.0.0.1:14551")}, 5000);

    _removeAircastLinkConfigs();
}

void AircastDeviceSetupTest::_clientOnlyTelemetryEndpointCreatesNoLink()
{
    FakeAircastd device;
    device.setDevice({QStringLiteral("cam1")}, {QStringLiteral("udpc:10.0.0.5:14550")});

    expectLogMessage("API.QGCApplication", QtWarningMsg, QRegularExpression(QStringLiteral("no udps/tcps telemetry endpoint")));
    const qsizetype logged = LogManager::capturedMessages().size();
    _applySetupDeepLink(device);

    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    QTRY_COMPARE_WITH_TIMEOUT(videoSettings->currentVideoUrl(), QStringLiteral("rtsp://127.0.0.1:8554/cam1"), 5000);

    const auto warned = [logged]() {
        const QList<LogEntry> messages = LogManager::capturedMessages();
        return std::any_of(messages.begin() + qMin(logged, messages.size()), messages.end(), [](const LogEntry &entry) {
            return entry.message.contains(QStringLiteral("no udps/tcps telemetry endpoint"));
        });
    };
    QTRY_VERIFY_WITH_TIMEOUT(warned(), 5000);
    verifyExpectedLogMessage();
    QCOMPARE(_aircastLinkConfigs().size(), 0);
}

void AircastDeviceSetupTest::_staleReplyFromSupersededSetupIsIgnored()
{
    FakeAircastd deviceA;
    deviceA.responseDelayMs = 300;
    deviceA.setDevice({QStringLiteral("stale")}, {QStringLiteral("udps:0.0.0.0:14550")});

    FakeAircastd deviceB;
    deviceB.setDevice({QStringLiteral("fresh")}, {QStringLiteral("udps:0.0.0.0:14551")});

    QGCLoggingCategoryManager::instance()->setCategoryEnabled(QStringLiteral("API.QGCApplication"), true);
    const auto quiet = qScopeGuard([]() { QGCLoggingCategoryManager::instance()->setCategoryEnabled(QStringLiteral("API.QGCApplication"), false); });
    ignoreLogMessage("API.QGCApplication", QtDebugMsg, QRegularExpression(QStringLiteral("Aircast device setup")));
    const qsizetype logged = LogManager::capturedMessages().size();
    _applySetupDeepLink(deviceA);
    _applySetupDeepLink(deviceB);

    VideoSettings *videoSettings = SettingsManager::instance()->videoSettings();
    QTRY_COMPARE_WITH_TIMEOUT(videoSettings->currentVideoUrl(), QStringLiteral("rtsp://127.0.0.1:8554/fresh"), 5000);

    const QString staleHost = deviceA.hostWithPort();
    const auto ignoredStale = [logged, staleHost]() {
        const QList<LogEntry> messages = LogManager::capturedMessages();
        return std::count_if(messages.begin() + qMin(logged, messages.size()), messages.end(), [&staleHost](const LogEntry &entry) {
            return entry.message.contains(QStringLiteral("ignored a reply from a superseded setup")) && entry.message.contains(staleHost);
        });
    };
    QTRY_COMPARE_WITH_TIMEOUT(ignoredStale(), 2, 5000);
    QCOMPARE(videoSettings->currentVideoUrl(), QStringLiteral("rtsp://127.0.0.1:8554/fresh"));
    QCOMPARE(videoSettings->cameraName(videoSettings->currentIndex()), QStringLiteral("fresh (127.0.0.1)"));

    QTRY_COMPARE_WITH_TIMEOUT(_aircastLinkConfigs().size(), 1, 5000);
    const UDPConfiguration *udpConfig = qobject_cast<UDPConfiguration*>(_aircastLinkConfigs().first());
    QVERIFY(udpConfig);
    QCOMPARE(udpConfig->hostList(), QStringList{QStringLiteral("127.0.0.1:14551")});

    _removeAircastLinkConfigs();
}

UT_REGISTER_TEST(AircastDeviceSetupTest, TestLabel::Unit)
