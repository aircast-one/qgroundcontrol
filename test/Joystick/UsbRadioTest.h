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
    void _radioHearsThePayloadStateAndTheFlightModeNames_test();
    void _vehicleKindFollowsTheAirframe_test();
    void _unmappedSticksFallBackToTheFirstFourChannels_test();
    void _linesSplitAcrossReadsAreJoined_test();
    void _aLineSplicedByARadioBufferOverrunIsDroppedNotReadShifted_test();
    void _radioFliesAtMostEverySendPeriodAndHandsBackWhenQuiet_test();
    void _losingTheVehicleHandsBackAndANewVehicleIsFlown_test();
    void _statusSaysWhatToDoWhenTheRadioIsBusyOrSilent_test();
    void _radioCardIsFoundByItsToolsFolder_test();
    void _savedScriptReplacesTheOldOneAndItsCompiledCopy_test();
    void _aPortStreamingTheScriptIsARadioAndMavlinkIsNot_test();
    void _indicatorStateFollowsTheRadioAndWhoFlies_test();
    void _cameraChannelsSteerTheGimbalZoomPictureAndPalette_test();
    void _dropSwitchDropsOncePerFlipUpAndBomarmArmsIt_test();
    void _theSixPosOnlyPicksTheBayAndTheDropSwitchDropsIt_test();
    void _whileThePayloadIsSafeTheSixPosOnlyPicksTheBay_test();
    void _aBayDropsOnItsOwnServoOnlyWhereTheDroneHasOne_test();
    void _r2d2SixPosPicksABayOnlyOnAStep_test();
    void _theDropBoardsBeaconSaysWhichHoldersAreEmpty_test();
    void _cameraAndPayloadChannelsAreSettingsWithTheR2d2LayoutAsDefault_test();
    void _aFunctionOnTheFlightModeChannelIsOff_test();
    void _aSliderMovedBeforeTheGimbalIsFoundAimsOnceItIs_test();
    void _theTiltKnobAimsLikeThePitchSlider_test();
    void _theLookSwitchAimsDownOrAheadWhenItFlips_test();
    void _yawAloneKeepsThePitchTheLookSwitchSet_test();
    void _theYawSliderTurnsAtTheSetSpeedAndHoldsAtTheCentre_test();
    void _theStickSwitchHandsTheLeftStickToTheCamera_test();
    void _theDroneHoldsTheLeftStickCentredUntilEachStickPassesTheCentre_test();
    void _aModeWhereTheStickSetsTheThrottleKeepsTheStickOnTheDrone_test();
    void _aYawSliderLeftOffCentreTurnsNothingUntilItHasBeenBackToTheCentre_test();
    void _lookAheadTurnsTheCameraBackToStraightAhead_test();
    void _aPitchKnobRestingAtItsEndDoesNotKeepAimingTheGimbal_test();
    void _aRadioThatStopsMidTurnStopsTheCamera_test();
};
