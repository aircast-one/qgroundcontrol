#include "QmlObjectListModel.h"
#include "MissionControllerTest.h"

#include "AppSettings.h"
#include "CameraCalc.h"
#include "CorridorScanComplexItem.h"
#include "FlightPathSegment.h"
#include "StructureScanComplexItem.h"
#include "SurveyComplexItem.h"
#include "UnitTestCoords.h"
#include "MissionController.h"
#include "MissionSettingsItem.h"
#include "PlanMasterController.h"
#include "PlanViewSettings.h"
#include "CorridorScanComplexItem.h"
#include "SurveyComplexItem.h"
#include "StructureScanComplexItem.h"

#include "SettingsManager.h"
#include "SimpleMissionItem.h"
#include "TestFixtures.h"
#include "MultiSignalSpy.h"

#include <QtCore/QRegularExpression>
#include <QtCore/QScopeGuard>
#include <QtCore/QTemporaryDir>
using namespace TestFixtures;

MissionControllerTest::~MissionControllerTest() = default;

namespace {
constexpr double kCoordToleranceMeters = 0.5;
constexpr double kAltToleranceMeters = 1e-4;
}  // namespace

void MissionControllerTest::cleanup()
{
    _masterController.reset();
    _missionController = nullptr;
    MissionControllerManagerTest::cleanup();
}

void MissionControllerTest::_initForFirmwareType(MAV_AUTOPILOT firmwareType)
{
    MissionControllerManagerTest::_initForFirmwareType(firmwareType);
    SettingsManager::instance()->appSettings()->offlineEditingFirmwareClass()->setRawValue(
        QGCMAVLink::firmwareClass(firmwareType));
    _masterController = std::make_unique<PlanMasterController>();
    _masterController->setFlyView(false);
    _missionController = _masterController->missionController();
    MultiSignalSpy missionControllerSpy;
    QVERIFY(missionControllerSpy.init(_missionController));
    _masterController->start();
    QVERIFY(missionControllerSpy.waitForSignal("visualItemsReset", TestTimeout::mediumMs()));
    QmlObjectListModel* visualItems = _missionController->visualItems();
    QVERIFY(visualItems);
    QCOMPARE(visualItems->count(), 1);
    MissionSettingsItem* settingsItem = visualItems->value<MissionSettingsItem*>(0);
    QVERIFY(settingsItem);
    QCOMPARE(settingsItem->coordinate().isValid(), false);
    QCOMPARE(settingsItem->childItems()->count(), 0);
    QmlObjectListModel* simpleFlightPathSegments = _missionController->simpleFlightPathSegments();
    QVERIFY(simpleFlightPathSegments);
    QCOMPARE(simpleFlightPathSegments->count(), 0);
}

void MissionControllerTest::_testEmptyVehicle_data()
{
    TestData::addAutopilotRows();
}

void MissionControllerTest::_testEmptyVehicle()
{
    UT_FETCH_AUTOPILOT();
    Q_UNUSED(autopilotName);
    ignoreLogMessage("FirmwarePlugin.ParameterMetaData", QtWarningMsg,
                     QRegularExpression("Skipping invalid enum value"));
    _initForFirmwareType(autopilot);
    QmlObjectListModel* visualItems = _missionController->visualItems();
    QVERIFY(visualItems);
    VisualMissionItem* visualItem = visualItems->value<VisualMissionItem*>(0);
    QVERIFY(visualItem);
    _setupVisualItemSignals(visualItem);
}

void MissionControllerTest::_setupVisualItemSignals(VisualMissionItem* visualItem)
{
    MultiSignalSpy visualItemSpy;
    QVERIFY(visualItemSpy.init(visualItem));
}

void MissionControllerTest::_testAddWaypointWorker(MAV_AUTOPILOT firmwareType)
{
    _initForFirmwareType(firmwareType);

    QCOMPARE(_missionController->visualItems()->count(), 1);

    _missionController->addWaypoint(QGeoCoordinate(37.803784, -122.462276), -1);

    QCOMPARE(_missionController->visualItems()->count(), 3);
    QCOMPARE(qobject_cast<SimpleMissionItem*>(_missionController->visualItems()->get(1))->mavCommand(), MAV_CMD_NAV_TAKEOFF);
    QCOMPARE(qobject_cast<SimpleMissionItem*>(_missionController->visualItems()->get(2))->mavCommand(), MAV_CMD_NAV_WAYPOINT);

    _missionController->addWaypoint(QGeoCoordinate(37.804784, -122.462276), -1);

    QCOMPARE(_missionController->visualItems()->count(), 4);
    QCOMPARE(qobject_cast<SimpleMissionItem*>(_missionController->visualItems()->get(3))->mavCommand(), MAV_CMD_NAV_WAYPOINT);
}

void MissionControllerTest::_testAddWaypointAtIndexWorker(MAV_AUTOPILOT firmwareType)
{
    _initForFirmwareType(firmwareType);

    _missionController->addWaypoint(QGeoCoordinate(37.803784, -122.462276), 1);

    QCOMPARE(_missionController->visualItems()->count(), 3);
    QCOMPARE(qobject_cast<SimpleMissionItem*>(_missionController->visualItems()->get(1))->mavCommand(), MAV_CMD_NAV_TAKEOFF);
    QCOMPARE(qobject_cast<SimpleMissionItem*>(_missionController->visualItems()->get(2))->mavCommand(), MAV_CMD_NAV_WAYPOINT);
}

void MissionControllerTest::_testNewPatternStartsInWizardMode(void)
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    VisualMissionItem* item = _missionController->insertComplexMissionItem(
                CorridorScanComplexItem::canonicalName, QGeoCoordinate(37.803784, -122.462276), -1);

    QVERIFY(item);
    QVERIFY(item->wizardMode());
}

