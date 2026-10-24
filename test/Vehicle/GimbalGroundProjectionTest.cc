#include "GimbalGroundProjectionTest.h"

#include <QtTest/QTest>
#include <cmath>

#include "GimbalController.h"
#include "QGCCameraManager.h"

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

void GimbalGroundProjectionTest::_fovComesFromTheCameraSensor()
{
    double hfov = 0;
    double vfov = 0;
    QVERIFY(QGCCameraManager::fovFromSensor(4.0, 4.6188, 0, 1280, 720, hfov, vfov));
    QVERIFY(qAbs(hfov - 60.0) < 0.01);
    QVERIFY(qAbs(vfov - 35.98) < 0.01);

    QVERIFY(QGCCameraManager::fovFromSensor(4.0, 4.6188, 2.0, 0, 0, hfov, vfov));
    QVERIFY(qAbs(vfov - 28.07) < 0.01);

    QVERIFY(!QGCCameraManager::fovFromSensor(0, 4.6188, 2.0, 1280, 720, hfov, vfov));
    QVERIFY(!QGCCameraManager::fovFromSensor(4.0, 4.6188, 0, 0, 0, hfov, vfov));
}

void GimbalGroundProjectionTest::_groundPointsProjectBackToThePressedPixel()
{
    const QList<QPointF> pixels = {{0, 0}, {0.4, -0.3}, {-0.8, 0.5}, {0.9, -0.9}};
    for (const QPointF& pixel : pixels) {
        const QGeoCoordinate ground =
            GimbalController::projectToGround(kCamera, 80, 37, -35, 60, 36, pixel.x(), pixel.y());
        QVERIFY(ground.isValid());
        QPointF back;
        QVERIFY(GimbalController::projectToVideo(kCamera, 80, 37, -35, 60, 36, ground, back));
        QVERIFY2(qAbs(back.x() - pixel.x()) < 0.001, qPrintable(QString::number(back.x())));
        QVERIFY2(qAbs(back.y() - pixel.y()) < 0.001, qPrintable(QString::number(back.y())));
    }

    QPointF behind;
    QVERIFY(!GimbalController::projectToVideo(kCamera, 80, 37, -35, 60, 36, kCamera.atDistanceAndAzimuth(200, 217),
                                              behind));
}

UT_REGISTER_TEST_LIGHTWEIGHT(GimbalGroundProjectionTest, TestLabel::Unit, TestLabel::Vehicle)
