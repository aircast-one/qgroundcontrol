#include "PlanMasterControllerTest.h"

#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QRegularExpression>
#include <QtCore/QScopeGuard>
#include <QtCore/QTemporaryDir>
#include <QtTest/QSignalSpy>

#include "AppSettings.h"
#include "CameraSection.h"
#include "MissionManager.h"
#include "MissionSettingsItem.h"
#include "MultiSignalSpy.h"
#include "MultiVehicleManager.h"
#include "PlanMasterController.h"
#include "QmlObjectListModel.h"
#include "RouteRecorder.h"
#include "SettingsManager.h"
#include "SimpleMissionItem.h"
#include "SurveyPlanCreator.h"
#include "TakeoffMissionItem.h"
#include "Vehicle.h"

void PlanMasterControllerTest::init()
{
    UnitTest::init();
    MultiVehicleManager::instance()->init();
    _masterController = new PlanMasterController(this);
    _masterController->setFlyView(false);
    _masterController->start();
}

void PlanMasterControllerTest::cleanup()
{
    delete _masterController;
    _masterController = nullptr;
    _disconnectMockLink();
    UnitTest::cleanup();
}

void PlanMasterControllerTest::_testMissionPlannerFileLoad()
{
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QCOMPARE(_masterController->missionController()->visualItems()->count(), 6);
}

void PlanMasterControllerTest::_testTakeoffTextFileLoad()
{
    // Plain-text mission file with home position, takeoff and one waypoint (#13167)
    static const char* kTakeoffMission =
        "QGC WPL 110\r\n"
        "0\t1\t0\t16\t0\t0\t0\t0\t34.577822\t-112.469101\t584.380005\t1\r\n"
        "1\t0\t3\t22\t20.000000\t0.000000\t0.000000\t0.000000\t0.000000\t0.000000\t30.000000\t1\r\n"
        "2\t0\t3\t16\t0.000000\t0.000000\t0.000000\t0.000000\t34.469587\t-112.534801\t90.000000\t1\r\n";

    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    const QString filename = tempDir.filePath(QStringLiteral("TakeoffMission.waypoints"));
    QFile file(filename);
    // No QIODevice::Text: write the CRLF line endings verbatim on all platforms to match
    // the original repro file from the issue.
    QVERIFY(file.open(QIODevice::WriteOnly));
    QVERIFY(file.write(kTakeoffMission) != -1);
    file.close();

    SettingsManager::instance()->appSettings()->offlineEditingFirmwareClass()->setRawValue(QGCMAVLink::FirmwareClassArduPilot);

    _masterController->loadFromFile(filename);

    QmlObjectListModel* visualItems = _masterController->missionController()->visualItems();
    QCOMPARE(visualItems->count(), 3); // Mission settings, takeoff, waypoint

    // The original bug caused the takeoff item to consume the following waypoint line,
    // resulting in a single takeoff item carrying the waypoint's values.
    TakeoffMissionItem* takeoffItem = visualItems->value<TakeoffMissionItem*>(1);
    QVERIFY(takeoffItem);
    QCOMPARE(static_cast<MAV_CMD>(takeoffItem->command()), MAV_CMD_NAV_TAKEOFF);
    QCOMPARE(takeoffItem->missionItem().param7(), 30.0);

    SimpleMissionItem* waypointItem = visualItems->value<SimpleMissionItem*>(2);
    QVERIFY(waypointItem);
    QVERIFY(!waypointItem->isTakeoffItem());
    QCOMPARE(static_cast<MAV_CMD>(waypointItem->command()), MAV_CMD_NAV_WAYPOINT);
    QCOMPARE(waypointItem->missionItem().param7(), 90.0);
}

void PlanMasterControllerTest::_testKmlGroundPathLoad()
{
    const double defaultAltitude =
        SettingsManager::instance()->appSettings()->defaultMissionItemAltitude()->rawValue().toDouble();

    expectAppMessage(QRegularExpression(
        "Imported 3 waypoints\\..*no flight altitudes.*above the terrain.*Offline.*takeoff was added.*return to launch",
        QRegularExpression::DotMatchesEverythingOption));
    _masterController->loadFromFile(":/unittest/RouteGroundPath.kml");
    verifyExpectedLogMessage();

    QmlObjectListModel* visualItems = _masterController->missionController()->visualItems();
    QCOMPARE(visualItems->count(), 6);
    QVERIFY(qobject_cast<TakeoffMissionItem*>(visualItems->get(1)));
    QCOMPARE(_masterController->missionController()->globalAltitudeFrame(),
             QGroundControlQmlGlobal::AltitudeFrameCalcAboveTerrain);
    for (int i = 2; i < 5; i++) {
        SimpleMissionItem* item = visualItems->value<SimpleMissionItem*>(i);
        QVERIFY(item);
        QCOMPARE(static_cast<MAV_CMD>(item->command()), MAV_CMD_NAV_WAYPOINT);
        QCOMPARE(item->altitudeFrame(), QGroundControlQmlGlobal::AltitudeFrameCalcAboveTerrain);
        QCOMPARE(item->altitude()->rawValue().toDouble(), defaultAltitude);
    }
    QCOMPARE(visualItems->value<SimpleMissionItem*>(4)->coordinate().longitude(), 44.8010);
    QCOMPARE(static_cast<MAV_CMD>(visualItems->value<SimpleMissionItem*>(5)->command()), MAV_CMD_NAV_RETURN_TO_LAUNCH);
}

