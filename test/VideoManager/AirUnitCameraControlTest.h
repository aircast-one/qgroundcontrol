#pragma once

#include "UnitTest.h"

class AirUnitCameraControlTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _cameraHeartbeatMakesItAvailable();
    void _legacyStreamInformationReportsActiveInput();
    void _selectInputSendsStartStreamingToTheCamera();
    void _switchInputCyclesThroughInputs();
    void _switchInputAlternatesWithoutStreamInformation();
    void _refusedSwitchRevertsTheInput();
};