void MissionControllerTest::_testPatternFromFileIsReadyToSave(void)
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    VisualMissionItem* item = _missionController->insertComplexMissionItemFromKMLOrSHP(
                SurveyComplexItem::canonicalName, QStringLiteral(":/unittest/PolygonGood.kml"), -1);

    QVERIFY(item);
    QVERIFY(!item->wizardMode());
    QCOMPARE(_missionController->readyForSaveState(), (int)VisualMissionItem::ReadyForSave);
}

void MissionControllerTest::_testPatternReadyMessageNamesItsOwnShape(void)
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    VisualMissionItem* corridor = _missionController->insertComplexMissionItem(
                CorridorScanComplexItem::canonicalName, QGeoCoordinate(37.803784, -122.462276), -1);
    VisualMissionItem* survey = _missionController->insertComplexMissionItem(
                SurveyComplexItem::canonicalName, QGeoCoordinate(37.803784, -122.462276), -1);
    VisualMissionItem* structure = _missionController->insertComplexMissionItem(
                StructureScanComplexItem::canonicalName, QGeoCoordinate(37.803784, -122.462276), -1);

    QCOMPARE(corridor->readyForSaveMessage(), QStringLiteral("Draw the corridor path"));
    QCOMPARE(survey->readyForSaveMessage(), QStringLiteral("Draw the area"));
    QCOMPARE(structure->readyForSaveMessage(), QStringLiteral("Draw the outline"));

    QVERIFY(!corridor->readyForSaveMessage().isEmpty());
    QCOMPARE(corridor->readyForSaveState(), VisualMissionItem::NotReadyForSaveData);
}

void MissionControllerTest::_testAddWaypointAtIndexAPM(void)
{
    if (!apmFirmwareSupported()) {
        QSKIP("ArduPilot support not registered in this build");
    }
    _testAddWaypointAtIndexWorker(MAV_AUTOPILOT_ARDUPILOTMEGA);
}

void MissionControllerTest::_testAddWaypointAtIndexPX4(void)
{
    _testAddWaypointAtIndexWorker(MAV_AUTOPILOT_PX4);
}

void MissionControllerTest::_testAddWaypointAPM(void)
{
    if (!apmFirmwareSupported()) {
        QSKIP("ArduPilot support not registered in this build");
    }
    _testAddWaypointWorker(MAV_AUTOPILOT_ARDUPILOTMEGA);
}

void MissionControllerTest::_testAddWaypointPX4(void)
{
    _testAddWaypointWorker(MAV_AUTOPILOT_PX4);
}

void MissionControllerTest::_testLandToolInsertsSingleRtl_data()
{
    QTest::addColumn<int>("vehicleClass");

    QTest::newRow("RoverBoat") << static_cast<int>(QGCMAVLink::VehicleClassRoverBoat);
    QTest::newRow("MultiRotor") << static_cast<int>(QGCMAVLink::VehicleClassMultiRotor);
}

void MissionControllerTest::_testLandToolInsertsSingleRtl()
{
    QFETCH(int, vehicleClass);

    AppSettings* appSettings = SettingsManager::instance()->appSettings();
    appSettings->offlineEditingFirmwareClass()->setRawValue(QGCMAVLink::firmwareClass(MAV_AUTOPILOT_ARDUPILOTMEGA));
    appSettings->offlineEditingVehicleClass()->setRawValue(vehicleClass);
    Fact* const allowMultipleLandingPatterns = SettingsManager::instance()->planViewSettings()->allowMultipleLandingPatterns();
    const QVariant savedAllowMultiple = allowMultipleLandingPatterns->rawValue();
    const auto restoreGuard = qScopeGuard([allowMultipleLandingPatterns, savedAllowMultiple] { allowMultipleLandingPatterns->setRawValue(savedAllowMultiple); });
    allowMultipleLandingPatterns->setRawValue(true);

    _masterController = std::make_unique<PlanMasterController>();
    _masterController->setFlyView(false);
    _missionController = _masterController->missionController();
    MultiSignalSpy missionControllerSpy;
    QVERIFY(missionControllerSpy.init(_missionController));
    _masterController->start();
    QVERIFY(missionControllerSpy.waitForSignal("visualItemsReset", TestTimeout::mediumMs()));

    _missionController->setHomePosition(Coord::zurich());

    if (!_missionController->property("isInsertLandValid").toBool()) {
        QVERIFY(_missionController->insertTakeoffItem(Coord::zurich(), 1, true /* makeCurrentItem */));
    }
    QCOMPARE(_missionController->property("isInsertLandValid").toBool(), true);

    VisualMissionItem* landItem = _missionController->insertLandItem(Coord::zurich(), -1, true /* makeCurrentItem */);
    SimpleMissionItem* simpleItem = qobject_cast<SimpleMissionItem*>(landItem);
    QVERIFY(simpleItem);
    QCOMPARE(simpleItem->mavCommand(), MAV_CMD_NAV_RETURN_TO_LAUNCH);

    QCOMPARE(_missionController->property("hasLandItem").toBool(), true);
    QCOMPARE(_missionController->property("isInsertLandValid").toBool(), false);
}

