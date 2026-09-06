#include "TCPLinkErrorTest.h"
#include "LinkManager.h"
#include "TCPLink.h"

#include <QtNetwork/QTcpServer>
#include <QtTest/QTest>

namespace {

quint16 closedPort()
{
    QTcpServer server;
    server.listen(QHostAddress::LocalHost);
    const quint16 port = server.serverPort();
    server.close();
    return port;
}

SharedLinkConfigurationPtr addTcpConfig(const QString &name, const QString &host)
{
    TCPConfiguration *const config = new TCPConfiguration(name);
    config->setHost(host);
    config->setPort(closedPort());
    return LinkManager::instance()->addConfiguration(config);
}

void connectAndWaitForFailure(SharedLinkConfigurationPtr config)
{
    SharedLinkConfigurationPtr shared = config;
    QVERIFY(LinkManager::instance()->createConnectedLink(shared));
    QTRY_VERIFY_WITH_TIMEOUT(!config->lastError().isEmpty(), 5000);
}

} // namespace

void TCPLinkErrorTest::_refusedConnectionAsksForTheAddress()
{
    const SharedLinkConfigurationPtr config = addTcpConfig(QStringLiteral("refused"), QStringLiteral("127.0.0.1"));
    connectAndWaitForFailure(config);

    QVERIFY2(config->lastError().contains(QStringLiteral("nothing is listening")), qPrintable(config->lastError()));
    QCOMPARE(config->lastErrorRemedy(), LinkConfiguration::RemedyEditAddress);
    QCOMPARE(LinkManager::instance()->failedLink(), config.get());

    LinkManager::instance()->removeConfiguration(config.get());
    QVERIFY(!LinkManager::instance()->failedLink());
}

void TCPLinkErrorTest::_missingAddressIsNamed()
{
    const SharedLinkConfigurationPtr config = addTcpConfig(QStringLiteral("blank"), QString());
    connectAndWaitForFailure(config);

    QCOMPARE(config->lastError(), QStringLiteral("blank has no address."));
    QCOMPARE(config->lastErrorRemedy(), LinkConfiguration::RemedyEditAddress);

    LinkManager::instance()->removeConfiguration(config.get());
}

void TCPLinkErrorTest::_newFailureClearsTheOldOne()
{
    const SharedLinkConfigurationPtr stale = addTcpConfig(QStringLiteral("stale"), QStringLiteral("127.0.0.1"));
    stale->setLastError(QStringLiteral("old failure"));
    const SharedLinkConfigurationPtr fresh = addTcpConfig(QStringLiteral("fresh"), QStringLiteral("127.0.0.1"));
    connectAndWaitForFailure(fresh);

    QVERIFY(stale->lastError().isEmpty());
    QCOMPARE(LinkManager::instance()->failedLinkName(), QStringLiteral("fresh"));

    LinkManager::instance()->removeConfiguration(stale.get());
    LinkManager::instance()->removeConfiguration(fresh.get());
}
