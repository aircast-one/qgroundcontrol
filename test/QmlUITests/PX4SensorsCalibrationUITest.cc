#include "PX4SensorsCalibrationUITest.h"

#include <QtCore/QElapsedTimer>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtTest/QTest>

#include "MockLink.h"
#include "ParameterManager.h"
#include "SensorsComponentController.h"
#include "Vehicle.h"

UT_REGISTER_TEST(PX4SensorsCalibrationUITest, TestLabel::Integration)

namespace {

struct PoseInfo {
    MockLinkPX4Calibration::Pose pose;
    const char *objectName;     ///< VehicleRotationCal objectName in SensorsSetup.qml
    const char *rotateImage;    ///< Image shown while rotating on this side (mag cal)
    const char *stillImage;     ///< Image shown while holding still on this side (accel cal)
};

constexpr PoseInfo kPoses[MockLinkPX4Calibration::kSideCount] = {
    { MockLinkPX4Calibration::Pose::RightSideUp, "sensorsCal_downSide",       "VehicleDownRotate.png",       "VehicleDown.png" },
    { MockLinkPX4Calibration::Pose::UpsideDown,  "sensorsCal_upsideDownSide", "VehicleUpsideDownRotate.png", "VehicleUpsideDown.png" },
    { MockLinkPX4Calibration::Pose::NoseDown,    "sensorsCal_noseDownSide",   "VehicleNoseDownRotate.png",   "VehicleNoseDown.png" },
    { MockLinkPX4Calibration::Pose::TailDown,    "sensorsCal_tailDownSide",   "VehicleTailDownRotate.png",   "VehicleTailDown.png" },
    { MockLinkPX4Calibration::Pose::Left,        "sensorsCal_leftSide",       "VehicleLeftRotate.png",       "VehicleLeft.png" },
    { MockLinkPX4Calibration::Pose::Right,       "sensorsCal_rightSide",      "VehicleRightRotate.png",      "VehicleRight.png" },
};

bool waitForCalState(QQuickItem *item, SensorsComponentController::SideCalState expected, int timeoutMs)
{
    return QTest::qWaitFor([&] { return item->property("calState").toInt() == static_cast<int>(expected); }, timeoutMs);
}

const char *calStateName(int state)
{
    switch (static_cast<SensorsComponentController::SideCalState>(state)) {
    case SensorsComponentController::SideCalStateIdle:       return "Idle";
    case SensorsComponentController::SideCalStateIncomplete: return "Incomplete";
    case SensorsComponentController::SideCalStateInProgress: return "InProgress";
    case SensorsComponentController::SideCalStateCompleted:  return "Completed";
    }
    return "Unknown";
}

}

void PX4SensorsCalibrationUITest::_navigateToSensorsPanel()
{
    navigateToConfigureView();
    if (QTest::currentTestFailed()) return;

    clickSidebarButton(QStringLiteral("setupComponentSensors"));
    if (QTest::currentTestFailed()) return;

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_calibrateCompass"), 3000),
             "Compass calibration row not found after opening Sensors page");
}

void PX4SensorsCalibrationUITest::_verifyAllPosesState(int expectedState, const char *context)
{
    for (const PoseInfo &info : kPoses) {
        QQuickItem *side = findItem(_rootItem, QLatin1String(info.objectName));
        QVERIFY2(side, qPrintable(QStringLiteral("Pose indicator not found (%1): %2")
                                      .arg(QLatin1String(context), QLatin1String(info.objectName))));
        const int state = side->property("calState").toInt();
        QVERIFY2(state == expectedState,
                 qPrintable(QStringLiteral("Pose in state %1, expected %2 (%3): %4")
                                .arg(QLatin1String(calStateName(state)), QLatin1String(calStateName(expectedState)),
                                     QLatin1String(context), QLatin1String(info.objectName))));
    }
}

void PX4SensorsCalibrationUITest::_verifySensorsSetupStates(const SensorsSetupStates &expected, const char *context)
{
    struct Check {
        const char *objectName;
        const char *attentionProperty;
        bool expectedComplete;
    };
    const Check checks[] = {
        { "setupComponentSensors",         "badgeVisible",     expected.sensorsComplete },
        { "sensorsSetup_calibrateCompass", "needsCalibration", expected.compassComplete },
        { "calRowGyroscope",               "needsCalibration", expected.gyroscopeComplete },
        { "sensorsSetup_calibrateAccel",   "needsCalibration", expected.accelerometerComplete },
    };

    for (const Check &check : checks) {
        QQuickItem *item = findVisibleItem(_rootItem, QLatin1String(check.objectName), 3000);
        QVERIFY2(item, qPrintable(QStringLiteral("Item not found (%1): %2")
                                      .arg(QLatin1String(context), QLatin1String(check.objectName))));
        QVERIFY2(item->property(check.attentionProperty).isValid(),
                 qPrintable(QStringLiteral("Item has no %1 property (%2): %3")
                                .arg(QLatin1String(check.attentionProperty), QLatin1String(context),
                                     QLatin1String(check.objectName))));
        const bool settled = waitForCondition(
            [&] { return item->property(check.attentionProperty).toBool() != check.expectedComplete; },
            TestTimeout::mediumMs(), QStringLiteral("%1 setup state").arg(QLatin1String(check.objectName)));
        QVERIFY2(settled, qPrintable(QStringLiteral("Setup complete state is not %1 (%2): %3")
                                         .arg(check.expectedComplete)
                                         .arg(QLatin1String(context), QLatin1String(check.objectName))));
    }
}

