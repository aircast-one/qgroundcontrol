#pragma once

#include "UnitTest.h"

class GimbalGroundProjectionTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _straightDownHitsBelowTheCamera();
    void _centreRayFollowsGimbalYawAndPitch();
    void _offCentreTapsBendTheRay();
    void _horizonAndSkyHitNoGround();
    void _fovComesFromTheCameraSensor();
    void _groundPointsProjectBackToThePressedPixel();
};
