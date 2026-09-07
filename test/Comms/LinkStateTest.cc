#include "LinkStateTest.h"
#include "LinkManager.h"
#include "TCPLink.h"

#include <QtNetwork/QTcpServer>
#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

namespace {

SharedLinkConfigurationPtr addLocalTcpConfig(const QString &name, quint16 port)
{
    TCPConfiguration *const config = new TCPConfiguration(name);
    config->setHost(QStringLiteral("127.0.0.1"));
    config->setPort(port);
    return LinkManager::instance()->addConfiguration(config);
}

quint16 closedPort()
{
    QTcpServer server;
    server.listen(QHostAddress::LocalHost);
    const quint16 port = server.serverPort();
    server.close();
    return port;
}

void connectAndWaitForLink(SharedLinkConfigurationPtr config)
{
    SharedLinkConfigurationPtr shared = config;
    QVERIFY(LinkManager::instance()->createConnectedLink(shared));
    QTRY_VERIFY_WITH_TIMEOUT(config->link() && config->link()->isConnected(), 5000);
}

} // namespace

void LinkStateTest::_connectingNameFollowsTheLink()
{
    QTcpServer silent;
    QVERIFY(silent.listen(QHostAddress::LocalHost));
    const SharedLinkConfigurationPtr config = addLocalTcpConfig(QStringLiteral("silent"), silent.serverPort());

    QSignalSpy nameChanged(LinkManager::instance(), &LinkManager::connectingLinkNameChanged);
    connectAndWaitForLink(config);
    QVERIFY(nameChanged.count() >= 1);
    QCOMPARE(LinkManager::instance()->connectingLinkName(), QStringLiteral("silent"));

    config->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(LinkManager::instance()->connectingLinkName().isEmpty(), 5000);

    LinkManager::instance()->removeConfiguration(config.get());
}

void LinkStateTest::_stallFiresWhileNothingAnswersAndClearsOnDisconnect()
{
    QTcpServer silent;
    QVERIFY(silent.listen(QHostAddress::LocalHost));
    const SharedLinkConfigurationPtr config = addLocalTcpConfig(QStringLiteral("stalled"), silent.serverPort());

    // The stall timer starts when connecting starts, not when the socket is up, and
    // connectAndWaitForLink polls for the connection. A 50ms threshold therefore
    // expires during the connect itself on anything but an idle machine, so the
    // "not stalled yet" assertion below fired spuriously. The threshold has to
    // comfortably outlast a loopback connect for that assertion to mean anything.
    LinkManager::instance()->setConnectingStallMSecs(1500);
    QSignalSpy stalledChanged(LinkManager::instance(), &LinkManager::connectingStalledChanged);
    connectAndWaitForLink(config);
    QVERIFY(!LinkManager::instance()->connectingStalled());
    QTRY_VERIFY_WITH_TIMEOUT(LinkManager::instance()->connectingStalled(), 10000);
    QCOMPARE(stalledChanged.count(), 1);

    config->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(!LinkManager::instance()->connectingStalled(), 5000);
    QTRY_VERIFY_WITH_TIMEOUT(LinkManager::instance()->connectingLinkName().isEmpty(), 5000);

    LinkManager::instance()->setConnectingStallMSecs(10000);
    LinkManager::instance()->removeConfiguration(config.get());
}

void LinkStateTest::_newConnectClearsTheFailure()
{
    const SharedLinkConfigurationPtr broken = addLocalTcpConfig(QStringLiteral("broken"), closedPort());
    SharedLinkConfigurationPtr shared = broken;
    QVERIFY(LinkManager::instance()->createConnectedLink(shared));
    QTRY_VERIFY_WITH_TIMEOUT(LinkManager::instance()->failedLink() == broken.get(), 5000);
    QVERIFY(!broken->lastError().isEmpty());

    QTcpServer silent;
    QVERIFY(silent.listen(QHostAddress::LocalHost));
    const SharedLinkConfigurationPtr working = addLocalTcpConfig(QStringLiteral("working"), silent.serverPort());
    QSignalSpy failedChanged(LinkManager::instance(), &LinkManager::failedLinkChanged);
    connectAndWaitForLink(working);
    QVERIFY(!LinkManager::instance()->failedLink());
    QCOMPARE(failedChanged.count(), 1);
    QVERIFY(working->lastError().isEmpty());

    working->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(LinkManager::instance()->connectingLinkName().isEmpty(), 5000);
    LinkManager::instance()->removeConfiguration(broken.get());
    LinkManager::instance()->removeConfiguration(working.get());
}