void PlanMasterControllerTest::_testKmzAbsolutePathLoad()
{
    expectAppMessage(QRegularExpression("above sea level, as in the file"));
    _masterController->loadFromFile(":/unittest/RouteAbsolutePath.kmz");
    verifyExpectedLogMessage();

    QmlObjectListModel* visualItems = _masterController->missionController()->visualItems();
    QCOMPARE(visualItems->count(), 6);
    QCOMPARE(_masterController->missionController()->globalAltitudeFrame(),
             QGroundControlQmlGlobal::AltitudeFrameAbsolute);
    SimpleMissionItem* first = visualItems->value<SimpleMissionItem*>(2);
    QCOMPARE(first->altitudeFrame(), QGroundControlQmlGlobal::AltitudeFrameAbsolute);
    QCOMPARE(first->altitude()->rawValue().toDouble(), 560.5);
    QCOMPARE(visualItems->value<SimpleMissionItem*>(4)->altitude()->rawValue().toDouble(), 590.25);
}

void PlanMasterControllerTest::_testKmlPlacemarksLoad()
{
    expectAppMessage(QRegularExpression("Imported 2 waypoints"));
    _masterController->loadFromFile(":/unittest/RoutePlacemarks.kml");
    verifyExpectedLogMessage();

    QmlObjectListModel* visualItems = _masterController->missionController()->visualItems();
    QCOMPARE(visualItems->count(), 5);
    QCOMPARE(visualItems->value<SimpleMissionItem*>(2)->coordinate().latitude(), 41.7100);
    QCOMPARE(visualItems->value<SimpleMissionItem*>(3)->coordinate().latitude(), 41.7150);
    QVERIFY(_masterController->dirtyForUpload());
}

static bool writeTowersKml(const QString& filename)
{
    QFile file(filename);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    return file.write(R"(<?xml version="1.0" encoding="UTF-8"?>
<kml xmlns="http://www.opengis.net/kml/2.2"><Document>
  <Placemark><LineString><coordinates>44.7900,41.7100,0 44.7950,41.7120,0 44.8010,41.7150,0</coordinates></LineString></Placemark>
  <Placemark><name>Tower A</name><Point><coordinates>44.7951,41.7122,0</coordinates></Point></Placemark>
  <Placemark><name>Tower B</name><Point><coordinates>44.8012,41.7151,0</coordinates></Point></Placemark>
</Document></kml>)") != -1;
}

void PlanMasterControllerTest::_testKmlTowersBecomeLookAtPoints()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    const QString filename = tempDir.filePath(QStringLiteral("towers.kml"));
    QVERIFY(writeTowersKml(filename));

    expectAppMessage(QRegularExpression("2 placemarks became look-at points"));
    _masterController->loadFromFile(filename);
    verifyExpectedLogMessage();

    const QList<MAV_CMD> expected = {MAV_CMD_NAV_WAYPOINT,        MAV_CMD_DO_SET_ROI_LOCATION, MAV_CMD_NAV_WAYPOINT,
                                     MAV_CMD_DO_SET_ROI_LOCATION, MAV_CMD_NAV_WAYPOINT,        MAV_CMD_DO_SET_ROI_NONE,
                                     MAV_CMD_NAV_RETURN_TO_LAUNCH};
    QmlObjectListModel* visualItems = _masterController->missionController()->visualItems();
    QCOMPARE(visualItems->count(), expected.count() + 2);
    for (qsizetype i = 0; i < expected.count(); i++) {
        QCOMPARE(static_cast<MAV_CMD>(visualItems->value<SimpleMissionItem*>(i + 2)->command()), expected[i]);
    }
    QCOMPARE(visualItems->value<SimpleMissionItem*>(3)->coordinate().longitude(), 44.7951);
}

