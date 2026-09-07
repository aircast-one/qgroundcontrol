#include "LinkDuplicateConnectTest.h"
#include "LinkManager.h"
#include "TCPLink.h"

#include "QmlObjectListModel.h"

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

    config->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(!config->link(), 5000);

    LinkManager::instance()->removeConfiguration(config.get());
}

void LinkDuplicateConnectTest::_disconnectingADynamicLinkRemovesItsConfiguration()
{
    QTcpServer server;
    QVERIFY(server.listen(QHostAddress::LocalHost));

    TCPConfiguration *const raw = new TCPConfiguration(QStringLiteral("dynamic"));
    raw->setHost(QStringLiteral("127.0.0.1"));
    raw->setPort(server.serverPort());
    raw->setDynamic(true);
    SharedLinkConfigurationPtr config = LinkManager::instance()->addConfiguration(raw);

    QmlObjectListModel *const configs = LinkManager::instance()->linkConfigurations();
    QVERIFY(configs->contains(raw));

    QVERIFY(LinkManager::instance()->createConnectedLink(config));
    QTRY_VERIFY_WITH_TIMEOUT(config->link() && config->link()->isConnected(), 5000);

    config->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(!configs->contains(raw), 5000);
}

void LinkDuplicateConnectTest::_createAndConnectLinkRefusesADuplicateName()
{
    QTcpServer server;
    QVERIFY(server.listen(QHostAddress::LocalHost));
    SharedLinkConfigurationPtr config = addLocalTcpConfig(QStringLiteral("taken"), server.serverPort());

    QVERIFY(!LinkManager::instance()->createAndConnectLink(
        QStringLiteral("tcp"), QStringLiteral("taken"), QStringLiteral("127.0.0.1"),
        server.serverPort()));

    LinkManager::instance()->removeConfiguration(config.get());
}

void LinkDuplicateConnectTest::_createAndConnectLinkConnectsAndRegisters()
{
    QTcpServer server;
    QVERIFY(server.listen(QHostAddress::LocalHost));
    QmlObjectListModel *const configs = LinkManager::instance()->linkConfigurations();
    const int before = configs->count();

    QVERIFY(LinkManager::instance()->createAndConnectLink(
        QStringLiteral("tcp"), QStringLiteral("made-by-invokable"),
        QStringLiteral("127.0.0.1"), server.serverPort()));
    QCOMPARE(configs->count(), before + 1);

    QVERIFY(!LinkManager::instance()->createAndConnectLink(
        QStringLiteral("tcp"), QStringLiteral("no-host"), QString(), server.serverPort()));
    QVERIFY(!LinkManager::instance()->createAndConnectLink(
        QStringLiteral("tcp"), QStringLiteral("bad-port"), QStringLiteral("127.0.0.1"), 0));
    QCOMPARE(configs->count(), before + 1);

    LinkConfiguration *const made = qobject_cast<LinkConfiguration *>(configs->get(before));
    QVERIFY(made);
    QTRY_VERIFY_WITH_TIMEOUT(made->link() && made->link()->isConnected(), 5000);
    made->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(!made->link(), 5000);
    LinkManager::instance()->removeConfiguration(made);
}
