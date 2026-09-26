#include "APMSensorsCalibrationUITest.h"

#include <QtQuick/QQuickItem>
#include <QtTest/QTest>

#include "MockLink.h"
#include "ParameterManager.h"
#include "Vehicle.h"

UT_REGISTER_TEST(APMSensorsCalibrationUITest, TestLabel::Integration)

void APMSensorsCalibrationUITest::init()
{
    if (!apmFirmwareSupported()) {
        QSKIP("ArduPilot support not registered in this build");
    }
    VehicleConfigUITestBase::init();
}

void APMSensorsCalibrationUITest::_testCompassCalibration()
{
    runWithMockLink(
        [] { return MockLink::startAPMArduCopterMockLink(); },
        [&](QPointer<MockLink> /*mockLink*/, Vehicle *vehicle) {

    resetAPMParamsToUncalibrated(vehicle);
    if (QTest::currentTestFailed()) return;

    navigateToAPMSensorsPage();
    if (QTest::currentTestFailed()) return;

    verifyAPMCalIndicators(/*compassGreen=*/false, /*accelGreen=*/false, "after param reset");
    if (QTest::currentTestFailed()) return;

    runAPMFullAccelCal();
    if (QTest::currentTestFailed()) return;

    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    verifyAPMCalIndicators(/*compassGreen=*/false, /*accelGreen=*/true, "after accel cal");
    if (QTest::currentTestFailed()) return;

    runAPMCompassCal();
    if (QTest::currentTestFailed()) return;

    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    verifyAPMCalIndicators(/*compassGreen=*/true, /*accelGreen=*/true, "after compass cal");

    });
}

void APMSensorsCalibrationUITest::_testCompassCalibrationCancel()
{
    runWithMockLink(
        [] { return MockLink::startAPMArduCopterMockLink(); },
        [&](QPointer<MockLink> /*mockLink*/, Vehicle *vehicle) {

    resetAPMParamsToUncalibrated(vehicle);
    if (QTest::currentTestFailed()) return;

    navigateToAPMSensorsPage();
    if (QTest::currentTestFailed()) return;

    runAPMFullAccelCal();
    if (QTest::currentTestFailed()) return;
    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    QVERIFY2(clickButton(QStringLiteral("sensorsSetup_calibrateCompass")),
             "Failed to click Compass button");
    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("popupDialog_acceptButton"), 5000),
             "Pre-compass-cal dialog not found");
    QVERIFY2(acceptDialog(),
             "Failed to accept pre-compass-cal dialog");

    QQuickItem *progressBar = findVisibleItem(_rootItem, QStringLiteral("sensorsSetup_progressBar"), 2000);
    QVERIFY2(progressBar, "Progress bar not found");
    QVERIFY2(QTest::qWaitFor([&] { return progressBar->property("value").toDouble() > 0.0; }, 5000),
             "Compass cal progress never started");

    QQuickItem *cancelBtn = findItem(_rootItem, QStringLiteral("sensorsSetup_cancelButton"));
    QVERIFY2(cancelBtn, "Cancel button not found");
    QVERIFY2(clickButton(QStringLiteral("sensorsSetup_cancelButton")),
             "Failed to click Cancel button");
    QVERIFY2(QTest::qWaitFor([&] { return !cancelBtn->property("enabled").toBool(); }, 5000),
             "Cancel button still enabled after cancellation");

    ParameterManager *mgr = vehicle->parameterManager();
    QVERIFY2(mgr->parameterExists(ParameterManager::defaultComponentId, QStringLiteral("COMPASS_OFS_X")),
             "COMPASS_OFS_X parameter not found");
    Fact *compassOfs = mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("COMPASS_OFS_X"));
    QVERIFY2(qFuzzyIsNull(compassOfs->rawValue().toFloat()),
             "COMPASS_OFS_X is non-zero after compass cal cancel");

    verifyAPMCalIndicators(/*compassGreen=*/false, /*accelGreen=*/true, "after compass cal cancel");

    });
}