void MissionControllerTest::_testGimbalRecalc()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);
    const QList<QGeoCoordinate> waypoints = Coord::waypointPath(Coord::zurich(), 4);
    for (int i = 0; i < waypoints.count(); ++i) {
        _missionController->insertSimpleMissionItem(waypoints[i], i + 1);
    }
    for (int i = 1; i < _missionController->visualItems()->count(); i++) {
        VisualMissionItem* visualItem = _missionController->visualItems()->value<VisualMissionItem*>(i);
        QVERIFY(qIsNaN(visualItem->missionGimbalYaw()));
    }
    const int yawIndex = 2;
    SimpleMissionItem* item = _missionController->visualItems()->value<SimpleMissionItem*>(yawIndex);
    item->cameraSection()->setSpecifyGimbal(true);
    item->cameraSection()->gimbalYaw()->setRawValue(0.0);
    SettingsManager::instance()->planViewSettings()->showGimbalOnlyWhenSet()->setRawValue(false);
    QVERIFY_TRUE_WAIT(([&]() {
        for (int i = 1; i < _missionController->visualItems()->count(); i++) {
            VisualMissionItem* visualItem = _missionController->visualItems()->value<VisualMissionItem*>(i);
            if (i >= yawIndex) {
                if (!qFuzzyCompare(visualItem->missionGimbalYaw() + 1.0, 1.0)) {
                    return false;
                }
            } else if (!qIsNaN(visualItem->missionGimbalYaw())) {
                return false;
            }
        }
        return true;
    }()),
                      TestTimeout::mediumMs());
    for (int i = 1; i < _missionController->visualItems()->count(); i++) {
        VisualMissionItem* visualItem = _missionController->visualItems()->value<VisualMissionItem*>(i);
        if (i >= yawIndex) {
            QCOMPARE(visualItem->missionGimbalYaw(), 0.0);
        } else {
            QVERIFY(qIsNaN(visualItem->missionGimbalYaw()));
        }
    }
}

void MissionControllerTest::_testVehicleYawRecalc()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);
    double wpDistance = 1000;
    double wpAngleInc = 45;
    double wpAngle = 0;
    int cMissionItems = 4;
    QGeoCoordinate currentCoord(0, 0);
    _missionController->insertSimpleMissionItem(currentCoord, 1);
    for (int i = 2; i <= cMissionItems; i++) {
        wpAngle += wpAngleInc;
        currentCoord = currentCoord.atDistanceAndAzimuth(wpDistance, wpAngle);
        _missionController->insertSimpleMissionItem(currentCoord, i);
    }
    QVERIFY_TRUE_WAIT(([&]() {
        double expectedVehicleYaw = wpAngleInc;
        for (int i = 2; i < cMissionItems; i++) {
            VisualMissionItem* visualItem = _missionController->visualItems()->value<VisualMissionItem*>(i);
            if (!qFuzzyCompare(visualItem->missionVehicleYaw() + 1.0, expectedVehicleYaw + 1.0)) {
                return false;
            }
            if (i <= cMissionItems - 1) {
                expectedVehicleYaw += wpAngleInc;
            }
        }
        return true;
    }()),
                      TestTimeout::mediumMs());
    double expectedVehicleYaw = wpAngleInc;
    for (int i = 2; i < cMissionItems; i++) {
        VisualMissionItem* visualItem = _missionController->visualItems()->value<VisualMissionItem*>(i);
        QCOMPARE(visualItem->missionVehicleYaw(), expectedVehicleYaw);
        if (i <= cMissionItems - 1) {
            expectedVehicleYaw += wpAngleInc;
        }
    }
    SimpleMissionItem* simpleItem = _missionController->visualItems()->value<SimpleMissionItem*>(3);
    simpleItem->missionItem().setParam4(66);
    QVERIFY_TRUE_WAIT(([&]() {
        double expectedYaw = wpAngleInc;
        for (int i = 2; i < cMissionItems; i++) {
            VisualMissionItem* visualItem = _missionController->visualItems()->value<VisualMissionItem*>(i);
            const double expected = (i == 3) ? 66.0 : expectedYaw;
            if (!qFuzzyCompare(visualItem->missionVehicleYaw() + 1.0, expected + 1.0)) {
                return false;
            }
            if (i <= cMissionItems - 1) {
                expectedYaw += wpAngleInc;
            }
        }
        return true;
    }()),
                      TestTimeout::mediumMs());
    expectedVehicleYaw = wpAngleInc;
    for (int i = 2; i < cMissionItems; i++) {
        VisualMissionItem* visualItem = _missionController->visualItems()->value<VisualMissionItem*>(i);
        QCOMPARE(visualItem->missionVehicleYaw(), i == 3 ? 66.0 : expectedVehicleYaw);
        if (i <= cMissionItems - 1) {
            expectedVehicleYaw += wpAngleInc;
        }
    }
}

void MissionControllerTest::_testMissionReposition()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    MissionSettingsItem* settingsItem = _missionController->visualItems()->value<MissionSettingsItem*>(0);
    QVERIFY(settingsItem);

    const QGeoCoordinate home = Coord::zurich();
    settingsItem->setCoordinate(home);

    const QGeoCoordinate wp1 = home.atDistanceAndAzimuth(150.0, 15.0);
    const QGeoCoordinate wp2 = home.atDistanceAndAzimuth(320.0, 120.0);
    _missionController->insertSimpleMissionItem(wp1, 1);
    _missionController->insertSimpleMissionItem(wp2, 2);

    SimpleMissionItem* item1 = _missionController->visualItems()->value<SimpleMissionItem*>(1);
    SimpleMissionItem* item2 = _missionController->visualItems()->value<SimpleMissionItem*>(2);
    QVERIFY(item1);
    QVERIFY(item2);

    const double oldHomeAlt = settingsItem->editableAlt();
    const double oldAlt1 = item1->editableAlt();
    const double oldAlt2 = item2->editableAlt();

    const QGeoCoordinate newHome = home.atDistanceAndAzimuth(200.0, 65.0);
    const QGeoCoordinate expectedWp1 =
        newHome.atDistanceAndAzimuth(home.distanceTo(wp1), home.azimuthTo(wp1));
    const QGeoCoordinate expectedWp2 =
        newHome.atDistanceAndAzimuth(home.distanceTo(wp2), home.azimuthTo(wp2));
    _missionController->repositionMission(newHome, true, true);
    QVERIFY_TRUE_WAIT((settingsItem->coordinate().distanceTo(newHome) <= kCoordToleranceMeters) &&
                          (item1->coordinate().distanceTo(expectedWp1) <= kCoordToleranceMeters) &&
                          (item2->coordinate().distanceTo(expectedWp2) <= kCoordToleranceMeters),
                      TestTimeout::shortMs());

    QCOMPARE_COORDS(settingsItem->coordinate(), newHome, kCoordToleranceMeters);
    QCOMPARE_COORDS(item1->coordinate(), expectedWp1, kCoordToleranceMeters);
    QCOMPARE_COORDS(item2->coordinate(), expectedWp2, kCoordToleranceMeters);
    QCOMPARE_FUZZY(settingsItem->editableAlt(), oldHomeAlt, kAltToleranceMeters);
    QCOMPARE_FUZZY(item1->editableAlt(), oldAlt1, kAltToleranceMeters);
    QCOMPARE_FUZZY(item2->editableAlt(), oldAlt2, kAltToleranceMeters);
}