void PlanMasterControllerTest::_testKmlTowersOnArduPilot()
{
    Fact* firmwareClass = SettingsManager::instance()->appSettings()->offlineEditingFirmwareClass();
    const QVariant previousFirmwareClass = firmwareClass->rawValue();
    const auto restoreFirmwareClass = qScopeGuard([&]() { firmwareClass->setRawValue(previousFirmwareClass); });
    firmwareClass->setRawValue(QGCMAVLink::FirmwareClassArduPilot);

    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    const QString filename = tempDir.filePath(QStringLiteral("towers.kml"));
    QVERIFY(writeTowersKml(filename));

    expectAppMessage(QRegularExpression("2 placemarks became look-at points"));
    _masterController->loadFromFile(filename);
    verifyExpectedLogMessage();

    const QList<MAV_CMD> expected = {MAV_CMD_NAV_WAYPOINT,        MAV_CMD_DO_SET_ROI,   MAV_CMD_NAV_WAYPOINT,
                                     MAV_CMD_DO_SET_ROI,          MAV_CMD_NAV_WAYPOINT, MAV_CMD_DO_SET_ROI,
                                     MAV_CMD_NAV_RETURN_TO_LAUNCH};
    QmlObjectListModel* visualItems = _masterController->missionController()->visualItems();
    QCOMPARE(visualItems->count(), expected.count() + 2);
    for (qsizetype i = 0; i < expected.count(); i++) {
        QCOMPARE(static_cast<MAV_CMD>(visualItems->value<SimpleMissionItem*>(i + 2)->command()), expected[i]);
    }
    QCOMPARE(visualItems->value<SimpleMissionItem*>(3)->missionItem().param1(), static_cast<double>(MAV_ROI_LOCATION));

    SimpleMissionItem* stopLooking = visualItems->value<SimpleMissionItem*>(7);
    QCOMPARE(stopLooking->missionItem().param1(), static_cast<double>(MAV_ROI_NONE));
    QCOMPARE(stopLooking->missionItem().param5(), 0.0);
    QCOMPARE(stopLooking->missionItem().param6(), 0.0);
    QCOMPARE(stopLooking->missionItem().param7(), 0.0);
    QCOMPARE(stopLooking->readyForSaveState(), VisualMissionItem::ReadyForSave);
}

void PlanMasterControllerTest::_testKmlTakeoffNeedsPlacingOnFixedWing()
{
    AppSettings* appSettings = SettingsManager::instance()->appSettings();
    const QVariant previousFirmwareClass = appSettings->offlineEditingFirmwareClass()->rawValue();
    const QVariant previousVehicleClass = appSettings->offlineEditingVehicleClass()->rawValue();
    const auto restoreOfflineVehicle = qScopeGuard([&]() {
        appSettings->offlineEditingFirmwareClass()->setRawValue(previousFirmwareClass);
        appSettings->offlineEditingVehicleClass()->setRawValue(previousVehicleClass);
    });
    appSettings->offlineEditingFirmwareClass()->setRawValue(QGCMAVLink::FirmwareClassArduPilot);
    appSettings->offlineEditingVehicleClass()->setRawValue(QGCMAVLink::VehicleClassFixedWing);

    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    const QString filename = tempDir.filePath(QStringLiteral("towers.kml"));
    QVERIFY(writeTowersKml(filename));

    expectAppMessage(QRegularExpression("Drag its T on the map"));
    _masterController->loadFromFile(filename);
    verifyExpectedLogMessage();
}

void PlanMasterControllerTest::_testRecordedRouteReplaysTheCamera()
{
    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());
    const QString filename = tempDir.filePath(QStringLiteral("recorded.kml"));
    const QGeoCoordinate start(41.7100, 44.7900, 650.0);
    const QGeoCoordinate tower(41.7110, 44.7930, 600.0);
    QVERIFY(RouteRecorder::saveKml(filename, QStringLiteral("Route"),
                                   {{start, -30.0, 10.0, QGeoCoordinate()},
                                    {start.atDistanceAndAzimuth(100, 0, 0.0), -30.2, 10.0, QGeoCoordinate()},
                                    {start.atDistanceAndAzimuth(150, 45, 10.0), -45.0, 0.0, tower},
                                    {start.atDistanceAndAzimuth(300, 90, 20.0), -60.0, 0.0, QGeoCoordinate()}}));

    expectAppMessage(QRegularExpression("camera angles and look-at points from the recording"));
    _masterController->loadFromFile(filename);
    verifyExpectedLogMessage();

    const QList<MAV_CMD> expected = {MAV_CMD_NAV_WAYPOINT, MAV_CMD_NAV_WAYPOINT,        MAV_CMD_DO_SET_ROI_LOCATION,
                                     MAV_CMD_NAV_WAYPOINT, MAV_CMD_DO_SET_ROI_NONE,     MAV_CMD_DO_MOUNT_CONTROL,
                                     MAV_CMD_NAV_WAYPOINT, MAV_CMD_NAV_RETURN_TO_LAUNCH};
    QmlObjectListModel* visualItems = _masterController->missionController()->visualItems();
    QCOMPARE(visualItems->count(), expected.count() + 2);
    for (qsizetype i = 0; i < expected.count(); i++) {
        QCOMPARE(static_cast<MAV_CMD>(visualItems->value<SimpleMissionItem*>(i + 2)->command()), expected[i]);
    }
    CameraSection* initialCamera = visualItems->value<MissionSettingsItem*>(0)->cameraSection();
    QVERIFY(initialCamera->specifyGimbal());
    QCOMPARE(initialCamera->gimbalPitch()->rawValue().toDouble(), -30.0);
    QCOMPARE(initialCamera->gimbalYaw()->rawValue().toDouble(), 10.0);
    SimpleMissionItem* roi = visualItems->value<SimpleMissionItem*>(4);
    QCOMPARE(roi->altitudeFrame(), QGroundControlQmlGlobal::AltitudeFrameAbsolute);
    QCOMPARE(roi->altitude()->rawValue().toDouble(), 600.0);
    const MissionItem& gimbalAfterLookAt = visualItems->value<SimpleMissionItem*>(7)->missionItem();
    QCOMPARE(gimbalAfterLookAt.param1(), -60.0);
    QCOMPARE(gimbalAfterLookAt.param7(), static_cast<double>(MAV_MOUNT_MODE_MAVLINK_TARGETING));
}

