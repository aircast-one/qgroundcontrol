#include "ArduCopterOrbitTest.h"

#include <QtTest/QTest>

#include "ArduCopterFirmwarePlugin.h"

void ArduCopterOrbitTest::_clockwiseNorthOfCenterMovesEast()
{
    double north = 0;
    double east = 0;
    ArduCopterFirmwarePlugin::orbitVelocityNorthEast(0, true, 4.0, north, east);
    QVERIFY(qAbs(north) < 1e-9);
    QVERIFY(qAbs(east - 4.0) < 1e-9);
}

void ArduCopterOrbitTest::_counterClockwiseEastOfCenterMovesNorth()
{
    double north = 0;
    double east = 0;
    ArduCopterFirmwarePlugin::orbitVelocityNorthEast(90, false, 4.0, north, east);
    QVERIFY(qAbs(north - 4.0) < 1e-9);
    QVERIFY(qAbs(east) < 1e-9);
}

UT_REGISTER_TEST_LIGHTWEIGHT(ArduCopterOrbitTest, TestLabel::Unit, TestLabel::Vehicle)
