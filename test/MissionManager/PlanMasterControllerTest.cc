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
#include "RallyPointController.h"
#include "MissionController.h"
#include "QmlObjectListModel.h"

#include <QtPositioning/QGeoCoordinate>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>

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

void PlanMasterControllerTest::_aPlanThatCouldNotBeWrittenIsStillUnsaved(void)
{
    QTemporaryDir scratch;
    QVERIFY(scratch.isValid());
    _masterController->setDirty(true);
    QVERIFY(_masterController->offline());
    QVERIFY(_masterController->dirty());

    const QString unreachable = QDir(scratch.path()).filePath(QStringLiteral("no-such-directory/plan.plan"));
    QVERIFY(!_masterController->saveToFile(unreachable));
    QVERIFY(!QFileInfo::exists(unreachable));
    QVERIFY2(_masterController->dirty(), "a plan the disk refused is still unsaved, and the flag every head reads to say so must stay up");

    const QString reachable = QDir(scratch.path()).filePath(QStringLiteral("written.plan"));
    QVERIFY(_masterController->saveToFile(reachable));
    QVERIFY(QFileInfo(reachable).size() > 0);
    QVERIFY2(!_masterController->dirty(), "a plan that reached the disk is saved, which is the case the flag still has to answer");
}

void PlanMasterControllerTest::_aFenceInThePlanDoesNotKeepItDirty(void)
{
    _masterController->geoFenceController()->addInclusionPolygon(QGeoCoordinate(47.0, 8.0), QGeoCoordinate(46.9, 8.1));
    QVERIFY(_masterController->dirty());

    _masterController->setDirty(false);
    QVERIFY2(!_masterController->dirty(), "clearing the flag clears it the first time; clearing the polygons underneath must not raise it again");

    _masterController->geoFenceController()->addInclusionCircle(QGeoCoordinate(47.0, 8.0), QGeoCoordinate(46.9, 8.1));
    QVERIFY(_masterController->dirty());
    _masterController->setDirty(false);
    QVERIFY(!_masterController->dirty());
}