void PX4SensorsCalibrationUITest::_startCalibration(const QString &calibrateButtonObjectName)
{
    QVERIFY2(clickButton(calibrateButtonObjectName),
             qPrintable(QStringLiteral("Failed to click calibrate button: %1").arg(calibrateButtonObjectName)));

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("popupDialog_acceptButton"), 3000),
             "Pre-calibration dialog Ok button not found");
    QVERIFY2(acceptDialog(), "Failed to accept pre-calibration dialog");
}

void PX4SensorsCalibrationUITest::_testMagCalibration()
{
    runWithMockLink(
        [] { return MockLink::startPX4MockLink(); },
        [&](QPointer<MockLink> mockLink, Vehicle *vehicle) {
    resetParamsToFirmwareDefaults(vehicle, QStringLiteral("CAL_MAG0_ID"));
    if (QTest::currentTestFailed()) return;

    _navigateToSensorsPanel();
    if (QTest::currentTestFailed()) return;

    _verifySensorsSetupStates({
        .sensorsComplete = false,
        .compassComplete = false,
        .gyroscopeComplete = false,
        .accelerometerComplete = false,
    }, "after param reset");
    if (QTest::currentTestFailed()) return;

    _startCalibration(QStringLiteral("sensorsSetup_calibrateCompass"));
    if (QTest::currentTestFailed()) return;

    for (const PoseInfo &info : kPoses) {
        QQuickItem *side = findVisibleItem(_rootItem, QLatin1String(info.objectName), 5000);
        QVERIFY2(side, qPrintable(QStringLiteral("Pose indicator not visible: %1").arg(QLatin1String(info.objectName))));
        QVERIFY2(waitForCalState(side, SensorsComponentController::SideCalStateIncomplete, 5000),
                 qPrintable(QStringLiteral("Pose not Incomplete at start (state %1): %2")
                                .arg(QLatin1String(calStateName(side->property("calState").toInt())),
                                     QLatin1String(info.objectName))));
    }

    QQuickItem *progressBar = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_progressBar"), 2000);
    QVERIFY2(progressBar, "Progress bar not visible during calibration");
    QVERIFY2(qFuzzyIsNull(progressBar->property("value").toDouble()), "Progress bar not at 0 at calibration start");

    QQuickItem *cancelButton = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_cancelCalibration"), 2000);
    QVERIFY2(cancelButton, "Cancel button not visible during calibration");

    double lastProgress = 0.0;
    int sidesDone = 0;

    for (const PoseInfo &info : kPoses) {
        const QString sideName = QLatin1String(info.objectName);
        QQuickItem *side = findVisibleItem(_rootItem, sideName);
        QVERIFY2(side, qPrintable(QStringLiteral("Pose indicator disappeared: %1").arg(sideName)));

        mockLink->setCalibrationPose(info.pose);

        QVERIFY2(waitForCalState(side, SensorsComponentController::SideCalStateInProgress, 5000),
                 qPrintable(QStringLiteral("Side never went in-progress: %1").arg(sideName)));
        QCOMPARE(side->property("calInProgressText").toString(), QStringLiteral("Rotate"));
        QVERIFY2(side->property("imageSource").toString().endsWith(QLatin1String(info.rotateImage)),
                 qPrintable(QStringLiteral("Wrong rotate image for %1: %2")
                                .arg(sideName, side->property("imageSource").toString())));

        QVERIFY2(waitForCalState(side, SensorsComponentController::SideCalStateCompleted, 5000),
                 qPrintable(QStringLiteral("Side never completed: %1").arg(sideName)));

        sidesDone++;

        const double progress = progressBar->property("value").toDouble();
        QVERIFY2(progress > lastProgress,
                 qPrintable(QStringLiteral("Progress did not advance after %1: %2 -> %3")
                                .arg(sideName).arg(lastProgress).arg(progress)));
        if (sidesDone < MockLinkPX4Calibration::kSideCount) {
            constexpr double sideCount = MockLinkPX4Calibration::kSideCount;
            QVERIFY2((progress >= ((sidesDone - 1) / sideCount)) && (progress <= ((sidesDone / sideCount) + 0.01)),
                     qPrintable(QStringLiteral("Progress out of expected band after %1 sides: %2")
                                    .arg(sidesDone).arg(progress)));
        }
        lastProgress = progress;

        for (const PoseInfo &doneInfo : kPoses) {
            if (&doneInfo == &info) break;
            QQuickItem *doneSide = findVisibleItem(_rootItem, QLatin1String(doneInfo.objectName));
            QVERIFY2(doneSide && (doneSide->property("calState").toInt() == SensorsComponentController::SideCalStateCompleted),
                     qPrintable(QStringLiteral("Previously completed side no longer marked complete: %1")
                                    .arg(QLatin1String(doneInfo.objectName))));
        }
    }

    QVERIFY2(QTest::qWaitFor([&] { return qFuzzyCompare(progressBar->property("value").toDouble(), 1.0); }, 5000),
             qPrintable(QStringLiteral("Progress bar never reached 1.0: %1")
                            .arg(progressBar->property("value").toDouble())));

    QVERIFY2(QTest::qWaitFor([&] { return !cancelButton->isVisible(); }, 5000),
             "Calibration sheet still open after calibration completed");
    QVERIFY2(waitForDialog(QStringLiteral("Compass Calibration Complete")),
             "Compass Calibration Complete dialog not shown");
    QVERIFY2(acceptDialog(), "Failed to dismiss completion dialog");

    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    _verifyAllPosesState(SensorsComponentController::SideCalStateCompleted, "after calibration finished");
    if (QTest::currentTestFailed()) return;

    _verifySensorsSetupStates({
        .sensorsComplete = false,
        .compassComplete = true,
        .gyroscopeComplete = false,
        .accelerometerComplete = false,
    }, "after mag calibration");

    });
}

