#include "MissingParameterNoticeTest.h"

#include "QGCApplication.h"
#include "QGCMAVLink.h"
#include "Vehicle.h"

void MissingParameterNoticeTest::_missingParameterIsAVehicleWarningNotADialog()
{
    QVERIFY(vehicle());

    qgcApp()->reportMissingParameter(MAV_COMP_ID_AUTOPILOT1, QStringLiteral("COMPASS_DEV_ID3"));

    QTRY_VERIFY_WITH_TIMEOUT(vehicle()->formattedMessages().contains(QStringLiteral("Missing params: 1:COMPASS_DEV_ID3")),
                             TestTimeout::mediumMs());
    QVERIFY(vehicle()->formattedMessages().contains(QStringLiteral("Warning: Parameters are missing from firmware")));
}

void MissingParameterNoticeTest::_theSameGroundStationWarningIsLoggedOnce()
{
    QVERIFY(vehicle());

    vehicle()->addGroundStationWarning(QStringLiteral("Ground station note"));
    vehicle()->addGroundStationWarning(QStringLiteral("Ground station note"));

    QCOMPARE(vehicle()->formattedMessages().count(QStringLiteral("Ground station note")), 1);
}

UT_REGISTER_TEST(MissingParameterNoticeTest, TestLabel::Integration, TestLabel::Vehicle)
