/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "MAVLinkLogManagerTest.h"
#include "MAVLinkLogManager.h"
#include "MultiVehicleManager.h"
#include "Vehicle.h"

#include <QtCore/QStandardPaths>
#include <QtTest/QTest>

void MAVLinkLogManagerTest::_testInitMAVLinkLogManager()
{
    _connectMockLinkNoInitialConnectSequence();

    MultiVehicleManager *const vehicleMgr = MultiVehicleManager::instance();
    Vehicle *const vehicle = vehicleMgr->activeVehicle();
    // MAVLinkLogManager's constructor calls vehicle->px4Firmware(), so a connect that did not
    // complete does not fail this test - it segfaults at 0xd24 and takes every suite after it
    // with it. Under load the vehicle is simply not there yet, and a timeout then reads as a
    // crashed run rather than as one slow test. Traced from a .ips after a recording refused at
    // 83 of 94 suites.
    QVERIFY(vehicle);
    MAVLinkLogManager *const mavlinkLogManager = new MAVLinkLogManager(vehicle, this);
    QVERIFY(mavlinkLogManager);
}
