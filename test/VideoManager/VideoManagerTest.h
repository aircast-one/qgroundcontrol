#pragma once

#include "UnitTest.h"

class VideoManagerTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _cameraToReceiverPinning();
    void _multiViewOffGatesInactiveCameras();
    void _widgetRoles();
    void _tileCameraNumbers();
};
