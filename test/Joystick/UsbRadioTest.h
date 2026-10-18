#pragma once

#include "UnitTest.h"

class UsbRadioTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _scriptLineReadsAsTheRadioShowsItsChannels_test();
    void _standbySendsToTheAircraftComputer_test();
    void _radioIsToldWhatItsSticksAreDoing_test();
    void _radioHearsBackOnlyWhileItStreams_test();
    void _radioLearnsTheVehicleKindAndItsStickChannels_test();
    void _vehicleKindFollowsTheAirframe_test();
    void _unmappedSticksFallBackToTheFirstFourChannels_test();
    void _linesSplitAcrossReadsAreJoined_test();
    void _radioFliesAtMostEverySendPeriodAndHandsBackWhenQuiet_test();
    void _losingTheVehicleHandsBackAndANewVehicleIsFlown_test();
};
