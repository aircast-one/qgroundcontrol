#include "RadiomasterAx12Test.h"

#include <QtTest/QTest>

#include "LinkManager.h"
#include "QmlObjectListModel.h"
#include "RadiomasterAx12.h"
#include "SerialLink.h"

void RadiomasterAx12Test::_isRemoteMatchesManufacturerIgnoringCase()
{
    QVERIFY(RadiomasterAx12::isRemote(QStringLiteral("Radiomaster")));
    QVERIFY(RadiomasterAx12::isRemote(QStringLiteral("RADIOMASTER")));
    QVERIFY(!RadiomasterAx12::isRemote(QStringLiteral("Fishsemi")));
    QVERIFY(!RadiomasterAx12::isRemote(QString()));
}

void RadiomasterAx12Test::_defaultsToBuiltInSerialOnlyOnAx12WithElrsPort()
{
    const QStringList withElrs = {QStringLiteral("ttyS0"), QStringLiteral("ttyS1")};
    const QStringList withoutElrs = {QStringLiteral("ttyS0"), QStringLiteral("ttyHS1")};

    QVERIFY(RadiomasterAx12::defaultsToBuiltInSerial(true, withElrs));
    QVERIFY(!RadiomasterAx12::defaultsToBuiltInSerial(true, withoutElrs));
    QVERIFY(!RadiomasterAx12::defaultsToBuiltInSerial(true, {}));
    QVERIFY(!RadiomasterAx12::defaultsToBuiltInSerial(false, withElrs));
}

namespace {

QList<SerialConfiguration*> ttyS1Configs()
{
    QList<SerialConfiguration*> found;
    QmlObjectListModel* configs = LinkManager::instance()->linkConfigurations();
    for (int i = 0; i < configs->count(); i++) {
        auto* config = qobject_cast<LinkConfiguration*>(configs->get(i));
        if (config && config->type() == LinkConfiguration::TypeSerial &&
            static_cast<SerialConfiguration*>(config)->portName() == QStringLiteral("/dev/ttyS1")) {
            found.append(static_cast<SerialConfiguration*>(config));
        }
    }
    return found;
}

}  // namespace

void RadiomasterAx12Test::_ensureElrsLinkCreatesAutoConnectLinkOnce()
{
    QVERIFY(ttyS1Configs().isEmpty());

    QVERIFY(RadiomasterAx12::ensureElrsLink(LinkManager::instance()));
    QCOMPARE(ttyS1Configs().size(), 1);
    SerialConfiguration* created = ttyS1Configs().first();
    QVERIFY(created->isAutoConnect());
    QCOMPARE(created->baud(), RadiomasterAx12::kElrsBaudRate);
    QCOMPARE(created->name(), RadiomasterAx12::kElrsLinkName);

    QVERIFY(!RadiomasterAx12::ensureElrsLink(LinkManager::instance()));
    QCOMPARE(ttyS1Configs().size(), 1);

    LinkManager::instance()->removeConfiguration(created);
}

void RadiomasterAx12Test::_ensureElrsLinkReusesExistingTtyS1Link()
{
    auto* drone = new SerialConfiguration(QStringLiteral("Drone"));
    drone->setPortName(QStringLiteral("/dev/ttyS1"));
    drone->setBaud(115200);
    drone->setAutoConnect(false);
    (void) LinkManager::instance()->addConfiguration(drone);

    QVERIFY(RadiomasterAx12::ensureElrsLink(LinkManager::instance()));
    QCOMPARE(ttyS1Configs().size(), 1);
    QVERIFY(drone->isAutoConnect());
    QCOMPARE(drone->name(), QStringLiteral("Drone"));
    QCOMPARE(drone->baud(), 115200);

    LinkManager::instance()->removeConfiguration(drone);
}

UT_REGISTER_TEST(RadiomasterAx12Test, TestLabel::Unit, TestLabel::Comms)
