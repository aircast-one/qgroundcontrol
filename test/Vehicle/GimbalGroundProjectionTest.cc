#include "GimbalGroundProjectionTest.h"

#include <QtTest/QTest>
#include <cmath>

#include "GimbalController.h"

namespace {

const QGeoCoordinate kCamera(41.7100, 44.7900, 650.0);

void verifyHit(const QGeoCoordinate& hit, double expectedDistance, double expectedAzimuth)
{
    QVERIFY(hit.isValid());
    QVERIFY2(qAbs(kCamera.distanceTo(hit) - expectedDistance) < 0.05,
             qPrintable(QString::number(kCamera.distanceTo(hit))));
    if (expectedDistance > 0) {
        QVERIFY2(qAbs(std::remainder(kCamera.azimuthTo(hit) - expectedAzimuth, 360.0)) < 0.05,
                 qPrintable(QString::number(kCamera.azimuthTo(hit))));
    }
}

}  // namespace

void GimbalGroundProjectionTest::_straightDownHitsBelowTheCamera()
{
    verifyHit(GimbalController::projectToGround(kCamera, 100, 0, -90, 90, 45, 0, 0), 0, 0);
}

void GimbalGroundProjectionTest::_centreRayFollowsGimbalYawAndPitch()
{
    verifyHit(GimbalController::projectToGround(kCamera, 100, 90, -45, 90, 45, 0, 0), 100, 90);
}

void GimbalGroundProjectionTest::_offCentreTapsBendTheRay()
{
    verifyHit(GimbalController::projectToGround(kCamera, 100, 0, -45, 90, 45, 1, 0), 173.205, 54.7356);
    verifyHit(GimbalController::projectToGround(kCamera, 100, 0, -45, 90, 45, 0, -1), 41.4214, 0);
}

void GimbalGroundProjectionTest::_horizonAndSkyHitNoGround()
{
    QVERIFY(!GimbalController::projectToGround(kCamera, 100, 0, 0, 90, 45, 0, 0).isValid());
    QVERIFY(!GimbalController::projectToGround(kCamera, 100, 0, -15, 90, 45, 0, 1).isValid());
    QVERIFY(!GimbalController::projectToGround(kCamera, 0, 0, -45, 90, 45, 0, 0).isValid());
}

UT_REGISTER_TEST_LIGHTWEIGHT(GimbalGroundProjectionTest, TestLabel::Unit, TestLabel::Vehicle)
