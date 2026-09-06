/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "UDPConfigurationTest.h"
#include "UDPLink.h"
#include "AutoConnectSettings.h"
#include "SettingsManager.h"
#include "QuickInteractionTestHelpers.h"

#include <QtQml/QQmlContext>
#include <QtQml/QQmlEngine>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickView>
#include <QtTest/QTest>

namespace {

quint16 configuredListenPort()
{
    return static_cast<quint16>(SettingsManager::instance()->autoConnectSettings()->udpListenPort()->rawValue().toUInt());
}

class ListenPortScope
{
public:
    explicit ListenPortScope(quint16 port)
        : _fact(SettingsManager::instance()->autoConnectSettings()->udpListenPort())
        , _saved(_fact->rawValue())
    {
        _fact->setRawValue(port);
    }

    ~ListenPortScope() { _fact->setRawValue(_saved); }

private:
    Fact *const _fact;
    const QVariant _saved;
};

QQuickItem *loadUdpSettings(QQuickView &view, UDPConfiguration *config)
{
    view.engine()->rootContext()->setContextProperty(QStringLiteral("udpConfig"), config);
    if (!loadTestView(view, QStringLiteral("qrc:/unittest/UdpSettingsTest.qml"))) {
        return nullptr;
    }

    QQuickItem *const loader = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("udpSettingsLoader"));
    return loader ? loader->property("item").value<QQuickItem*>() : nullptr;
}

void setFieldText(QQuickItem *root, const QString &objectName, const QString &text)
{
    QQuickItem *const field = root->findChild<QQuickItem*>(objectName);
    QVERIFY2(field, qPrintable(QStringLiteral("missing field %1").arg(objectName)));
    field->setProperty("text", text);
}

} // namespace

void UDPConfigurationTest::_newConfigurationListensOnTheStandardMavlinkPort()
{
    const UDPConfiguration config(QStringLiteral("test"));

    QCOMPARE(config.localPort(), configuredListenPort());
    QVERIFY(config.localPort() != 0);
}

void UDPConfigurationTest::_newConfigurationFollowsTheConfiguredListenPort()
{
    const ListenPortScope scope(14599);
    const UDPConfiguration config(QStringLiteral("test"));

    QCOMPARE(config.localPort(), static_cast<quint16>(14599));
}

void UDPConfigurationTest::_hostIsParsedFromAnAddressPortString()
{
    UDPConfiguration config(QStringLiteral("test"));

    config.addHost(QStringLiteral("10.0.0.7:14551"));
    QCOMPARE(config.hostList(), QStringList{QStringLiteral("10.0.0.7:14551")});
}

void UDPConfigurationTest::_hostWithoutPortFallsBackToTheLocalPort()
{
    UDPConfiguration config(QStringLiteral("test"));
    config.setLocalPort(14562);

    config.addHost(QStringLiteral("10.0.0.8"));
    QCOMPARE(config.hostList(), QStringList{QStringLiteral("10.0.0.8:14562")});
}

void UDPConfigurationTest::_copyFromCarriesPortAndHosts()
{
    UDPConfiguration source(QStringLiteral("source"));
    source.setLocalPort(14570);
    source.addHost(QStringLiteral("10.0.0.9:14571"));

    UDPConfiguration target(QStringLiteral("target"));
    target.copyFrom(&source);

    QCOMPARE(target.localPort(), static_cast<quint16>(14570));
    QCOMPARE(target.hostList(), QStringList{QStringLiteral("10.0.0.9:14571")});
}

void UDPConfigurationTest::_typedServerAddressIsCommittedOnSave()
{
    UDPConfiguration config(QStringLiteral("test"));
    QQuickView view;
    QQuickItem *const settings = loadUdpSettings(view, &config);
    QVERIFY(settings);

    setFieldText(settings, QStringLiteral("udpHostField"), QStringLiteral("10.0.0.7:14551"));
    QVERIFY(QMetaObject::invokeMethod(settings, "saveSettings"));

    QCOMPARE(config.hostList(), QStringList{QStringLiteral("10.0.0.7:14551")});
}

void UDPConfigurationTest::_typedPortIsCommittedOnSave()
{
    UDPConfiguration config(QStringLiteral("test"));
    QQuickView view;
    QQuickItem *const settings = loadUdpSettings(view, &config);
    QVERIFY(settings);

    setFieldText(settings, QStringLiteral("udpPortField"), QStringLiteral("14588"));
    QVERIFY(QMetaObject::invokeMethod(settings, "saveSettings"));

    QCOMPARE(config.localPort(), static_cast<quint16>(14588));
}

void UDPConfigurationTest::_outOfRangePortFallsBackToTheDefault()
{
    UDPConfiguration config(QStringLiteral("test"));
    QQuickView view;
    QQuickItem *const settings = loadUdpSettings(view, &config);
    QVERIFY(settings);

    setFieldText(settings, QStringLiteral("udpPortField"), QStringLiteral("0"));
    QVERIFY(QMetaObject::invokeMethod(settings, "saveSettings"));
    QCOMPARE(config.localPort(), configuredListenPort());

    setFieldText(settings, QStringLiteral("udpPortField"), QStringLiteral("99999"));
    QVERIFY(QMetaObject::invokeMethod(settings, "saveSettings"));
    QCOMPARE(config.localPort(), configuredListenPort());
}
