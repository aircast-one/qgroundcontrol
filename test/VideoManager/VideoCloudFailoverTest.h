#pragma once

#include "UnitTest.h"

class VideoCloudFailoverTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _cloudUrlForAnAircastCamera();
    void _aStalledStreamSwitchesToTheCloudCopyWithAViewToken();
    void _theDeviceAnsweringAgainSwitchesBack();
    void _withoutAViewTokenItStaysOnTheDevice();
    void _aDecodingStreamNeverFailsOver();
    void _choosingAnotherSourceDropsTheCloudCopy();
    void _failingOverSoonAfterReturningWaitsLongerBeforeTheNextTry();
};
