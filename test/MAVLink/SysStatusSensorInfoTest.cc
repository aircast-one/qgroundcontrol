/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "SysStatusSensorInfoTest.h"
#include "SysStatusSensorInfo.h"
#include "MAVLinkLib.h"

#include <QtTest/QTest>

void SysStatusSensorInfoTest::_testOrderingAndStatus()
{
    mavlink_sys_status_t sysStatus{};
    sysStatus.onboard_control_sensors_present = MAV_SYS_STATUS_SENSOR_GPS | MAV_SYS_STATUS_SENSOR_3D_GYRO | MAV_SYS_STATUS_GEOFENCE;
    sysStatus.onboard_control_sensors_enabled = MAV_SYS_STATUS_SENSOR_GPS | MAV_SYS_STATUS_SENSOR_3D_GYRO;
    sysStatus.onboard_control_sensors_health  = MAV_SYS_STATUS_SENSOR_3D_GYRO;

    SysStatusSensorInfo sensorInfo;
    sensorInfo.update(sysStatus);

    const QStringList  names   = sensorInfo.sensorNames();
    const QStringList  status  = sensorInfo.sensorStatus();
    const QVariantList healthy = sensorInfo.sensorHealthy();
    const QVariantList enabled = sensorInfo.sensorEnabled();

    QCOMPARE(names.count(), 3);
    QCOMPARE(status.count(), names.count());
    QCOMPARE(healthy.count(), names.count());
    QCOMPARE(enabled.count(), names.count());

    QCOMPARE(names[0], QStringLiteral("GPS"));
    QCOMPARE(healthy[0].toBool(), false);
    QCOMPARE(enabled[0].toBool(), true);

    QCOMPARE(names[1], QStringLiteral("Gyro"));
    QCOMPARE(healthy[1].toBool(), true);
    QCOMPARE(enabled[1].toBool(), true);

    QCOMPARE(names[2], QStringLiteral("Geofence"));
    QCOMPARE(healthy[2].toBool(), false);
    QCOMPARE(enabled[2].toBool(), false);

    QVERIFY(status[0] != status[1]);
    QVERIFY(status[1] != status[2]);
    QVERIFY(status[0] != status[2]);
}
