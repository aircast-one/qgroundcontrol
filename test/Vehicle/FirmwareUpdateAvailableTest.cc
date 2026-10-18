#include "FirmwareUpdateAvailableTest.h"
#include "MockLink.h"
#include "MultiVehicleManager.h"
#include "Vehicle.h"

#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

void FirmwareUpdateAvailableTest::_theVersionStartsEmptySoNothingClaimsAnUpdate()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);
    QCOMPARE(vehicle->latestStableFirmwareVersion(), QString());
}

void FirmwareUpdateAvailableTest::_settingTheVersionAnnouncesItOnce()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    QSignalSpy spy(vehicle, &Vehicle::latestStableFirmwareVersionChanged);
    vehicle->setLatestStableFirmwareVersion(QStringLiteral("4.7.1"));

    QCOMPARE(spy.count(), 1);
    QCOMPARE(vehicle->latestStableFirmwareVersion(), QStringLiteral("4.7.1"));
}

void FirmwareUpdateAvailableTest::_settingTheSameVersionAgainAnnouncesNothing()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    vehicle->setLatestStableFirmwareVersion(QStringLiteral("4.7.1"));

    QSignalSpy spy(vehicle, &Vehicle::latestStableFirmwareVersionChanged);
    vehicle->setLatestStableFirmwareVersion(QStringLiteral("4.7.1"));

    QCOMPARE(spy.count(), 0);
}

void FirmwareUpdateAvailableTest::_clearingTheVersionAnnouncesIt()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    vehicle->setLatestStableFirmwareVersion(QStringLiteral("4.7.1"));

    QSignalSpy spy(vehicle, &Vehicle::latestStableFirmwareVersionChanged);
    vehicle->setLatestStableFirmwareVersion(QString());

    QCOMPARE(spy.count(), 1);
    QCOMPARE(vehicle->latestStableFirmwareVersion(), QString());
}

UT_REGISTER_TEST(FirmwareUpdateAvailableTest, TestLabel::Integration, TestLabel::Vehicle)