void APMSensorsCalibrationUITest::_testCompassCalibrationIgnoresStaleFailedReports()
{
    runWithMockLink(
        [] { return MockLink::startAPMArduCopterMockLink(); },
        [&](QPointer<MockLink> mockLink, Vehicle *vehicle) {

    resetAPMParamsToUncalibrated(vehicle);
    if (QTest::currentTestFailed()) return;

    navigateToAPMSensorsPage();
    if (QTest::currentTestFailed()) return;

    runAPMFullAccelCal();
    if (QTest::currentTestFailed()) return;
    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    mockLink->startAPMStaleFailedMagCalReportStreaming();

    QVERIFY2(clickButton(QStringLiteral("sensorsSetup_calibrateCompass")),
             "Failed to click Compass button");
    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("popupDialog_acceptButton"), 5000),
             "Pre-compass-cal dialog not found");
    QVERIFY2(acceptDialog(),
             "Failed to accept pre-compass-cal dialog");

    QQuickItem *cancelBtn = findItem(_rootItem, QStringLiteral("sensorsSetup_cancelButton"));
    QVERIFY2(cancelBtn, "Cancel button not found");
    QVERIFY2(QTest::qWaitFor([&] { return cancelBtn->property("enabled").toBool(); }, 5000),
             "Compass cal never became active");

    QVERIFY2(QTest::qWaitFor([&] { return !mockLink->apmStaleFailedMagCalReportStreamingActive(); }, 5000),
             "Stale failed MAG_CAL_REPORT stream was never cancelled");

    QVERIFY2(!QTest::qWaitFor([&] { return !cancelBtn->property("enabled").toBool(); }, 2000),
             "Compass cal terminated prematurely - stale MAG_CAL_REPORT was processed");

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("postOnboardCompassCalibrationDialog"), 25000),
             "Post-compass-cal dialog not shown");
    QQuickItem *const progressBar = findItem(_rootItem, QStringLiteral("sensorsSetup_progressBar"));
    QVERIFY2(progressBar && waitForCondition([progressBar] { return !progressBar->isVisible(); }, TestTimeout::shortMs(),
                                             QStringLiteral("calibration sheet closed")),
             "Calibration sheet still covers the post-compass-cal dialog");
    QVERIFY2(acceptDialog(),
             "Failed to dismiss post-compass-cal dialog");

    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    verifyAPMCalIndicators(/*compassGreen=*/true, /*accelGreen=*/true, "after compass cal with stale reports");

    });
}

void APMSensorsCalibrationUITest::_testCompassCalibrationStartRejected()
{
    runWithMockLink(
        [] { return MockLink::startAPMArduCopterMockLink(); },
        [&](QPointer<MockLink> mockLink, Vehicle *vehicle) {

    resetAPMParamsToUncalibrated(vehicle);
    if (QTest::currentTestFailed()) return;

    navigateToAPMSensorsPage();
    if (QTest::currentTestFailed()) return;

    runAPMFullAccelCal();
    if (QTest::currentTestFailed()) return;
    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    mockLink->setAPMMagCalStartFailureMode(true);

    expectAppMessage(QRegularExpression(QStringLiteral("command failed")));
    expectAppMessage(QRegularExpression(QStringLiteral("Calibration failed")));

    QVERIFY2(clickButton(QStringLiteral("sensorsSetup_calibrateCompass")),
             "Failed to click Compass button");
    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("popupDialog_acceptButton"), 5000),
             "Pre-compass-cal dialog not found");
    QVERIFY2(acceptDialog(),
             "Failed to accept pre-compass-cal dialog");

    QQuickItem *cancelBtn = findItem(_rootItem, QStringLiteral("sensorsSetup_cancelButton"));
    QVERIFY2(cancelBtn, "Cancel button not found");
    QVERIFY2(QTest::qWaitFor([&] { return !cancelBtn->property("enabled").toBool(); }, 10000),
             "Compass cal still active after START was rejected");

    verifyExpectedLogMessage();
    verifyExpectedLogMessage();

    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    verifyAPMCalIndicators(/*compassGreen=*/false, /*accelGreen=*/true, "after rejected compass cal start");

    });
}

void APMSensorsCalibrationUITest::_testAccelCalibration()
{
    runWithMockLink(
        [] { return MockLink::startAPMArduCopterMockLink(); },
        [&](QPointer<MockLink> /*mockLink*/, Vehicle *vehicle) {

    resetAPMParamsToUncalibrated(vehicle);
    if (QTest::currentTestFailed()) return;

    navigateToAPMSensorsPage();
    if (QTest::currentTestFailed()) return;

    verifyAPMCalIndicators(/*compassGreen=*/false, /*accelGreen=*/false, "after param reset");
    if (QTest::currentTestFailed()) return;

    runAPMFullAccelCal();
    if (QTest::currentTestFailed()) return;

    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    ParameterManager *mgr = vehicle->parameterManager();
    QVERIFY2(mgr->parameterExists(ParameterManager::defaultComponentId, QStringLiteral("INS_ACCOFFS_X")),
             "INS_ACCOFFS_X parameter not found");
    Fact *accelOfs = mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("INS_ACCOFFS_X"));
    QVERIFY2(QTest::qWaitFor([&] { return !qFuzzyIsNull(accelOfs->rawValue().toFloat()); }, 10000),
             "INS_ACCOFFS_X still 0 after accel cal — param refresh did not bring back non-zero value");

    verifyAPMCalIndicators(/*compassGreen=*/false, /*accelGreen=*/true, "after accel cal");

    });
}
