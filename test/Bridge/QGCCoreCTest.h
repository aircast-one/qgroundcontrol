#pragma once

#include "UnitTest.h"

class QGCCoreCTest : public UnitTest
{
    Q_OBJECT

private slots:
    void init();
    void cleanup();

    void _viewMessagesReachTheHeadThroughTheRustCore();
    void _viewPathsAreReadOnlyAtTheCAbi();
    void _viewFieldsProjectAndNameTheUnknown();
    void _clientsWatchIndependently();
    void _planViewFollowsTheVehicle();
    void _guidedActionsFollowTheVehicle();
    void _guidedAltitudeTakesATarget();
    void _takeoffAndSpeedRangesFollowTheVehicle();
    void _batteryAndPreflightFollowTheVehicle();
    void _warningsFollowTheVehicle();
    void _labelsAreHumanised();
    void _instrumentsResolveTheSelection();
    void _vibrationBandsAreServed();
    void _sensorHealthIsOrdered();
    void _controlsDescribeAFact();
    void _linksAreListedAndTheFormValidates();
    void _mapScaleFollowsTheUnitSetting();
    void _terrainProfileReadsThePlan();
    void _missionKindsAndSeedsAreServed();
    void _calibrationIsListedWithoutAnApmVehicle();
    void _radioFollowsTheVehicle();
    void _logsFollowTheController();
    void _inspectorListsMessages();
    void _flightModesFollowTheVehicle();
    void _settingsPagesDecodeTheirControls();
    void _surveyStatsNeedAnItem();

private:
    static bool _unavailable(const char *path);
};