void MissionControllerTest::_testMissionOffset()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    MissionSettingsItem* settingsItem = _missionController->visualItems()->value<MissionSettingsItem*>(0);
    QVERIFY(settingsItem);

    const QGeoCoordinate home = Coord::zurich();
    settingsItem->setCoordinate(home);

    const QGeoCoordinate wp1 = home.atDistanceAndAzimuth(150.0, 15.0);
    const QGeoCoordinate wp2 = home.atDistanceAndAzimuth(320.0, 120.0);
    _missionController->insertSimpleMissionItem(wp1, 1);
    _missionController->insertSimpleMissionItem(wp2, 2);

    SimpleMissionItem* item1 = _missionController->visualItems()->value<SimpleMissionItem*>(1);
    SimpleMissionItem* item2 = _missionController->visualItems()->value<SimpleMissionItem*>(2);
    QVERIFY(item1);
    QVERIFY(item2);

    const double oldHomeAlt = settingsItem->editableAlt();
    const double oldAlt1 = item1->editableAlt();
    const double oldAlt2 = item2->editableAlt();

    _missionController->offsetMission(0.0, 0.0, 12.0, true, true);
    QVERIFY_TRUE_WAIT(qAbs(settingsItem->editableAlt() - oldHomeAlt) <= kAltToleranceMeters &&
                          qAbs(item1->editableAlt() - (oldAlt1 + 12.0)) <= kAltToleranceMeters &&
                          qAbs(item2->editableAlt() - (oldAlt2 + 12.0)) <= kAltToleranceMeters,
                      TestTimeout::shortMs());

    QCOMPARE_FUZZY(settingsItem->editableAlt(), oldHomeAlt, kAltToleranceMeters);
    QCOMPARE_FUZZY(item1->editableAlt(), oldAlt1 + 12.0, kAltToleranceMeters);
    QCOMPARE_FUZZY(item2->editableAlt(), oldAlt2 + 12.0, kAltToleranceMeters);
}

void MissionControllerTest::_testMissionRotate()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    MissionSettingsItem* settingsItem = _missionController->visualItems()->value<MissionSettingsItem*>(0);
    QVERIFY(settingsItem);

    const QGeoCoordinate home = Coord::zurich();
    settingsItem->setCoordinate(home);

    const QGeoCoordinate wp1 = home.atDistanceAndAzimuth(180.0, 20.0);
    const QGeoCoordinate wp2 = home.atDistanceAndAzimuth(260.0, 135.0);
    _missionController->insertSimpleMissionItem(wp1, 1);
    _missionController->insertSimpleMissionItem(wp2, 2);

    SimpleMissionItem* item1 = _missionController->visualItems()->value<SimpleMissionItem*>(1);
    SimpleMissionItem* item2 = _missionController->visualItems()->value<SimpleMissionItem*>(2);
    QVERIFY(item1);
    QVERIFY(item2);

    const double oldHomeAlt = settingsItem->editableAlt();
    const double oldAlt1 = item1->editableAlt();
    const double oldAlt2 = item2->editableAlt();

    const QGeoCoordinate expectedWp1 =
        home.atDistanceAndAzimuth(home.distanceTo(wp1), home.azimuthTo(wp1) + 90.0);
    const QGeoCoordinate expectedWp2 =
        home.atDistanceAndAzimuth(home.distanceTo(wp2), home.azimuthTo(wp2) + 90.0);

    _missionController->rotateMission(90.0, true, true);
    QVERIFY_TRUE_WAIT((settingsItem->coordinate().distanceTo(home) <= kCoordToleranceMeters) &&
                          (item1->coordinate().distanceTo(expectedWp1) <= kCoordToleranceMeters) &&
                          (item2->coordinate().distanceTo(expectedWp2) <= kCoordToleranceMeters),
                      TestTimeout::shortMs());

    QCOMPARE_COORDS(settingsItem->coordinate(), home, kCoordToleranceMeters);
    QCOMPARE_COORDS(item1->coordinate(), expectedWp1, kCoordToleranceMeters);
    QCOMPARE_COORDS(item2->coordinate(), expectedWp2, kCoordToleranceMeters);
    QCOMPARE_FUZZY(settingsItem->editableAlt(), oldHomeAlt, kAltToleranceMeters);
    QCOMPARE_FUZZY(item1->editableAlt(), oldAlt1, kAltToleranceMeters);
    QCOMPARE_FUZZY(item2->editableAlt(), oldAlt2, kAltToleranceMeters);
}