void PlanMasterControllerTest::_testKmlAreaOnlyPointsToSurvey()
{
    expectAppMessage(QRegularExpression("only has areas.*Pattern > Survey"));
    _masterController->loadFromFile(":/unittest/PolygonGood.kml");
    verifyExpectedLogMessage();

    QCOMPARE(_masterController->missionController()->visualItems()->count(), 1);
}

void PlanMasterControllerTest::_testActiveVehicleChanged()
{
    // The test emits missionManager->error() twice to verify signal propagation.
    // Each emission triggers a showAppMessage debug log via PlanMasterController.
    ignoreLogMessage("API.QGCApplication.AppMessage", QtDebugMsg,
                     QRegularExpression("Mission transfer failed"));
    // There was a defect where the PlanMasterController would, upon a new active vehicle,
    // overzelously disconnect all subscribers interested in the outgoing active vechicle.
    Vehicle* outgoingManagerVehicle = _masterController->managerVehicle();
    // spyMissionManager emulates a subscriber that should not be disconnected when
    // the active vehicle changes
    MultiSignalSpy spyMissionManager;
    spyMissionManager.init(outgoingManagerVehicle->missionManager());
    MultiSignalSpy spyMasterController;
    spyMasterController.init(_masterController);
    // Since MissionManager works with actual vehicles (which we don't have in the test cycle)
    // we have to be a bit creative emulating a signal emitted by a MissionManager.
    emit outgoingManagerVehicle->missionManager()->error(0, "");
    QVERIFY(spyMissionManager.onlyEmittedOnce("error"));
    spyMissionManager.clearSignal("error");
    QVERIFY(spyMissionManager.noneEmitted());

    _connectMockLink(MAV_AUTOPILOT_PX4);
    QVERIFY(spyMasterController.emittedOnce("managerVehicleChanged"));

    emit outgoingManagerVehicle->missionManager()->error(0, "");
    // This signal was affected by the defect - it wouldn't reach the subscriber. Here
    // we make sure it does.
    QVERIFY(spyMissionManager.onlyEmittedOnce("error"));
}

void PlanMasterControllerTest::_testDirtyFlagsMatrix_data()
{
    // Dirty-state transition matrix ("unchanged" means preserve prior value):
    //
    // | State \ Action | Upload OK | Clear | SaveDirty=true | Load plan | Save file OK | Clear save-dirty | Download w/ items | Download empty |
    // |----------------|-----------|-------|----------------|-----------|--------------|------------------|-------------------|----------------|
    // | dirtyForSave   | unchanged | false | true           | false     | false        | false            | false             | false          |
    // | dirtyForUpload | false     | false | true           | true      | unchanged    | unchanged        | false             | false          |

    // Data columns:
    //  - scenario: DirtyScenario enum value selecting which action path to execute
    //  - initialDirtyForSave: initial dirtyForSave state before action (DirtyStateTrue/False)
    //  - initialDirtyForUpload: initial dirtyForUpload state before action (DirtyStateTrue/False)
    //  - expectedDirtyForSave: expected final dirtyForSave state (DirtyState)
    //  - expectedDirtyForUpload: expected final dirtyForUpload state (DirtyState)

    QTest::addColumn<int>("scenario");
    QTest::addColumn<int>("initialDirtyForSave");
    QTest::addColumn<int>("initialDirtyForUpload");
    QTest::addColumn<int>("expectedDirtyForSave");
    QTest::addColumn<int>("expectedDirtyForUpload");

    struct ScenarioExpectation {
        DirtyScenario scenario;
        const char* name;
        DirtyState expectedDirtyForSave;
        DirtyState expectedDirtyForUpload;
    };

    const QList<ScenarioExpectation> scenarioExpectations = {
        { UploadPreservesSaveDirtyFalse,      "upload completion keeps save false",  DirtyStateUnchanged, DirtyStateFalse },
        { UploadPreservesSaveDirtyTrue,       "upload completion keeps save true",   DirtyStateUnchanged, DirtyStateFalse },
        { UploadFalseOnPlanClear,             "upload false on clear",               DirtyStateFalse,     DirtyStateFalse },
        { UploadTrueWhenSaveTrue,             "upload true when save true",          DirtyStateTrue,      DirtyStateTrue },
        { UploadTrueOnNewPlanLoad,            "upload true on new plan load",        DirtyStateFalse,     DirtyStateTrue },
        { SaveToFilePreservesUploadDirtyTrue, "saveToFile keeps upload true",        DirtyStateFalse,     DirtyStateUnchanged },
        { SaveToFilePreservesUploadDirtyFalse,"saveToFile keeps upload false",       DirtyStateFalse,     DirtyStateUnchanged },
        { SaveFalseOnSuccessfulLoad,          "save false on successful load",       DirtyStateFalse,     DirtyStateTrue },
        { ClearSaveDirtyPreservesUploadTrue,  "clear save dirty keeps upload true",  DirtyStateFalse,     DirtyStateUnchanged },
        { ClearSaveDirtyPreservesUploadFalse, "clear save dirty keeps upload false", DirtyStateFalse,     DirtyStateUnchanged },
        { DownloadWithItemsNotDirtyForSave,   "download with items stays clean",     DirtyStateFalse,     DirtyStateFalse },
        { DownloadEmptyNotDirtyForSave,       "download empty keeps save clean",     DirtyStateFalse,     DirtyStateFalse },
    };

    const QList<DirtyState> initialStates = {
        DirtyStateFalse,
        DirtyStateTrue,
    };

    for (const ScenarioExpectation& expectation : scenarioExpectations) {
        for (const DirtyState initialDirtyForSave : initialStates) {
            for (const DirtyState initialDirtyForUpload : initialStates) {
                DirtyState expectedDirtyForSave = expectation.expectedDirtyForSave;
                DirtyState expectedDirtyForUpload = expectation.expectedDirtyForUpload;

                if ((expectation.scenario == UploadTrueWhenSaveTrue) && (initialDirtyForSave == DirtyStateTrue)) {
                    // _setDirtyForSave(true) only drives dirtyForUpload when dirtyForSave transitions false->true.
                    // If dirtyForSave already starts true, dirtyForUpload is preserved.
                    expectedDirtyForUpload = DirtyStateUnchanged;
                }

                const QString rowName = QStringLiteral("%1 [init save=%2 upload=%3]")
                                            .arg(expectation.name)
                                            .arg(initialDirtyForSave == DirtyStateTrue ? QStringLiteral("true") : QStringLiteral("false"))
                                            .arg(initialDirtyForUpload == DirtyStateTrue ? QStringLiteral("true") : QStringLiteral("false"));
                QTest::newRow(rowName.toLatin1().constData())
                    << +expectation.scenario
                    << +initialDirtyForSave
                    << +initialDirtyForUpload
                    << +expectedDirtyForSave
                    << +expectedDirtyForUpload;
            }
        }
    }
}

