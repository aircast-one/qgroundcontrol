/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "PlanMasterControllerTest.h"
#include "MultiSignalSpyV2.h"
#include "MissionManager.h"
#include "PlanMasterController.h"
#include "Vehicle.h"
#include "GeoFenceController.h"
#include "QmlObjectListModel.h"

#include <QtPositioning/QGeoCoordinate>

#include <QtTest/QTest>

PlanMasterControllerTest::PlanMasterControllerTest(void)
    : _masterController(nullptr)
{

}

void PlanMasterControllerTest::init(void)
{
    UnitTest::init();

    _masterController = new PlanMasterController(this);
    _masterController->setFlyView(false);
    _masterController->start();
}

void PlanMasterControllerTest::cleanup(void)
{
    delete _masterController;
    _masterController = nullptr;
    UnitTest::cleanup();
}

void PlanMasterControllerTest::_testMissionFileLoad(void)
{
    _masterController->loadFromFile(":/unittest/OldFileFormat.mission");
    QCOMPARE(_masterController->missionController()->visualItems()->count(), 7);
}


void PlanMasterControllerTest::_testMissionPlannerFileLoad(void)
{
    _masterController->loadFromFile(":/unittest/MissionPlanner.waypoints");
    QCOMPARE(_masterController->missionController()->visualItems()->count(), 6);
}

void PlanMasterControllerTest::_testUndo(void)
{
    _masterController->_captureUndoSnapshot();
    QVERIFY(!_masterController->canUndo());

    _masterController->loadFromFile(":/unittest/OldFileFormat.mission");
    _masterController->_captureUndoSnapshot();
    QVERIFY(_masterController->canUndo());
    QVERIFY(!_masterController->canRedo());
    QCOMPARE(_masterController->missionController()->visualItems()->count(), 7);

    _masterController->undo();
    QVERIFY(!_masterController->canUndo());
    QVERIFY(_masterController->canRedo());
    QCOMPARE(_masterController->missionController()->visualItems()->count(), 1);

    _masterController->redo();
    QVERIFY(_masterController->canUndo());
    QVERIFY(!_masterController->canRedo());
    QCOMPARE(_masterController->missionController()->visualItems()->count(), 7);
}

void PlanMasterControllerTest::_testUndoFence(void)
{
    GeoFenceController* fence = _masterController->geoFenceController();
    _masterController->_captureUndoSnapshot();

    fence->addInclusionPolygon(QGeoCoordinate(47.0, 8.0), QGeoCoordinate(46.9, 8.1));
    _masterController->_captureUndoSnapshot();
    QCOMPARE(fence->polygons()->count(), 1);

    _masterController->undo();
    QCOMPARE(fence->polygons()->count(), 0);

    _masterController->redo();
    QCOMPARE(fence->polygons()->count(), 1);
}

void PlanMasterControllerTest::_testUndoCorruptSnapshot(void)
{
    _masterController->_captureUndoSnapshot();
    _masterController->loadFromFile(":/unittest/OldFileFormat.mission");
    _masterController->_captureUndoSnapshot();
    QCOMPARE(_masterController->_undoStack.count(), 1);

    _masterController->_undoStack.append("{\"fileType\":\"Plan\"");
    _masterController->undo();
    QCOMPARE(_masterController->missionController()->visualItems()->count(), 7);
    QCOMPARE(_masterController->_undoStack.count(), 1);
    QVERIFY(!_masterController->canRedo());

    _masterController->_undoStack.append("{\"fileType\":\"Plan\",\"version\":1,\"groundStation\":\"QGroundControl\",\"mission\":{},\"geoFence\":{},\"rallyPoints\":{}}");
    _masterController->undo();
    QCOMPARE(_masterController->missionController()->visualItems()->count(), 7);
    QCOMPARE(_masterController->_undoStack.count(), 1);

    _masterController->undo();
    QCOMPARE(_masterController->missionController()->visualItems()->count(), 1);
}

void PlanMasterControllerTest::_testUndoTracksDirty(void)
{
    GeoFenceController* fence = _masterController->geoFenceController();
    _masterController->setDirty(false);
    _masterController->_captureUndoSnapshot();
    QVERIFY(!_masterController->dirty());

    fence->addInclusionPolygon(QGeoCoordinate(47.0, 8.0), QGeoCoordinate(46.9, 8.1));
    _masterController->_captureUndoSnapshot();
    QVERIFY(_masterController->dirty());

    _masterController->undo();
    QVERIFY(!_masterController->dirty());

    _masterController->redo();
    QVERIFY(_masterController->dirty());
}

void PlanMasterControllerTest::_testActiveVehicleChanged(void) {
    // There was a defect where the PlanMasterController would, upon a new active vehicle,
    // overzelously disconnect all subscribers interested in the outgoing active vechicle.
    Vehicle* outgoingManagerVehicle = _masterController->managerVehicle();

    // spyMissionManager emulates a subscriber that should not be disconnected when
    // the active vehicle changes
    MultiSignalSpyV2 spyMissionManager;
    spyMissionManager.init(outgoingManagerVehicle->missionManager());
    MultiSignalSpyV2 spyMasterController;
    spyMasterController.init(_masterController);

    // Since MissionManager works with actual vehicles (which we don't have in the test cycle)
    // we have to be a bit creative emulating a signal emitted by a MissionManager.
    emit outgoingManagerVehicle->missionManager()->error(0,"");
    auto missionManagerErrorSignalMask = spyMissionManager.signalNameToMask("error");
    QVERIFY(spyMissionManager.checkOnlySignalByMask(missionManagerErrorSignalMask));
    spyMissionManager.clearSignal("error");
    QVERIFY(spyMissionManager.checkNoSignals());

    _connectMockLink(MAV_AUTOPILOT_PX4);
    auto masterControllerMgrVehicleChanged = spyMasterController.signalNameToMask("managerVehicleChanged");
    QVERIFY(spyMasterController.checkSignalByMask(masterControllerMgrVehicleChanged));

    emit outgoingManagerVehicle->missionManager()->error(0,"");
    // This signal was affected by the defect - it wouldn't reach the subscriber. Here
    // we make sure it does.
    QVERIFY(spyMissionManager.checkOnlySignalByMask(missionManagerErrorSignalMask));
}