void MissionControllerTest::_testMissionTransformsInvalidHome()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    MissionSettingsItem* settingsItem =
        _missionController->visualItems()->value<MissionSettingsItem*>(0);
    QVERIFY(settingsItem);

    const QGeoCoordinate home = Coord::zurich();
    settingsItem->setCoordinate(home);

    const QGeoCoordinate wp1 = home.atDistanceAndAzimuth(180.0, 45.0);
    _missionController->insertSimpleMissionItem(wp1, 1);

    SimpleMissionItem* item1 = _missionController->visualItems()->value<SimpleMissionItem*>(1);
    QVERIFY(item1);

    const QGeoCoordinate oldItemCoord = item1->coordinate();
    const double oldItemAlt = item1->editableAlt();

    settingsItem->setCoordinate(QGeoCoordinate());
    QVERIFY_TRUE_WAIT(!settingsItem->coordinate().isValid(), TestTimeout::shortMs());

    expectLogMessage("PlanManager.MissionController", QtWarningMsg, QRegularExpression("Cannot reposition mission while home is invalid"));
    _missionController->repositionMission(home.atDistanceAndAzimuth(100.0, 0.0), true, true);
    verifyExpectedLogMessage();
    expectLogMessage("PlanManager.MissionController", QtWarningMsg, QRegularExpression("Cannot rotate mission while home is invalid"));
    _missionController->rotateMission(45.0, true, true);
    verifyExpectedLogMessage();
    QCOMPARE_COORDS(item1->coordinate(), oldItemCoord, kCoordToleranceMeters);
    QCOMPARE_FUZZY(item1->editableAlt(), oldItemAlt, kAltToleranceMeters);

    _missionController->offsetMission(10.0, 5.0, 8.0, true, true);
    QVERIFY(item1->coordinate().distanceTo(oldItemCoord) > kCoordToleranceMeters);
    QCOMPARE_FUZZY(item1->editableAlt(), oldItemAlt + 8.0, kAltToleranceMeters);
}

void MissionControllerTest::_testLoadJsonSectionAvailable()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);
    _masterController->loadFromFile(":/unittest/SectionTest.plan");
    QmlObjectListModel* visualItems = _missionController->visualItems();
    QVERIFY(visualItems);
    QCOMPARE(visualItems->count(), 5);
    for (int i = 1; i < visualItems->count(); i++) {
        SimpleMissionItem* item = visualItems->value<SimpleMissionItem*>(i);
        QVERIFY(item);
        if ((int)item->command() == MAV_CMD_NAV_WAYPOINT) {
            QCOMPARE(item->cameraSection()->available(), true);
            QCOMPARE(item->speedSection()->available(), true);
        } else {
            QCOMPARE(item->cameraSection()->available(), false);
            QCOMPARE(item->speedSection()->available(), false);
        }
    }
}

void MissionControllerTest::_testGlobalAltFrame()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    struct _globalAltFrame_s
    {
        QGroundControlQmlGlobal::AltitudeFrame altFrame;
        MAV_FRAME expectedMavFrame;
    } altFrameTestCases[] = {
        {QGroundControlQmlGlobal::AltitudeFrameRelative, MAV_FRAME_GLOBAL_RELATIVE_ALT},
        {QGroundControlQmlGlobal::AltitudeFrameAbsolute, MAV_FRAME_GLOBAL},
        {QGroundControlQmlGlobal::AltitudeFrameCalcAboveTerrain, MAV_FRAME_GLOBAL},
        {QGroundControlQmlGlobal::AltitudeFrameTerrain, MAV_FRAME_GLOBAL_TERRAIN_ALT},
    };

    for (const _globalAltFrame_s& testCase : altFrameTestCases) {
        _missionController->removeAll();
        _missionController->setGlobalAltitudeFrame(testCase.altFrame);
        const QGeoCoordinate start = Coord::zurich();
        _missionController->insertTakeoffItem(start, 1);
        _missionController->insertSimpleMissionItem(start.atDistanceAndAzimuth(100, 0), 2);
        _missionController->insertSimpleMissionItem(start.atDistanceAndAzimuth(200, 0), 3);
        _missionController->insertSimpleMissionItem(start.atDistanceAndAzimuth(300, 0), 4);
        SimpleMissionItem* si =
            qobject_cast<SimpleMissionItem*>(_missionController->visualItems()->value<VisualMissionItem*>(1));
        QCOMPARE(si->altitudeFrame(), QGroundControlQmlGlobal::AltitudeFrameRelative);
        QCOMPARE(si->missionItem().frame(), MAV_FRAME_GLOBAL_RELATIVE_ALT);
        for (int i = 2; i < _missionController->visualItems()->count(); i++) {
            TEST_DEBUG(QStringLiteral("Validating altitude frame index %1").arg(i));
            SimpleMissionItem* siLoop =
                qobject_cast<SimpleMissionItem*>(_missionController->visualItems()->value<VisualMissionItem*>(i));
            QCOMPARE(siLoop->altitudeFrame(), testCase.altFrame);
            QCOMPARE(siLoop->missionItem().frame(), testCase.expectedMavFrame);
        }
    }
}