void PX4SensorsCalibrationUITest::_runCalibrationCancelTest(const QString &calibrateButtonObjectName)
{
    runWithMockLink(
        [] { return MockLink::startPX4MockLink(); },
        [&](QPointer<MockLink> mockLink, Vehicle *vehicle) {
    resetParamsToFirmwareDefaults(vehicle, QStringLiteral("CAL_MAG0_ID"));
    if (QTest::currentTestFailed()) return;

    _navigateToSensorsPanel();
    if (QTest::currentTestFailed()) return;

    _verifySensorsSetupStates({
        .sensorsComplete = false,
        .compassComplete = false,
        .gyroscopeComplete = false,
        .accelerometerComplete = false,
    }, "after param reset");
    if (QTest::currentTestFailed()) return;

    _startCalibration(calibrateButtonObjectName);
    if (QTest::currentTestFailed()) return;

    const PoseInfo &info = kPoses[0];
    QQuickItem *side = findVisibleItem(_rootItem, QLatin1String(info.objectName), 5000);
    QVERIFY2(side, "Pose indicator not visible after calibration start");

    mockLink->setCalibrationPose(info.pose);
    QVERIFY2(waitForCalState(side, SensorsComponentController::SideCalStateInProgress, 5000),
             "Side never went in-progress");

    QQuickItem *cancelButton = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_cancelCalibration"));
    QVERIFY2(cancelButton, "Cancel button not visible during calibration");
    QVERIFY2(clickButton(QStringLiteral("sensorsSetup_cancelCalibration")), "Failed to click Cancel");

    QVERIFY2(QTest::qWaitFor([&] { return !cancelButton->isVisible(); }, 5000),
             "Cancel button still visible after cancel");

    QVERIFY2(waitForCalState(side, SensorsComponentController::SideCalStateIdle, 5000),
             qPrintable(QStringLiteral("Side not Idle after cancel (state %1)")
                            .arg(QLatin1String(calStateName(side->property("calState").toInt())))));

    QVERIFY2(findVisibleItem(_rootItem, calibrateButtonObjectName, 3000),
             qPrintable(QStringLiteral("Calibrate button not available after cancel: %1")
                            .arg(calibrateButtonObjectName)));

    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    _verifySensorsSetupStates({
        .sensorsComplete = false,
        .compassComplete = false,
        .gyroscopeComplete = false,
        .accelerometerComplete = false,
    }, "after calibration cancelled");

    });
}

void PX4SensorsCalibrationUITest::_testMagCalibrationCancel()
{
    _runCalibrationCancelTest(QStringLiteral("sensorsSetup_calibrateCompass"));
}