void PlanMasterControllerTest::_testDirtyFlagsMatrix()
{
    QFETCH(int, scenario);
    QFETCH(int, initialDirtyForSave);
    QFETCH(int, initialDirtyForUpload);
    QFETCH(int, expectedDirtyForSave);
    QFETCH(int, expectedDirtyForUpload);

    QVERIFY(initialDirtyForSave != DirtyStateUnchanged);
    QVERIFY(initialDirtyForUpload != DirtyStateUnchanged);

    // Pre-load items for scenarios that need containsItems() == true
    if (scenario == DownloadWithItemsNotDirtyForSave) {
        _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    }

    const auto dirtyStateToBool = [](int state) -> bool {
        switch (state) {
        case DirtyStateFalse:
            return false;
        case DirtyStateTrue:
            return true;
        default:
            Q_ASSERT(false); // Invalid test data
            return false;
        }
    };

    _masterController->_setDirtyForSaveUnitTest(dirtyStateToBool(initialDirtyForSave));
    _masterController->_setDirtyForUploadUnitTest(dirtyStateToBool(initialDirtyForUpload));

    const bool initialDirtyForSaveBool = _masterController->dirtyForSave();
    const bool initialDirtyForUploadBool = _masterController->dirtyForUpload();

    QSignalSpy dirtyForSaveChangedSpy(_masterController, &PlanMasterController::dirtyForSaveChanged);
    QSignalSpy dirtyForUploadChangedSpy(_masterController, &PlanMasterController::dirtyForUploadChanged);

    switch (scenario) {
    case UploadPreservesSaveDirtyFalse: {
        _masterController->_sendSequence = PlanMasterController::SyncSequence::RallyPoints;
        _masterController->_sendRallyPointsComplete();
        break;
    }
    case UploadPreservesSaveDirtyTrue: {
        _masterController->_sendSequence = PlanMasterController::SyncSequence::RallyPoints;
        _masterController->_sendRallyPointsComplete();
        break;
    }
    case UploadFalseOnPlanClear:
        _masterController->removeAll();
        break;
    case UploadTrueWhenSaveTrue:
        _masterController->_setDirtyForSave(true);
        break;
    case UploadTrueOnNewPlanLoad:
        _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
        break;
    case SaveToFilePreservesUploadDirtyTrue: {
        const QString saveFile = QDir::temp().filePath(QStringLiteral("qgc_planmaster_test_%1.plan").arg(QDateTime::currentMSecsSinceEpoch()));
        QVERIFY(_masterController->saveToFile(saveFile));
        QFile::remove(saveFile);
        break;
    }
    case SaveToFilePreservesUploadDirtyFalse: {
        const QString saveFile = QDir::temp().filePath(QStringLiteral("qgc_planmaster_test_%1.plan").arg(QDateTime::currentMSecsSinceEpoch()));
        QVERIFY(_masterController->saveToFile(saveFile));
        QFile::remove(saveFile);
        break;
    }
    case SaveFalseOnSuccessfulLoad:
        _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
        break;
    case ClearSaveDirtyPreservesUploadTrue:
        _masterController->_setDirtyForSave(false);
        break;
    case ClearSaveDirtyPreservesUploadFalse:
        _masterController->_setDirtyForSave(false);
        break;
    case DownloadWithItemsNotDirtyForSave: {
        QVERIFY(_masterController->containsItems());
        _masterController->_loadSequence = PlanMasterController::SyncSequence::RallyPoints;
        _masterController->_loadRallyPointsComplete();
        break;
    }
    case DownloadEmptyNotDirtyForSave: {
        QVERIFY(!_masterController->containsItems());
        _masterController->_loadSequence = PlanMasterController::SyncSequence::RallyPoints;
        _masterController->_loadRallyPointsComplete();
        break;
    }
    }

    const auto resolveExpected = [](int expectedState, bool unchangedValue) -> bool {
        switch (expectedState) {
        case DirtyStateFalse:
            return false;
        case DirtyStateTrue:
            return true;
        case DirtyStateUnchanged:
            return unchangedValue;
        }
        return unchangedValue;
    };

    const bool expectedDirtyForSaveBool = resolveExpected(expectedDirtyForSave, initialDirtyForSaveBool);
    const bool expectedDirtyForUploadBool = resolveExpected(expectedDirtyForUpload, initialDirtyForUploadBool);
    const int expectedDirtyForSaveSignalCount = (expectedDirtyForSaveBool != initialDirtyForSaveBool) ? 1 : 0;
    const int expectedDirtyForUploadSignalCount = (expectedDirtyForUploadBool != initialDirtyForUploadBool) ? 1 : 0;

    QCOMPARE(_masterController->dirtyForSave(), expectedDirtyForSaveBool);
    QCOMPARE(_masterController->dirtyForUpload(), expectedDirtyForUploadBool);

    QCOMPARE(dirtyForSaveChangedSpy.count(), expectedDirtyForSaveSignalCount);
    QCOMPARE(dirtyForUploadChangedSpy.count(), expectedDirtyForUploadSignalCount);

    if (dirtyForSaveChangedSpy.count() > 0) {
        const QList<QVariant>& args = dirtyForSaveChangedSpy.at(dirtyForSaveChangedSpy.count() - 1);
        QCOMPARE(args.count(), 1);
        QCOMPARE(args.first().toBool(), _masterController->dirtyForSave());
    }

    if (dirtyForUploadChangedSpy.count() > 0) {
        const QList<QVariant>& args = dirtyForUploadChangedSpy.at(dirtyForUploadChangedSpy.count() - 1);
        QCOMPARE(args.count(), 1);
        QCOMPARE(args.first().toBool(), _masterController->dirtyForUpload());
    }
}