void MissionControllerTest::_testFlightPathSegmentCacheReuse()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    MissionSettingsItem* settingsItem = _missionController->visualItems()->value<MissionSettingsItem*>(0);
    QVERIFY(settingsItem);
    const QGeoCoordinate home = Coord::zurich();
    settingsItem->setCoordinate(home);

    VisualMissionItem* takeoffItem = _missionController->insertTakeoffItem(home, 1);
    VisualMissionItem* spacerItem = _missionController->insertSimpleMissionItem(home.atDistanceAndAzimuth(50, 0), 2);
    SimpleMissionItem* wp3 =
        qobject_cast<SimpleMissionItem*>(_missionController->insertSimpleMissionItem(home.atDistanceAndAzimuth(100, 0), 3));
    SimpleMissionItem* wp4 =
        qobject_cast<SimpleMissionItem*>(_missionController->insertSimpleMissionItem(home.atDistanceAndAzimuth(200, 0), 4));
    SimpleMissionItem* landWp =
        qobject_cast<SimpleMissionItem*>(_missionController->insertSimpleMissionItem(home.atDistanceAndAzimuth(300, 0), 5));
    QVERIFY(takeoffItem);
    QVERIFY(spacerItem);
    QVERIFY(wp3);
    QVERIFY(wp4);
    QVERIFY(landWp);

    QmlObjectListModel* segments = _missionController->simpleFlightPathSegments();
    QCOMPARE_TRUE_WAIT(segments->count(), 5, TestTimeout::mediumMs());
    QVERIFY(settingsItem->simpleFlightPathSegment());
    QVERIFY(spacerItem->simpleFlightPathSegment());
    QCOMPARE(settingsItem->simpleFlightPathSegment()->segmentType(), FlightPathSegment::SegmentTypeTakeoff);
    QCOMPARE(spacerItem->simpleFlightPathSegment()->segmentType(), FlightPathSegment::SegmentTypeGeneric);

    wp3->setAltitudeFrame(QGroundControlQmlGlobal::AltitudeFrameTerrain);
    wp4->setAltitudeFrame(QGroundControlQmlGlobal::AltitudeFrameTerrain);
    landWp->setCommand(MAV_CMD_NAV_LAND);

    FlightPathSegment* staleWp3Wp4 = wp3->simpleFlightPathSegment();
    FlightPathSegment* staleWp4Land = wp4->simpleFlightPathSegment();
    QVERIFY(staleWp3Wp4);
    QVERIFY(staleWp4Land);
    QCOMPARE(staleWp3Wp4->segmentType(), FlightPathSegment::SegmentTypeGeneric);
    QCOMPARE(staleWp4Land->segmentType(), FlightPathSegment::SegmentTypeGeneric);
    const quintptr preRecalcHomeTakeoffPtr = quintptr(settingsItem->simpleFlightPathSegment());
    const quintptr staleWp3Wp4Ptr = quintptr(staleWp3Wp4);
    const quintptr staleWp4LandPtr = quintptr(staleWp4Land);

    _missionController->removeVisualItem(2);
    QCOMPARE_TRUE_WAIT(segments->count(), 4, TestTimeout::mediumMs());

    QVERIFY(settingsItem->simpleFlightPathSegment());
    QVERIFY(takeoffItem->simpleFlightPathSegment());
    QVERIFY(wp3->simpleFlightPathSegment());
    QVERIFY(wp4->simpleFlightPathSegment());
    QCOMPARE(quintptr(settingsItem->simpleFlightPathSegment()), preRecalcHomeTakeoffPtr);
    QCOMPARE(settingsItem->simpleFlightPathSegment()->segmentType(), FlightPathSegment::SegmentTypeTakeoff);
    QCOMPARE(takeoffItem->simpleFlightPathSegment()->segmentType(), FlightPathSegment::SegmentTypeGeneric);
    QVERIFY(quintptr(wp3->simpleFlightPathSegment()) != staleWp3Wp4Ptr);
    QCOMPARE(wp3->simpleFlightPathSegment()->segmentType(), FlightPathSegment::SegmentTypeTerrainFrame);
    QVERIFY(quintptr(wp4->simpleFlightPathSegment()) != staleWp4LandPtr);
    QCOMPARE(wp4->simpleFlightPathSegment()->segmentType(), FlightPathSegment::SegmentTypeLand);

    const quintptr segHomeTakeoffPtr = quintptr(settingsItem->simpleFlightPathSegment());
    const quintptr segWp3Wp4Ptr = quintptr(wp3->simpleFlightPathSegment());
    const quintptr segWp4LandPtr = quintptr(wp4->simpleFlightPathSegment());

    _missionController->insertSimpleMissionItem(home.atDistanceAndAzimuth(75, 0), 2);
    QCOMPARE_TRUE_WAIT(segments->count(), 5, TestTimeout::mediumMs());

    QVERIFY(wp3->simpleFlightPathSegment());
    QVERIFY(wp4->simpleFlightPathSegment());
    QCOMPARE(quintptr(settingsItem->simpleFlightPathSegment()), segHomeTakeoffPtr);
    QCOMPARE(quintptr(wp3->simpleFlightPathSegment()), segWp3Wp4Ptr);
    QCOMPARE(quintptr(wp4->simpleFlightPathSegment()), segWp4LandPtr);
    QCOMPARE(wp3->simpleFlightPathSegment()->segmentType(), FlightPathSegment::SegmentTypeTerrainFrame);
    QCOMPARE(wp4->simpleFlightPathSegment()->segmentType(), FlightPathSegment::SegmentTypeLand);
}

void MissionControllerTest::_testSplitSegmentTracksSegmentRebuild_data()
{
    QTest::addColumn<QGroundControlQmlGlobal::AltitudeFrame>("initialFrame");
    QTest::addColumn<bool>("editPreviousItem");
    QTest::addColumn<QGroundControlQmlGlobal::AltitudeFrame>("newFrame");
    QTest::addColumn<int>("newCommand");
    QTest::addColumn<bool>("segmentRecreated");
    QTest::addColumn<FlightPathSegment::SegmentType>("recreatedSegmentType");

    const auto relative = QGroundControlQmlGlobal::AltitudeFrameRelative;
    const auto terrain = QGroundControlQmlGlobal::AltitudeFrameTerrain;
    const int keepCommand = MAV_CMD_NAV_WAYPOINT;

    QTest::newRow("previous item to terrain frame")
        << relative << true << terrain << keepCommand << true << FlightPathSegment::SegmentTypeTerrainFrame;
    QTest::newRow("previous item from terrain frame")
        << terrain << true << relative << keepCommand << true << FlightPathSegment::SegmentTypeGeneric;
    QTest::newRow("current item to land command")
        << relative << false << relative << int(MAV_CMD_NAV_LAND) << true << FlightPathSegment::SegmentTypeLand;

    QTest::newRow("current item to non-coordinate command")
        << relative << false << relative << int(MAV_CMD_DO_CHANGE_SPEED) << false
        << FlightPathSegment::SegmentTypeGeneric;
    QTest::newRow("current item to return to launch")
        << relative << false << relative << int(MAV_CMD_NAV_RETURN_TO_LAUNCH) << false
        << FlightPathSegment::SegmentTypeGeneric;
    QTest::newRow("previous item to land command")
        << relative << true << relative << int(MAV_CMD_NAV_LAND) << false << FlightPathSegment::SegmentTypeGeneric;
    QTest::newRow("previous item to non-coordinate command")
        << relative << true << relative << int(MAV_CMD_DO_CHANGE_SPEED) << false
        << FlightPathSegment::SegmentTypeGeneric;
}

