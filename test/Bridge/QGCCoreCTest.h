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
    void _fencesAndPolygonsAreServed();
    void _setupOverviewFollowsTheVehicle();
    void _videoAndCameraAreServed();
    void _viewShapesMatchTheRecordedContract();
    void _serialConfigurationsCanBeCreatedByPath();
    void _mapProvidersMatchTheRecordedHashes();
    void _theTileCacheSchemaMatchesTheRecordedOne();
    void _polygonGeometryMatchesTheRecordedOracle();
    void _structureScanFlightPathMatchesTheRecordedOracle();
    void _operatorNoticesReachAHeadWithNoQmlRoot();
    void _everyNameTheCoreHandsAHeadToInterpolateResolves();
    void _theCoreRefusesAMissionItemThePlanHasDecidedAgainst();
    void _theFlyViewControllerCountsTheMissionThePlanEditorCannot();
    void _everyRootTheCoreReadsFromIsRegistered();
    void _setupSeesTheComponentsTheVehicleReports();
    void _listsRecordedAsEmptyAreCheckedAgainstAVehicle();
    void _aMisspelledPropertyIsToldApartFromANullOne();
    void _missionStatisticsAnswerOnThePlanTheEditorHolds();
    void _structureScanItemsMatchTheRecordedUpload();
    void _aLargeSurveyMakesTheRoundTripUnchanged();
    void _surveyTransectsMatchTheRecordedOracle();
    void _tlogSummaryDecodesTheSampleLog();
    void _planFileAgreesWithTheCppLoader();
    void _waypointsFileAgreesWithTheCppLoader();
    void _missionFileAgreesWithTheCppLoader();
    void _setupPageServesApmParameters();
    void _coreUdpLinkFramesAPeer();
    void _coreBackedLinkBringsUpAVehicle();
    void _coreGuidedTakeoffReachesThePeer();
    void _detectionsFollowTheRtspUrl();
    void _coreConnectSequenceReachesParameters();
    void _replayedLogAgreesBetweenTheModels();
    void _planWrittenFromWaypointsLoadsInCpp();
    void _kmlFilesFollowTheMapPolygonTest();
    void _shapeFilesFollowShapeTest();
    void _geoConversionsMatchGeoTest();
    void _terrainTileNeedsAFile();

private:
    static bool _unavailable(const char *path);
};
