#include "AppCloseWarningUITest.h"

#include <QtCore/QPointer>
#include <QtCore/QRegularExpression>
#include <QtCore/QScopeGuard>
#include <QtCore/QTemporaryDir>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtTest/QTest>

#include "MissionController.h"
#include "MockLink.h"
#include "MultiVehicleManager.h"
#include "ParameterManager.h"
#include "PlanMasterController.h"
#include "Vehicle.h"

UT_REGISTER_TEST(AppCloseWarningUITest, TestLabel::Integration, TestLabel::MissionManager)

PlanMasterController *AppCloseWarningUITest::_planViewMasterController()
{
    QQuickItem *planView = _window ? _window->findChild<QQuickItem *>(QStringLiteral("planView")) : nullptr;
    if (!planView) {
        QTest::qFail("Could not find Plan view item (planView)", __FILE__, __LINE__);
        return nullptr;
    }

    auto *masterController = qvariant_cast<PlanMasterController *>(planView->property("_planMasterController"));
    if (!masterController) {
        QTest::qFail("Plan view _planMasterController property is not a PlanMasterController", __FILE__, __LINE__);
        return nullptr;
    }

    return masterController;
}

bool AppCloseWarningUITest::_forcePlanViewMissionDirty()
{
    PlanMasterController *masterController = _planViewMasterController();
    if (!masterController) {
        return false;
    }

    masterController->missionController()->setDirty(true);
    return true;
}

void AppCloseWarningUITest::_testCloseWarningMatrix_data()
{
    QTest::addColumn<bool>("mission");
    QTest::addColumn<bool>("pendingWrites");
    QTest::addColumn<bool>("connection");
    QTest::addColumn<int>("rejectAtStep");

    QTest::newRow("none-acceptAll")            << false << false << false << 0;

    QTest::newRow("C-acceptAll")               << false << false << true  << 0;
    QTest::newRow("C-rejectConnection")        << false << false << true  << 1;

    QTest::newRow("PC-acceptAll")              << false << true  << true  << 0;
    QTest::newRow("PC-rejectPending")          << false << true  << true  << 1;
    QTest::newRow("PC-rejectConnection")       << false << true  << true  << 2;

    QTest::newRow("M-acceptAll")               << true  << false << false << 0;
    QTest::newRow("M-rejectMission")           << true  << false << false << 1;

    QTest::newRow("MC-acceptAll")              << true  << false << true  << 0;
    QTest::newRow("MC-rejectMission")          << true  << false << true  << 1;
    QTest::newRow("MC-rejectConnection")       << true  << false << true  << 2;

    QTest::newRow("MPC-acceptAll")             << true  << true  << true  << 0;
    QTest::newRow("MPC-rejectMission")         << true  << true  << true  << 1;
    QTest::newRow("MPC-rejectPending")         << true  << true  << true  << 2;
    QTest::newRow("MPC-rejectConnection")      << true  << true  << true  << 3;
}

void AppCloseWarningUITest::_testCloseWarningMatrix()
{
    QFETCH(bool, mission);
    QFETCH(bool, pendingWrites);
    QFETCH(bool, connection);
    QFETCH(int, rejectAtStep);

    QVERIFY2(!pendingWrites || connection, "Invalid matrix row: pending writes without a connection");

    ignoreLogMessage("API.QGCApplication.AppMessage", QtDebugMsg,
                     QRegularExpression(QStringLiteral("Offline Map Cache database has been upgraded")));

    startUI();
    if (QTest::currentTestFailed()) {
        return;
    }

    QPointer<MockLink> mockLink;
    Vehicle *vehicle = nullptr;
    bool appClosed = false;

    const auto guard = qScopeGuard([&] {
        if (!appClosed) {
            disconnectMockLink(mockLink);
        }
        closeUIWindow();
        destroyUIEngine();
    });

    if (connection) {
        mockLink = connectMockLinkAndWaitReady(
            [] { return MockLink::startPX4MockLink(); },
            vehicle);
        if (!mockLink) {
            return;
        }
    }

    if (mission) {
        if (!_forcePlanViewMissionDirty()) {
            return;
        }
    }

    if (pendingWrites) {
        QVERIFY2(vehicle, "Pending writes requested but no vehicle is connected");
        vehicle->parameterManager()->setPendingWritesForTest(true);
    }

    QVERIFY2(QMetaObject::invokeMethod(_window, "close"),
             "Failed to close the main window");

    QStringList expectedDialogs;
    if (mission)       expectedDialogs << QStringLiteral("mission edit in progress");
    if (pendingWrites) expectedDialogs << QStringLiteral("pending parameter updates");
    if (connection)    expectedDialogs << QStringLiteral("still active connections");

    for (int step = 0; step < expectedDialogs.size(); ++step) {
        const QString &title = expectedDialogs.at(step);
        QVERIFY2(waitForDialog(title),
                 qPrintable(QStringLiteral("Expected close-warning dialog not shown: %1").arg(title)));

        const bool rejectHere = (rejectAtStep != 0) && (step == rejectAtStep - 1);
        if (rejectHere) {
            QVERIFY2(rejectDialog(),
                     qPrintable(QStringLiteral("Failed to reject dialog: %1").arg(title)));

            QVERIFY2(_window && _window->isVisible(),
                     qPrintable(QStringLiteral("Window closed after rejecting dialog: %1").arg(title)));
            if (step + 1 < expectedDialogs.size()) {
                QVERIFY2(!dialogVisible(expectedDialogs.at(step + 1)),
                         qPrintable(QStringLiteral("Later dialog shown after a reject: %1").arg(expectedDialogs.at(step + 1))));
            }
            return;
        }

        QVERIFY2(acceptDialog(),
                 qPrintable(QStringLiteral("Failed to accept dialog: %1").arg(title)));
    }

    QVERIFY2(QTest::qWaitFor([this] { return _window && !_window->isVisible(); }, 3000),
             "App did not close after accepting all close-warning dialogs");
    appClosed = true;
}