void MissionControllerTest::_testSplitSegmentTracksSegmentRebuild()
{
    QFETCH(QGroundControlQmlGlobal::AltitudeFrame, initialFrame);
    QFETCH(bool, editPreviousItem);
    QFETCH(QGroundControlQmlGlobal::AltitudeFrame, newFrame);
    QFETCH(int, newCommand);
    QFETCH(bool, segmentRecreated);
    QFETCH(FlightPathSegment::SegmentType, recreatedSegmentType);

    _initForFirmwareType(MAV_AUTOPILOT_ARDUPILOTMEGA);

    MissionSettingsItem* settingsItem = _missionController->visualItems()->value<MissionSettingsItem*>(0);
    QVERIFY(settingsItem);
    const QGeoCoordinate home = Coord::zurich();
    settingsItem->setCoordinate(home);
    _missionController->setGlobalAltitudeFrame(QGroundControlQmlGlobal::AltitudeFrameMixed);

    SimpleMissionItem* wp1 = qobject_cast<SimpleMissionItem*>(
        _missionController->insertSimpleMissionItem(home.atDistanceAndAzimuth(100, 0), 1, true /* makeCurrentItem */));
    SimpleMissionItem* wp2 = qobject_cast<SimpleMissionItem*>(
        _missionController->insertSimpleMissionItem(home.atDistanceAndAzimuth(200, 0), 2, true /* makeCurrentItem */));
    QVERIFY(wp1);
    QVERIFY(wp2);
    SimpleMissionItem* editedItem = editPreviousItem ? wp1 : wp2;
    editedItem->setAltitudeFrame(initialFrame);

    const auto splitSegment = [this]() {
        return _missionController->property("splitSegment").value<FlightPathSegment*>();
    };
    QVERIFY_TRUE_WAIT(splitSegment() != nullptr, TestTimeout::mediumMs());
    QCOMPARE(splitSegment(), wp1->simpleFlightPathSegment());

    QSignalSpy staleSplitSegmentDestroyedSpy(splitSegment(), &QObject::destroyed);
    QSignalSpy splitSegmentChangedSpy(_missionController, &MissionController::splitSegmentChanged);

    editedItem->setAltitudeFrame(newFrame);
    editedItem->setCommand(newCommand);
    if (newFrame != initialFrame) {
        _missionController->insertSimpleMissionItem(home.atDistanceAndAzimuth(300, 0), 3);
    }
    QCOMPARE_TRUE_WAIT(staleSplitSegmentDestroyedSpy.count(), 1, TestTimeout::mediumMs());

    if (segmentRecreated) {
        QVERIFY(wp1->simpleFlightPathSegment());
        QCOMPARE(quintptr(splitSegment()), quintptr(wp1->simpleFlightPathSegment()));
        QCOMPARE(wp1->simpleFlightPathSegment()->segmentType(), recreatedSegmentType);
    } else {
        QCOMPARE(quintptr(splitSegment()), quintptr(0));
    }
    QCOMPARE(splitSegmentChangedSpy.count(), 1);
}

void MissionControllerTest::_testInsertComplexItemFromKML()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    constexpr int kExpectedPolygonVertexCount = 4;
    const QString polygonKml = QStringLiteral(":/unittest/PolygonAreaTest.kml");
    const QString polylineKml = QStringLiteral(":/unittest/polyline.kml");

    VisualMissionItem* surveyVisual = _missionController->insertComplexMissionItemFromKMLOrSHP(
        SurveyComplexItem::canonicalName, polygonKml, 1, false);
    QVERIFY(surveyVisual);
    SurveyComplexItem* surveyItem = qobject_cast<SurveyComplexItem*>(surveyVisual);
    QVERIFY(surveyItem);
    QCOMPARE(surveyItem->surveyAreaPolygon()->count(), kExpectedPolygonVertexCount);

    VisualMissionItem* corridorVisual = _missionController->insertComplexMissionItemFromKMLOrSHP(
        CorridorScanComplexItem::canonicalName, polylineKml, 2, false);
    QVERIFY(corridorVisual);
    CorridorScanComplexItem* corridorItem = qobject_cast<CorridorScanComplexItem*>(corridorVisual);
    QVERIFY(corridorItem);
    QVERIFY(corridorItem->surveyAreaPolygon()->count() > 0);

    VisualMissionItem* structureVisual = _missionController->insertComplexMissionItemFromKMLOrSHP(
        StructureScanComplexItem::canonicalName, polygonKml, 3, false);
    QVERIFY(structureVisual);
    StructureScanComplexItem* structureItem = qobject_cast<StructureScanComplexItem*>(structureVisual);
    QVERIFY(structureItem);
    QCOMPARE(structureItem->structurePolygon()->count(), kExpectedPolygonVertexCount);

    QCOMPARE(_missionController->visualItems()->count(), 4);
}

void MissionControllerTest::_testLoadPlanRoundTripComplexItems()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    const QGeoCoordinate center = Coord::zurich();

    VisualMissionItem* surveyVisual = _missionController->insertComplexMissionItem(
        SurveyComplexItem::canonicalName, center, 1, false);
    QVERIFY(surveyVisual);
    QVERIFY(qobject_cast<SurveyComplexItem*>(surveyVisual));

    VisualMissionItem* corridorVisual = _missionController->insertComplexMissionItem(
        CorridorScanComplexItem::canonicalName, center, 2, false);
    QVERIFY(corridorVisual);
    QVERIFY(qobject_cast<CorridorScanComplexItem*>(corridorVisual));

    QCOMPARE(_missionController->visualItems()->count(), 3);

    QTemporaryDir tmpDir;
    QVERIFY(tmpDir.isValid());
    const QString planPath = QStringLiteral("%1/test.%2").arg(tmpDir.path(), _masterController->fileExtension());
    QVERIFY(_masterController->saveToFile(planPath));

    _missionController->removeAll();
    QCOMPARE(_missionController->visualItems()->count(), 1);

    _masterController->loadFromFile(planPath);
    QCOMPARE(_missionController->visualItems()->count(), 3);

    QVERIFY(qobject_cast<SurveyComplexItem*>(_missionController->visualItems()->value<VisualMissionItem*>(1)));
    QVERIFY(qobject_cast<CorridorScanComplexItem*>(_missionController->visualItems()->value<VisualMissionItem*>(2)));
}

