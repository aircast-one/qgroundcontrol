/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "FlightModeReportingTest.h"
#include "FirmwarePlugin.h"
#include "MockLink.h"
#include "MultiVehicleManager.h"
#include "Vehicle.h"

#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

void FlightModeReportingTest::_renamingTheModeAnnouncesItEvenThoughTheModeBitsAreUnchanged()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);
    QTRY_VERIFY_WITH_TIMEOUT(!vehicle->flightMode().isEmpty() && !vehicle->flightMode().startsWith(QStringLiteral("Mode ")), 5000);

    FirmwarePlugin* const plugin = vehicle->firmwarePlugin();
    QVERIFY(plugin);

    const FlightModeList original = plugin->flightModeList();
    const uint8_t  baseModeBefore   = vehicle->baseMode();
    const uint32_t customModeBefore = vehicle->customMode();
    const QString  nameBefore       = vehicle->flightMode();

    const QString renamedTo = QStringLiteral("RenamedByTest");
    FlightModeList renamed = original;
    bool renamedOne = false;
    for (FirmwareFlightMode &mode : renamed) {
        if (mode.custom_mode == customModeBefore) {
            mode.mode_name = renamedTo;
            renamedOne = true;
        }
    }
    QVERIFY2(renamedOne, "the vehicle's current mode is not in the firmware mode table");

    QSignalSpy spy(vehicle, &Vehicle::flightModeChanged);
    plugin->updateAvailableFlightModes(renamed);

    FlightModeList restore = original;
    const auto restoreTable = qScopeGuard([plugin, &restore]() { plugin->updateAvailableFlightModes(restore); });

    QTRY_VERIFY_WITH_TIMEOUT(!spy.isEmpty(), 5000);
    QCOMPARE(spy.last().at(0).toString(), renamedTo);

    QCOMPARE(vehicle->baseMode(), baseModeBefore);
    QCOMPARE(vehicle->customMode(), customModeBefore);
    QVERIFY(nameBefore != renamedTo);
}