void PlanMasterControllerTest::_testFileAssociationSetOnLoad()
{
    QSignalSpy currentFileSpy(_masterController, &PlanMasterController::currentPlanFileChanged);

    // Before load, file association should be empty
    QVERIFY(_masterController->currentPlanFile().isEmpty());
    QVERIFY(_masterController->currentPlanFileName().isEmpty());

    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");

    // After successful load, the file association should be set
    QVERIFY(!_masterController->currentPlanFile().isEmpty());
    QCOMPARE(_masterController->currentPlanFileName(), QStringLiteral("MissionPlanner"));
    QVERIFY(currentFileSpy.count() >= 1);
}

void PlanMasterControllerTest::_testFailedLoadClearsFileAssociation()
{
    struct MalformedFileCase {
        const char* fileName;
        const char* contents;
    };
    const QList<MalformedFileCase> malformedCases = {
        { "Malformed.waypoints", "not a mission file" }, // Text parse failure
        { "Malformed.plan",      "{ not valid json" },   // JSON validation failure
    };

    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());

    for (const MalformedFileCase& malformedCase : malformedCases) {
        const QString context = QString::fromLatin1(malformedCase.fileName);

        _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
        QVERIFY2(!_masterController->currentPlanFile().isEmpty(), qPrintable(context));

        const QString malformedPath = tempDir.filePath(context);
        QFile malformedFile(malformedPath);
        QVERIFY2(malformedFile.open(QIODevice::WriteOnly), qPrintable(context));
        QVERIFY2(malformedFile.write(malformedCase.contents) != -1, qPrintable(context));
        malformedFile.close();

        // A failed load clears the file association
        expectLogMessage("API.QGCApplication.AppMessage", QtDebugMsg,
                         QRegularExpression("Error loading Plan file"));
        _masterController->loadFromFile(malformedPath);
        verifyExpectedLogMessage();

        QVERIFY2(_masterController->currentPlanFile().isEmpty(), qPrintable(context));
        QVERIFY2(_masterController->currentPlanFileName().isEmpty(), qPrintable(context));
    }
}

void PlanMasterControllerTest::_testDownloadClearsFileAssociation()
{
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(!_masterController->currentPlanFile().isEmpty());

    // Download completion replaces editor contents with the vehicle's plan, so the
    // previous file association no longer describes what is in the editor
    _masterController->_loadSequence = PlanMasterController::SyncSequence::RallyPoints;
    _masterController->_loadRallyPointsComplete();

    QVERIFY(_masterController->currentPlanFile().isEmpty());
    QVERIFY(_masterController->currentPlanFileName().isEmpty());
    QVERIFY(!_masterController->dirtyForSave());
    QVERIFY(!_masterController->dirtyForUpload());
}