void AppCloseWarningUITest::_testNoUnsavedMissionWarningForDownloadedMission()
{
    ignoreLogMessage("API.QGCApplication.AppMessage", QtDebugMsg,
                     QRegularExpression(QStringLiteral("Offline Map Cache database has been upgraded")));

    runWithMockLink(
        [] { return MockLink::startPX4MockLinkWithMission(); },
        [this](QPointer<MockLink> mockLink, Vehicle *vehicle) {
            Q_UNUSED(mockLink);
            Q_UNUSED(vehicle);

            QVERIFY2(QMetaObject::invokeMethod(_window, "close"),
                     "Failed to close the main window");

            QVERIFY2(waitForDialog(QStringLiteral("still active connections")),
                     "Active vehicle connection warning dialog was not shown on close");

            QVERIFY2(!dialogVisible(QStringLiteral("mission edit in progress")),
                     "Unsaved mission warning shown for a freshly downloaded, unedited plan");
        });
}

void AppCloseWarningUITest::_testNoUnsavedMissionWarningAfterSuccessfulUpload()
{
    ignoreLogMessage("API.QGCApplication.AppMessage", QtDebugMsg,
                     QRegularExpression(QStringLiteral("Offline Map Cache database has been upgraded")));

    runWithMockLink(
        [] { return MockLink::startPX4MockLink(); },
        [this](QPointer<MockLink> mockLink, Vehicle *vehicle) {
            Q_UNUSED(mockLink);
            Q_UNUSED(vehicle);

            PlanMasterController *masterController = _planViewMasterController();
            if (!masterController) {
                return;
            }

            masterController->missionController()->setDirty(true);
            QVERIFY2(masterController->dirtyForSave() && masterController->dirtyForUpload(),
                     "Dirtying the mission controller did not propagate to the master controller");

            masterController->sendToVehicle();
            QTRY_VERIFY_WITH_TIMEOUT(!masterController->syncInProgress() && !masterController->dirtyForUpload(), TestTimeout::mediumMs());

            QVERIFY(masterController->dirtyForSave());

            QVERIFY2(QMetaObject::invokeMethod(_window, "close"),
                     "Failed to close the main window");

            QVERIFY2(waitForDialog(QStringLiteral("still active connections")),
                     "Active vehicle connection warning dialog was not shown on close");

            QVERIFY2(!dialogVisible(QStringLiteral("mission edit in progress")),
                     "Unsaved mission warning shown after a successful mission upload");
        });
}

void AppCloseWarningUITest::_testNoUnsavedMissionWarningAfterSaveToFile()
{
    ignoreLogMessage("API.QGCApplication.AppMessage", QtDebugMsg,
                     QRegularExpression(QStringLiteral("Offline Map Cache database has been upgraded")));

    QTemporaryDir tempDir;
    QVERIFY(tempDir.isValid());

    runWithMockLink(
        [] { return MockLink::startPX4MockLink(); },
        [this, &tempDir](QPointer<MockLink> mockLink, Vehicle *vehicle) {
            Q_UNUSED(mockLink);
            Q_UNUSED(vehicle);

            PlanMasterController *masterController = _planViewMasterController();
            if (!masterController) {
                return;
            }

            masterController->missionController()->setDirty(true);
            QVERIFY2(masterController->dirtyForSave() && masterController->dirtyForUpload(),
                     "Dirtying the mission controller did not propagate to the master controller");

            QVERIFY(masterController->saveToFile(tempDir.filePath(QStringLiteral("close-warning-test"))));

            QVERIFY(!masterController->dirtyForSave());
            QVERIFY(masterController->dirtyForUpload());

            QVERIFY2(QMetaObject::invokeMethod(_window, "close"),
                     "Failed to close the main window");

            QVERIFY2(waitForDialog(QStringLiteral("still active connections")),
                     "Active vehicle connection warning dialog was not shown on close");

            QVERIFY2(!dialogVisible(QStringLiteral("mission edit in progress")),
                     "Unsaved mission warning shown after saving the plan to disk");
        });
}
