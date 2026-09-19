#include "MAVLinkLogManagerTest.h"

#include <QtCore/QStandardPaths>

#include "MAVLinkLogManager.h"
#include "MultiVehicleManager.h"
#include "UnitTest.h"
#include "Vehicle.h"

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

UT_REGISTER_TEST(MAVLinkLogManagerTest, TestLabel::Integration, TestLabel::Vehicle)
