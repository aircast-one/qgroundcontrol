#pragma once

#include "BaseClasses/VehicleTestManualConnect.h"

class MockLink;
class Vehicle;

class VehicleCameraControlTest : public VehicleTestManualConnect
{
    Q_OBJECT

private slots:
    void initTestCase() override;
    void init() override;
    void cleanup() override;

    UT_PARAMETERIZED_TEST(_testCameraCapFlags);
    void _testZoomTriggersCameraSettingsRequest();

private:
    MockLink* _mockLink = nullptr;
    Vehicle* _vehicle = nullptr;
};