void PlanMasterControllerTest::_testBackgroundSyncPreservesFileAssociation()
{
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(!_masterController->currentPlanFile().isEmpty());
    _masterController->_setDirtyForSaveUnitTest(true);

    // Manager loadComplete also fires for the automatic download at vehicle connect. When no
    // download was requested the editor contents are preserved, so the file association and
    // dirty state must survive.
    _masterController->_loadRallyPointsComplete();

    QVERIFY(!_masterController->currentPlanFile().isEmpty());
    QCOMPARE(_masterController->currentPlanFileName(), QStringLiteral("MissionPlanner"));
    QVERIFY(_masterController->dirtyForSave());
}

void PlanMasterControllerTest::_testUnrequestedSendCompletePreservesDirtyForUpload()
{
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(_masterController->dirtyForUpload());

    // Manager sendComplete can fire for sends this controller did not request (Fly view and
    // Plan view controllers share the same vehicle managers), so dirtyForUpload must survive.
    _masterController->_sendRallyPointsComplete();

    QVERIFY(_masterController->dirtyForUpload());
}

void PlanMasterControllerTest::_testStaleInitialPlanLoadRallyCompletePreservesPlan()
{
    // A rally-unsupported vehicle never emits rally loadComplete during initial connect, so the
    // sequence can still be InitialPlanLoad long after the download. A later unrelated rally
    // completion must not clear a newly opened plan.
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(!_masterController->currentPlanFile().isEmpty());

    _masterController->_loadSequence = PlanMasterController::SyncSequence::InitialPlanLoad;
    _masterController->_loadRallyPointsComplete();

    QVERIFY(!_masterController->currentPlanFile().isEmpty());
    QCOMPARE(_masterController->currentPlanFileName(), QStringLiteral("MissionPlanner"));
}

void PlanMasterControllerTest::_testShowPlanFromVehicleClearsFileAssociation()
{
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(_masterController->managerVehicle()->initialPlanRequestComplete(), 10000);

    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(!_masterController->currentPlanFile().isEmpty());

    // Showing the vehicle's plan replaces the editor contents, so the previous file
    // association no longer describes them
    _masterController->showPlanFromManagerVehicle();

    QVERIFY(_masterController->currentPlanFile().isEmpty());
    QVERIFY(!_masterController->dirtyForSave());
}

void PlanMasterControllerTest::_testFileAssociationClearedOnRemoveAll()
{
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(!_masterController->currentPlanFile().isEmpty());

    QSignalSpy currentFileSpy(_masterController, &PlanMasterController::currentPlanFileChanged);

    _masterController->removeAll();

    QVERIFY(_masterController->currentPlanFile().isEmpty());
    QVERIFY(_masterController->currentPlanFileName().isEmpty());
    QVERIFY(currentFileSpy.count() >= 1);
}

void PlanMasterControllerTest::_testFileAssociationClearedOnRemoveAllFromVehicle()
{
    _connectMockLink(MAV_AUTOPILOT_PX4);

    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(!_masterController->currentPlanFile().isEmpty());

    QSignalSpy currentFileSpy(_masterController, &PlanMasterController::currentPlanFileChanged);

    _masterController->removeAllFromVehicle();

    QVERIFY(_masterController->currentPlanFile().isEmpty());
    QVERIFY(_masterController->currentPlanFileName().isEmpty());
    QVERIFY(currentFileSpy.count() >= 1);
}

void PlanMasterControllerTest::_testSaveUpdatesFileName()
{
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QCOMPARE(_masterController->currentPlanFileName(), QStringLiteral("MissionPlanner"));

    // Save to a completely different path
    const QString saveFile = QDir::temp().filePath(
        QStringLiteral("qgc_planmaster_rename_%1.plan").arg(QDateTime::currentMSecsSinceEpoch()));
    QVERIFY(_masterController->saveToFile(saveFile));

    // Name should now reflect the new file base name
    QCOMPARE(_masterController->currentPlanFileName(), QFileInfo(saveFile).completeBaseName());

    // Clean up
    QFile::remove(saveFile);
}

void PlanMasterControllerTest::_testTemplateModeHidesTemplatesOnPlanCreatorSelection()
{
    // Initial state: empty plan → templates shown
    QVERIFY(_masterController->showCreateFromTemplate());

    QSignalSpy spyShow(_masterController, &PlanMasterController::showCreateFromTemplateChanged);

    // User selects a plan creator (e.g. Survey) — adds items to the plan
    SurveyPlanCreator creator(_masterController);
    creator.createPlan(QGeoCoordinate(47.0, -122.0));

    QVERIFY(_masterController->containsItems());
    QVERIFY(!_masterController->showCreateFromTemplate());
    QCOMPARE(spyShow.count(), 1);
}

