#pragma once

#include <memory>

#include "MissionControllerManagerTest.h"
#include "UnitTest.h"

class MissionController;
class PlanMasterController;
class VisualMissionItem;

class MissionControllerTest : public MissionControllerManagerTest
{
    Q_OBJECT

public:
    ~MissionControllerTest() override;

private slots:
    void cleanup() override;

    void _testLoadJsonSectionAvailable();
    void _testGlobalAltFrame();
    void _testFlightPathSegmentCacheReuse();
    void _testGimbalRecalc();
    void _testVehicleYawRecalc();
    void _testMissionReposition();
    void _testMissionOffset();
    void _testMissionRotate();
    void _testMissionTransformsInvalidHome();
    void _testLoadPlanRoundTripComplexItems();
    void _testInsertSurveyAppliesAltFrameInMixedMode();
    void _testInsertNonSurveyComplexItemMixedModeNoCrash();
    void _testInsertComplexItemFromKML();
    void _testInsertValidityHomePositionGating();
    void _testLandToolInsertsSingleRtl_data();
    void _testLandToolInsertsSingleRtl();
    void _testEmptyVehicleAPM();
    void _testEmptyVehiclePX4();
    void _testAddWaypointAPM();
    void _testAddWaypointPX4();
    void _testAddWaypointAtIndexAPM();
    void _testAddWaypointAtIndexPX4();
    void _testNewPatternStartsInWizardMode();
    void _testPatternFromFileIsReadyToSave();
    void _testPatternReadyMessageNamesItsOwnShape();

    // Parameterized tests - runs once per autopilot type
    UT_PARAMETERIZED_TEST(_testEmptyVehicle);

private:
    void _initForFirmwareType(MAV_AUTOPILOT firmwareType);
    void _testEmptyVehicleWorker(MAV_AUTOPILOT firmwareType);
    void _testAddWaypointWorker(MAV_AUTOPILOT firmwareType);
    void _testAddWaypointAtIndexWorker(MAV_AUTOPILOT firmwareType);
#if 0
    void _testOfflineToOnlineWorker(MAV_AUTOPILOT firmwareType);
#endif
    void _setupVisualItemSignals(VisualMissionItem* visualItem);

    std::unique_ptr<PlanMasterController> _masterController;
    MissionController* _missionController = nullptr;
};