void PX4SensorsCalibrationUITest::_testAccelCalibration()
{
    runWithMockLink(
        [] { return MockLink::startPX4MockLink(); },
        [&](QPointer<MockLink> mockLink, Vehicle *vehicle) {
    resetParamsToFirmwareDefaults(vehicle, QStringLiteral("CAL_MAG0_ID"));
    if (QTest::currentTestFailed()) return;

    _navigateToSensorsPanel();
    if (QTest::currentTestFailed()) return;

    _verifySensorsSetupStates({
        .sensorsComplete = false,
        .compassComplete = false,
        .gyroscopeComplete = false,
        .accelerometerComplete = false,
    }, "after param reset");
    if (QTest::currentTestFailed()) return;

    _startCalibration(QStringLiteral("sensorsSetup_calibrateAccel"));
    if (QTest::currentTestFailed()) return;

    for (const PoseInfo &info : kPoses) {
        QQuickItem *side = findVisibleItem(_rootItem, QLatin1String(info.objectName), 5000);
        QVERIFY2(side, qPrintable(QStringLiteral("Pose indicator not visible: %1").arg(QLatin1String(info.objectName))));
        QVERIFY2(waitForCalState(side, SensorsComponentController::SideCalStateIncomplete, 5000),
                 qPrintable(QStringLiteral("Pose not Incomplete at start (state %1): %2")
                                .arg(QLatin1String(calStateName(side->property("calState").toInt())),
                                     QLatin1String(info.objectName))));
    }

    QQuickItem *progressBar = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_progressBar"), 2000);
    QVERIFY2(progressBar, "Progress bar not visible during calibration");
    QVERIFY2(qFuzzyIsNull(progressBar->property("value").toDouble()), "Progress bar not at 0 at calibration start");

    QQuickItem *cancelButton = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_cancelCalibration"), 2000);
    QVERIFY2(cancelButton, "Cancel button not visible during calibration");

    int sidesDone = 0;

    for (const PoseInfo &info : kPoses) {
        const QString sideName = QLatin1String(info.objectName);
        QQuickItem *side = findVisibleItem(_rootItem, sideName);
        QVERIFY2(side, qPrintable(QStringLiteral("Pose indicator disappeared: %1").arg(sideName)));

        mockLink->setCalibrationPose(info.pose);

        QVERIFY2(waitForCalState(side, SensorsComponentController::SideCalStateInProgress, 5000),
                 qPrintable(QStringLiteral("Side never went in-progress: %1").arg(sideName)));
        QCOMPARE(side->property("calInProgressText").toString(), QStringLiteral("Hold Still"));
        QVERIFY2(side->property("imageSource").toString().endsWith(QLatin1String(info.stillImage)),
                 qPrintable(QStringLiteral("Wrong hold-still image for %1: %2")
                                .arg(sideName, side->property("imageSource").toString())));

        QVERIFY2(waitForCalState(side, SensorsComponentController::SideCalStateCompleted, 5000),
                 qPrintable(QStringLiteral("Side never completed: %1").arg(sideName)));

        sidesDone++;

        if (sidesDone < MockLinkPX4Calibration::kSideCount) {
            const double progress = progressBar->property("value").toDouble();
            const double expected = (17.0 * sidesDone) / 100.0;
            QVERIFY2(qAbs(progress - expected) < 0.005,
                     qPrintable(QStringLiteral("Unexpected progress after %1 sides: %2, expected %3")
                                    .arg(sidesDone).arg(progress).arg(expected)));
        }

        for (const PoseInfo &doneInfo : kPoses) {
            if (&doneInfo == &info) break;
            QQuickItem *doneSide = findVisibleItem(_rootItem, QLatin1String(doneInfo.objectName));
            QVERIFY2(doneSide && (doneSide->property("calState").toInt() == SensorsComponentController::SideCalStateCompleted),
                     qPrintable(QStringLiteral("Previously completed side no longer marked complete: %1")
                                    .arg(QLatin1String(doneInfo.objectName))));
        }
    }

    QVERIFY2(QTest::qWaitFor([&] { return qFuzzyCompare(progressBar->property("value").toDouble(), 1.0); }, 5000),
             qPrintable(QStringLiteral("Progress bar never reached 1.0: %1")
                            .arg(progressBar->property("value").toDouble())));

    QVERIFY2(QTest::qWaitFor([&] { return !cancelButton->isVisible(); }, 5000),
             "Cancel button still visible after calibration completed");

    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    _verifyAllPosesState(SensorsComponentController::SideCalStateCompleted, "after calibration finished");
    if (QTest::currentTestFailed()) return;

    _verifySensorsSetupStates({
        .sensorsComplete = false,
        .compassComplete = false,
        .gyroscopeComplete = false,
        .accelerometerComplete = true,
    }, "after accel calibration");

    });
}

void PX4SensorsCalibrationUITest::_testAccelCalibrationCancel()
{
    _runCalibrationCancelTest(QStringLiteral("sensorsSetup_calibrateAccel"));
}
