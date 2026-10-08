#pragma once

#include "UnitTest.h"

class VideoCameraSwitchTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _onlyTheActiveAndPipCamerasArePlayed();
    void _multiViewOffGatesInactiveCameras();
    void _widgetRoles();
    void _pipCameraNumbers();
    void _nativeChannelsFollowTheActiveAndPipCameras();
    void _pipAndSwitchSkipCamerasWithNoAddress();
    void _pipNeverSharesTheMainCamerasStream();
    void _oneCameraTypedTwoWaysIsOneStream();
    void _pipSkipsCamerasOnlyThisComputerCanOpen();
    void _pipSlotNamesThePipChoice();
    void _streamOffAndOnStartsAgainAtConnecting();
    void _aReceiverLeavingItsChannelDetachesIt();
    void _cameraSignalsFollowTheReceivers();
    void _urlWhitespaceIsTrimmed();
    void _currentCameraFallsBackToTheFirstUsable();
    void _adoptingReplacesTheSameCamera();
    void _deviceSetupReplacesOnlyThatHost();
    void _listEditsKeepTheCameraOnScreen();
    void _droneCameraIsLiveOnly();
    void _unreadableListIsLeftAlone();
    void _onlyPlayableKindsAreOffered();
    void _addressesAreCheckedAndCleanedLikeTheCore();
    void _droneCameraNamesFollowTheCore();
    void _storedListAndIndexAreSeenTogether();
    void _videoStatsReadLikeTheWatchPage();
};
