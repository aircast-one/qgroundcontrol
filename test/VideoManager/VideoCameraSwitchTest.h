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
