#include "AircastCloudLinkTest.h"

#include <QtCore/QSettings>
#include <QtNetwork/QHostAddress>
#include <QtTest/QSignalSpy>
#include <QtTest/QTest>
#include <QtWebSockets/QWebSocket>
#include <QtWebSockets/QWebSocketServer>

#include "AircastCloudLink.h"
#include "LinkManager.h"

namespace {

void removeCloudConfig(const SharedLinkConfigurationPtr& config)
{
    if (LinkInterface* const link = config->link()) {
        link->disconnect();
    }
    LinkManager::instance()->removeConfiguration(config.get());
    QTRY_VERIFY_WITH_TIMEOUT(LinkManager::instance()->links().isEmpty(), 5000);
}

SharedLinkConfigurationPtr addCloudConfig(const QString& apiBase)
{
    AircastCloudConfiguration* const config = new AircastCloudConfiguration(QStringLiteral("cloud under test"));
    config->setApiBase(apiBase);
    config->setDeviceId(QStringLiteral("d-1"));
    return LinkManager::instance()->addConfiguration(config);
}

}  // namespace

void AircastCloudLinkTest::_carriesMAVLinkBothWaysWithTheAccountToken()
{
    QWebSocketServer relay(QStringLiteral("relay"), QWebSocketServer::NonSecureMode);
    QVERIFY(relay.listen(QHostAddress::LocalHost));
    const QString apiBase = QStringLiteral("http://127.0.0.1:%1").arg(relay.serverPort());
    QSettings().setValue(QStringLiteral("AircastAccount/tokens/127.0.0.1"), QStringLiteral("session-token"));

    QSignalSpy connections(&relay, &QWebSocketServer::newConnection);
    const SharedLinkConfigurationPtr config = addCloudConfig(apiBase);
    SharedLinkConfigurationPtr shared = config;
    QVERIFY(LinkManager::instance()->createConnectedLink(shared));
    QVERIFY(connections.wait(5000));

    QWebSocket* const device = relay.nextPendingConnection();
    QVERIFY(device);
    QCOMPARE(device->requestUrl().path(), QStringLiteral("/v1/mavlink/web/d-1/ws"));
    QCOMPARE(device->request().rawHeader("Authorization"), QByteArrayLiteral("Bearer session-token"));

    LinkInterface* const link = config->link();
    QVERIFY(link);
    QTRY_VERIFY_WITH_TIMEOUT(link->isConnected(), 5000);

    QSignalSpy received(link, &LinkInterface::bytesReceived);
    const QByteArray heartbeat = QByteArray::fromHex("fd09000001010100000000000000000000030351040303");
    (void) device->sendBinaryMessage(QByteArray::fromHex("0001") + QByteArrayLiteral("{\"type\":\"relay_status\"}"));
    (void) device->sendBinaryMessage(heartbeat);
    QTRY_COMPARE_WITH_TIMEOUT(received.count(), 1, 5000);
    QCOMPARE(received.first().at(1).toByteArray(), heartbeat);

    QSignalSpy fromGcs(device, &QWebSocket::binaryMessageReceived);
    const QByteArray command = QByteArray::fromHex("fd21000002ffbe4c00");
    link->writeBytesThreadSafe(command.constData(), static_cast<int>(command.size()));
    QVERIFY(fromGcs.wait(5000));
    QCOMPARE(fromGcs.first().at(0).toByteArray(), command);

    QVERIFY(link->isRelayed());
    removeCloudConfig(config);
    QSettings().remove(QStringLiteral("AircastAccount"));
}

void AircastCloudLinkTest::_withoutSigningInItAsksToSignIn()
{
    QSettings().remove(QStringLiteral("AircastAccount"));
    const SharedLinkConfigurationPtr config = addCloudConfig(QStringLiteral("https://api.example.invalid"));
    SharedLinkConfigurationPtr shared = config;
    QVERIFY(LinkManager::instance()->createConnectedLink(shared));

    QTRY_VERIFY_WITH_TIMEOUT(!config->lastError().isEmpty(), 5000);
    QVERIFY2(config->lastError().contains(QStringLiteral("Sign in")), qPrintable(config->lastError()));

    removeCloudConfig(config);
}

UT_REGISTER_TEST(AircastCloudLinkTest, TestLabel::Unit)
