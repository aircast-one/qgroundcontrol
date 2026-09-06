#include "TCPConfigurationTest.h"
#include "TCPLink.h"

#include <QtTest/QTest>

void TCPConfigurationTest::_hostnameIsKeptVerbatim()
{
    TCPConfiguration config(QStringLiteral("test"));

    config.setHost(QStringLiteral("fix"));
    QCOMPARE(config.host(), QStringLiteral("fix"));
    QCOMPARE(config.summary(), QStringLiteral("fix:5760"));

    config.setHost(QStringLiteral("10.0.0.7"));
    QCOMPARE(config.host(), QStringLiteral("10.0.0.7"));
}

void TCPConfigurationTest::_copyFromCarriesHostAndPort()
{
    TCPConfiguration source(QStringLiteral("source"));
    source.setHost(QStringLiteral("drone.local"));
    source.setPort(5761);

    TCPConfiguration target(QStringLiteral("target"));
    target.copyFrom(&source);

    QCOMPARE(target.host(), QStringLiteral("drone.local"));
    QCOMPARE(target.port(), static_cast<quint16>(5761));
}