void PlanMasterControllerTest::_testTemplateModeHidesTemplatesOnFileLoad()
{
    // Initial state: empty plan, not manual creation → templates shown
    QVERIFY(_masterController->showCreateFromTemplate());

    QSignalSpy spyShow(_masterController, &PlanMasterController::showCreateFromTemplateChanged);

    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");

    QVERIFY(_masterController->containsItems());
    QVERIFY(!_masterController->showCreateFromTemplate());
    QCOMPARE(spyShow.count(), 1);
}

void PlanMasterControllerTest::_testTemplateModeRestoredOnRemoveAll()
{
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(!_masterController->showCreateFromTemplate());

    QSignalSpy spyShow(_masterController, &PlanMasterController::showCreateFromTemplateChanged);

    _masterController->removeAll();

    QVERIFY(!_masterController->containsItems());
    QVERIFY(_masterController->showCreateFromTemplate());
    QCOMPARE(spyShow.count(), 1);
}

void PlanMasterControllerTest::_testTemplateModeRestoredOnIndividualItemRemoval()
{
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(!_masterController->showCreateFromTemplate());

    QSignalSpy spyShow(_masterController, &PlanMasterController::showCreateFromTemplateChanged);

    _masterController->missionController()->removeAll();
    _masterController->geoFenceController()->removeAll();
    _masterController->rallyPointController()->removeAll();

    QVERIFY(!_masterController->containsItems());
    QVERIFY(_masterController->showCreateFromTemplate());
    QCOMPARE(spyShow.count(), 1);
}

void PlanMasterControllerTest::_testManualCreationHidesTemplates()
{
    // Initial state: empty plan → templates shown
    QVERIFY(_masterController->showCreateFromTemplate());
    QVERIFY(!_masterController->userSelectedManualCreation());

    QSignalSpy spyShow(_masterController, &PlanMasterController::showCreateFromTemplateChanged);
    QSignalSpy spyManual(_masterController, &PlanMasterController::userSelectedManualCreationChanged);

    // User clicks "No Template" — hides templates even though plan is empty
    _masterController->setUserSelectedManualCreation(true);

    QVERIFY(_masterController->userSelectedManualCreation());
    QVERIFY(!_masterController->showCreateFromTemplate());
    QCOMPARE(spyShow.count(), 1);
    QCOMPARE(spyManual.count(), 1);

    // Setting the same value again should not re-emit
    _masterController->setUserSelectedManualCreation(true);
    QCOMPARE(spyShow.count(), 1);
    QCOMPARE(spyManual.count(), 1);
}

void PlanMasterControllerTest::_testManualCreationRestoredOnRemoveAll()
{
    _masterController->setUserSelectedManualCreation(true);
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(!_masterController->showCreateFromTemplate());

    QSignalSpy spyShow(_masterController, &PlanMasterController::showCreateFromTemplateChanged);
    QSignalSpy spyManual(_masterController, &PlanMasterController::userSelectedManualCreationChanged);

    _masterController->removeAll();

    QVERIFY(!_masterController->containsItems());
    QVERIFY(!_masterController->userSelectedManualCreation());
    QVERIFY(_masterController->showCreateFromTemplate());
    QCOMPARE(spyShow.count(), 1);
    QCOMPARE(spyManual.count(), 1);
}

void PlanMasterControllerTest::_testManualCreationRestoredOnIndividualItemRemoval()
{
    _masterController->setUserSelectedManualCreation(true);
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QVERIFY(!_masterController->showCreateFromTemplate());

    QSignalSpy spyShow(_masterController, &PlanMasterController::showCreateFromTemplateChanged);
    QSignalSpy spyManual(_masterController, &PlanMasterController::userSelectedManualCreationChanged);

    _masterController->missionController()->removeAll();
    _masterController->geoFenceController()->removeAll();
    _masterController->rallyPointController()->removeAll();

    QVERIFY(!_masterController->containsItems());
    QVERIFY(!_masterController->userSelectedManualCreation());
    QVERIFY(_masterController->showCreateFromTemplate());
    QCOMPARE(spyShow.count(), 1);
    QCOMPARE(spyManual.count(), 1);
}

void PlanMasterControllerTest::_testPlanCreatorsFiltered()
{
    // MultiRotor supports StructureScan — expect all 4 creators
    PlanMasterController multiRotorController(MAV_AUTOPILOT_PX4, MAV_TYPE_QUADROTOR);
    multiRotorController.setFlyView(false);
    multiRotorController.start();
    QVERIFY(multiRotorController.planCreators() != nullptr);
    const int multiRotorCount = multiRotorController.planCreators()->count();
    QVERIFY(multiRotorCount > 0);

    // FixedWing does not support StructureScan — expect one fewer creator
    PlanMasterController fixedWingController(MAV_AUTOPILOT_PX4, MAV_TYPE_FIXED_WING);
    fixedWingController.setFlyView(false);
    fixedWingController.start();
    QVERIFY(fixedWingController.planCreators() != nullptr);
    const int fixedWingCount = fixedWingController.planCreators()->count();
    QVERIFY(fixedWingCount > 0);

    QCOMPARE(fixedWingCount, multiRotorCount - 1);
}

#include "UnitTest.h"

UT_REGISTER_TEST(PlanMasterControllerTest, TestLabel::Integration, TestLabel::MissionManager)