void MissionControllerTest::_testInsertSurveyAppliesAltFrameInMixedMode()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);
    _missionController->setGlobalAltitudeFrame(QGroundControlQmlGlobal::AltitudeFrameMixed);

    const QGeoCoordinate coord = Coord::zurich();

    VisualMissionItem* simpleVisual = _missionController->insertSimpleMissionItem(coord, 1, false);
    QVERIFY(simpleVisual);
    SimpleMissionItem* simpleItem = qobject_cast<SimpleMissionItem*>(simpleVisual);
    QVERIFY(simpleItem);
    simpleItem->setAltitudeFrame(QGroundControlQmlGlobal::AltitudeFrameAbsolute);
    simpleItem->altitude()->setRawValue(150.0);

    QCOMPARE(_missionController->visualItems()->count(), 2);
    VisualMissionItem* surveyVisual = _missionController->insertComplexMissionItem(
        SurveyComplexItem::canonicalName, coord, 2, false);
    QVERIFY(surveyVisual);
    SurveyComplexItem* surveyItem = qobject_cast<SurveyComplexItem*>(surveyVisual);
    QVERIFY(surveyItem);

    QCOMPARE(surveyItem->cameraCalc()->distanceMode(), QGroundControlQmlGlobal::AltitudeFrameAbsolute);
}

void MissionControllerTest::_testInsertNonSurveyComplexItemMixedModeNoCrash()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);
    _missionController->setGlobalAltitudeFrame(QGroundControlQmlGlobal::AltitudeFrameMixed);

    const QGeoCoordinate coord = Coord::zurich();

    VisualMissionItem* simpleVisual = _missionController->insertSimpleMissionItem(coord, 1, false);
    QVERIFY(simpleVisual);
    SimpleMissionItem* simpleItem = qobject_cast<SimpleMissionItem*>(simpleVisual);
    QVERIFY(simpleItem);
    simpleItem->setAltitudeFrame(QGroundControlQmlGlobal::AltitudeFrameAbsolute);

    VisualMissionItem* corridorVisual = _missionController->insertComplexMissionItem(
        CorridorScanComplexItem::canonicalName, coord, 2, false);
    QVERIFY(corridorVisual);
    CorridorScanComplexItem* corridorItem = qobject_cast<CorridorScanComplexItem*>(corridorVisual);
    QVERIFY(corridorItem);

    QCOMPARE(corridorItem->cameraCalc()->distanceMode(), QGroundControlQmlGlobal::AltitudeFrameRelative);
    QCOMPARE(_missionController->visualItems()->count(), 3);
}

#include "UnitTest.h"

UT_REGISTER_TEST(MissionControllerTest, TestLabel::Integration, TestLabel::MissionManager)

void MissionControllerTest::_testEmptyVehicleAPM(void)
{
    _testEmptyVehicleWorker(MAV_AUTOPILOT_ARDUPILOTMEGA);
}

void MissionControllerTest::_testEmptyVehiclePX4(void)
{
    _testEmptyVehicleWorker(MAV_AUTOPILOT_PX4);
}

void MissionControllerTest::_testInsertValidityHomePositionGating()
{
    _initForFirmwareType(MAV_AUTOPILOT_PX4);

    auto boolProperty = [this](const char* name) {
        const QVariant value = _missionController->property(name);
        return value.isValid() && value.toBool();
    };

    QVERIFY(_missionController->property("isInsertTakeoffValid").isValid());
    QVERIFY(_missionController->property("isInsertLandValid").isValid());
    QVERIFY(_missionController->property("isInsertROIValid").isValid());
    QVERIFY(_missionController->property("flyThroughCommandsAllowed").isValid());

    QCOMPARE(_missionController->homePositionSet(), false);
    QCOMPARE(boolProperty("isInsertTakeoffValid"), false);
    QCOMPARE(boolProperty("isInsertLandValid"), false);
    QCOMPARE(boolProperty("isInsertROIValid"), false);
    QCOMPARE(boolProperty("flyThroughCommandsAllowed"), false);

    _missionController->setHomePosition(Coord::zurich());
    QCOMPARE(_missionController->homePositionSet(), true);
    QCOMPARE(boolProperty("isInsertTakeoffValid"), true);
    QCOMPARE(boolProperty("isInsertLandValid"), false);
    QCOMPARE(boolProperty("isInsertROIValid"), false);
    QCOMPARE(boolProperty("flyThroughCommandsAllowed"), false);

    QVERIFY(_missionController->insertTakeoffItem(Coord::zurich(), 1, true /* makeCurrentItem */));
    QCOMPARE(boolProperty("isInsertTakeoffValid"), false);
    QCOMPARE(boolProperty("isInsertLandValid"), true);
    QCOMPARE(boolProperty("isInsertROIValid"), true);
    QCOMPARE(boolProperty("flyThroughCommandsAllowed"), true);
}

void MissionControllerTest::_testEmptyVehicleWorker(MAV_AUTOPILOT firmwareType)
{
    _initForFirmwareType(firmwareType);

    QmlObjectListModel* visualItems = _missionController->visualItems();
    QVERIFY(visualItems);
    VisualMissionItem* visualItem = visualItems->value<VisualMissionItem*>(0);
    QVERIFY(visualItem);

    _setupVisualItemSignals(visualItem);
}
