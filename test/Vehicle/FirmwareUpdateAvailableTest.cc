/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "FirmwareUpdateAvailableTest.h"
#include "MockLink.h"
#include "MultiVehicleManager.h"
#include "Vehicle.h"

#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

// An empty latestStableFirmwareVersion is what the Setup sidebar badge and the Firmware page's
// "Update available" line key off, so an accidental non-empty default would tell every pilot
// their firmware is stale. The dedupe matters because the version file is re-fetched per connect.
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
