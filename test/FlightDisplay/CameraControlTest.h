/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include "UnitTest.h"

class CameraControlTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _unmappedChannelsShowNoControls();
    void _mappedChannelsRevealTheirControls();
    void _rcSlidersAlternateEdgesClearOfTheCameraCluster();
    void _aimingIsIncrementalAndClamped();
    void _rcFallbackUsedWhenNoGimbalManager();
    void _aimDragDrivesTheGimbal();
    void _shutterAppearsForAMappedRecordChannel();
    void _pinchAndSliderShareOneZoomValue();
    void _yawModeButtonOnlyWithAGimbalManager();
    void _aimAreaMustNotHoldOntoTouchPoints();
    void _joystickSharingOneChannelForBothAxesIsRejected();
    void _shutterFollowsTheRecordChannelWhenTheStreamIsDead();
    void _settingsExposeTheChannelMapping();
    void _customRcControlsAppearForTheirChannels();
    void _threePositionSwitchCyclesThroughPositions();
    void _momentarySwitchTogglesCheckedOnPressAndRelease();
    void _sliderOrientationCanBeHorizontal();
    void _rotateBadgeFlipsSliderOrientationAndPersistsIt();
    void _malformedRcControlsSettingLeavesTheLayerEmptyAndIntact();
    void _addingAControlHighlightsItInTheFlyView();
    void _longPressHintHidesOnceEditModeIsDiscovered();
    void _momentarySwitchReleasesWhenEditModeInterruptsThePress();
    void _removingAControlOffersUndo();
    void _scrollToItemPositionsTheFlickableAtTheSection();
    void _pendingScrollFlagScrollsOnPageOpen();
    void _settingsFlagConflictingChannels();
    void _overrideIndicatorLaysOutForTheToolbar();
};
