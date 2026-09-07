#include "LinkDuplicateConnectTest.h"
#include "LinkManager.h"
#include "TCPLink.h"

#include <QtNetwork/QTcpServer>
#include <QtTest/QTest>

namespace {

SharedLinkConfigurationPtr addLocalTcpConfig(const QString &name, quint16 port)
{
    TCPConfiguration *const config = new TCPConfiguration(name);
    config->setHost(QStringLiteral("127.0.0.1"));
    config->setPort(port);
    return LinkManager::instance()->addConfiguration(config);
}

} // namespace

void LinkDuplicateConnectTest::_connectingTwiceReusesTheSameLink()
{
    QTcpServer server;
    QVERIFY(server.listen(QHostAddress::LocalHost));
    SharedLinkConfigurationPtr config = addLocalTcpConfig(QStringLiteral("duplicate"), server.serverPort());

    QVERIFY(LinkManager::instance()->createConnectedLink(config));
    QTRY_VERIFY_WITH_TIMEOUT(config->link() && config->link()->isConnected(), 5000);
    LinkInterface *const first = config->link();

    QVERIFY(LinkManager::instance()->createConnectedLink(config));
    QCOMPARE(config->link(), first);

    config->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(!config->link(), 5000);
    LinkManager::instance()->removeConfiguration(config.get());
}

void LinkDuplicateConnectTest::_oneDisconnectClosesTheConfiguration()
{
    QTcpServer server;
    QVERIFY(server.listen(QHostAddress::LocalHost));
    SharedLinkConfigurationPtr config = addLocalTcpConfig(QStringLiteral("single"), server.serverPort());

    QVERIFY(LinkManager::instance()->createConnectedLink(config));
    QTRY_VERIFY_WITH_TIMEOUT(config->link() && config->link()->isConnected(), 5000);

    // One disconnect must be enough: every frontend keys its Connect/Disconnect
    // control off config->link().
    config->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(!config->link(), 5000);

    LinkManager::instance()->removeConfiguration(config.get());
}
