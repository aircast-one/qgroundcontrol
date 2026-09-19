#pragma once

#include "UnitTest.h"

class VideoCameraSwitchTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _cameraToReceiverPinning();
    void _multiViewOffGatesInactiveCameras();
    void _widgetRoles();
    void _tileCameraNumbers();
    void _urlWhitespaceIsTrimmed();
};
