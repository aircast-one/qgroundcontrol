#include "APMFreshFlashUITest.h"

#include <QtCore/QPointer>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

#include "AutoPilotPlugin.h"
#include "Fact.h"
#include "MockLink.h"
#include "ParameterManager.h"
#include "Vehicle.h"
#include "VehicleComponent.h"

UT_REGISTER_TEST(APMFreshFlashUITest, TestLabel::Integration)

void APMFreshFlashUITest::init()
{
    if (!apmFirmwareSupported()) {
        QSKIP("ArduPilot support not registered in this build");
    }
    VehicleConfigUITestBase::init();
}

void APMFreshFlashUITest::_verifyFramePrereq(const QString &compObjectName, bool expectPrereqShown)
{
    QQuickItem *compBtn = clickSidebarButton(compObjectName);
    if (QTest::currentTestFailed()) return;
    QVERIFY2(compBtn, qPrintable(QStringLiteral("Component button not found: %1").arg(compObjectName)));

    QQuickItem *messagePanel = findVisibleItem(_rootItem, QStringLiteral("setupMessagePanel"), expectPrereqShown ? 3000 : 0);
    if (expectPrereqShown) {
        QVERIFY2(messagePanel,
                 qPrintable(QStringLiteral("Frame prerequisite message panel not shown for: %1").arg(compObjectName)));
    } else {
        QVERIFY2(!messagePanel,
                 qPrintable(QStringLiteral("Prerequisite message panel unexpectedly shown for: %1").arg(compObjectName)));
    }
}

void APMFreshFlashUITest::_verifySetupIndicator(const QString &compObjectName, bool expectSetupComplete)
{
    QPointer<QQuickItem> compBtn = findVisibleItem(_rootItem, compObjectName, 3000);
    QVERIFY2(compBtn, qPrintable(QStringLiteral("Component button not found: %1").arg(compObjectName)));

    QVERIFY2(waitForCondition([&] { return compBtn && (compBtn->property("badgeVisible").toBool() != expectSetupComplete); },
                              TestTimeout::mediumMs(), QStringLiteral("%1 setup badge").arg(compObjectName)),
             qPrintable(QStringLiteral("%1 setup-required badge not %2")
                            .arg(compObjectName, expectSetupComplete ? QStringLiteral("hidden") : QStringLiteral("shown"))));
}

void APMFreshFlashUITest::_testFreshFlashSetupState()
{
    expectAppMessage(QRegularExpression(QStringLiteral("Configuration tasks remain before this vehicle is ready to fly")));
    runWithMockLink(
        [] { return MockLink::startAPMArduCopterMockLink(MockConfiguration::OptionAPMStartFreshParams); },
        [&](QPointer<MockLink> /*mockLink*/, Vehicle *vehicle) {

    QVERIFY2(acceptDialog(5000), "Setup-incomplete app message dialog never shown");
    verifyExpectedLogMessage();
    if (QTest::currentTestFailed()) return;

    ParameterManager *mgr = vehicle->parameterManager();
    QCOMPARE(mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("FRAME_CLASS"))->rawValue().toInt(), 0);
    QVERIFY2(qFuzzyIsNull(mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("COMPASS_OFS_X"))->rawValue().toFloat()),
             "COMPASS_OFS_X not 0 on fresh-flash MockLink");
    QVERIFY2(qFuzzyIsNull(mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("INS_ACCOFFS_X"))->rawValue().toFloat()),
             "INS_ACCOFFS_X not 0 on fresh-flash MockLink");
    QCOMPARE(mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("RC1_MIN"))->rawValue().toInt(), 1100);
    QCOMPARE(mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("RC1_MAX"))->rawValue().toInt(), 1900);
    QCOMPARE(mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("RC1_TRIM"))->rawValue().toInt(), 1500);

    VehicleComponent *frameComp = findVehicleComponent(vehicle, QStringLiteral("Frame"));
    VehicleComponent *sensorsComp = findVehicleComponent(vehicle, QStringLiteral("Sensors"));
    VehicleComponent *radioComp = findVehicleComponent(vehicle, QStringLiteral("Radio"));
    QVERIFY2(frameComp, "Frame component not found");
    QVERIFY2(sensorsComp, "Sensors component not found");
    QVERIFY2(radioComp, "Radio component not found");

    QVERIFY2(!frameComp->setupComplete(), "Frame setupComplete despite FRAME_CLASS == 0");
    QVERIFY2(!sensorsComp->setupComplete(), "Sensors setupComplete despite zero cal offsets");
    QVERIFY2(!radioComp->setupComplete(), "Radio setupComplete despite default RC min/max/trim");

    AutoPilotPlugin *autopilot = vehicle->autopilotPlugin();
    QCOMPARE(autopilot->prerequisiteSetup(sensorsComp), frameComp->name());
    QCOMPARE(autopilot->prerequisiteSetup(radioComp), frameComp->name());
    QVERIFY2(autopilot->prerequisiteSetup(frameComp).isEmpty(), "Frame unexpectedly has a prerequisite");

    navigateToConfigureView();
    if (QTest::currentTestFailed()) return;

    _verifySetupIndicator(QStringLiteral("setupComponentFrame"), /*expectSetupComplete=*/false);
    if (QTest::currentTestFailed()) return;
    _verifySetupIndicator(QStringLiteral("setupComponentSensors"), /*expectSetupComplete=*/false);
    if (QTest::currentTestFailed()) return;
    _verifySetupIndicator(QStringLiteral("setupComponentRadio"), /*expectSetupComplete=*/false);
    if (QTest::currentTestFailed()) return;

    _verifyFramePrereq(QStringLiteral("setupComponentFrame"), /*expectPrereqShown=*/false);
    if (QTest::currentTestFailed()) return;
    _verifyFramePrereq(QStringLiteral("setupComponentSensors"), /*expectPrereqShown=*/true);
    if (QTest::currentTestFailed()) return;
    _verifyFramePrereq(QStringLiteral("setupComponentRadio"), /*expectPrereqShown=*/true);
    if (QTest::currentTestFailed()) return;

    clickSidebarButton(QStringLiteral("setupComponentFrame"));
    if (QTest::currentTestFailed()) return;

    QQuickItem *quadBox = findVisibleItem(_rootItem, QStringLiteral("apmAirframeBox_Quad"), 3000);
    QVERIFY2(quadBox, "Quad airframe box not found on Frame page");

    expectAppMessage(QRegularExpression(QStringLiteral("Reboot vehicle for changes to take effect")));

    Fact *frameClassFact = mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("FRAME_CLASS"));
    Fact *frameTypeFact = mgr->getParameter(ParameterManager::defaultComponentId, QStringLiteral("FRAME_TYPE"));
    QSignalSpy frameClassAckSpy(frameClassFact, &Fact::vehicleUpdated);
    QVERIFY2(frameClassAckSpy.isValid(), "Failed to create FRAME_CLASS vehicleUpdated spy");

    QVERIFY(_clickItemAt(quadBox, 0.5, 0.5, QStringLiteral("apmAirframeBox_Quad")));

    QVERIFY2(QTest::qWaitFor([&] { return frameClassFact->rawValue().toInt() == 1; }, 5000),
             "FRAME_CLASS never set to Quad after clicking the Quad box");
    QCOMPARE(frameTypeFact->rawValue().toInt(), 1);

    QVERIFY2(QTest::qWaitFor([&] { return frameClassAckSpy.count() >= 1; }, 5000),
             "FRAME_CLASS write never acked by the vehicle");
    QVERIFY2(rejectDialog(5000), "Reboot app message dialog never shown");
    verifyExpectedLogMessage();
    if (QTest::currentTestFailed()) return;

    QVERIFY2(QTest::qWaitFor([&] { return frameComp->setupComplete(); }, 5000),
             "Frame setupComplete still false after selecting Quad frame");
    _verifySetupIndicator(QStringLiteral("setupComponentFrame"), /*expectSetupComplete=*/true);
    if (QTest::currentTestFailed()) return;

    _verifySetupIndicator(QStringLiteral("setupComponentSensors"), /*expectSetupComplete=*/false);
    if (QTest::currentTestFailed()) return;
    _verifySetupIndicator(QStringLiteral("setupComponentRadio"), /*expectSetupComplete=*/false);
    if (QTest::currentTestFailed()) return;

    QVERIFY2(autopilot->prerequisiteSetup(sensorsComp).isEmpty(), "Sensors still has a prerequisite after frame selection");
    QVERIFY2(autopilot->prerequisiteSetup(radioComp).isEmpty(), "Radio still has a prerequisite after frame selection");

    _verifyFramePrereq(QStringLiteral("setupComponentSensors"), /*expectPrereqShown=*/false);
    if (QTest::currentTestFailed()) return;
    _verifyFramePrereq(QStringLiteral("setupComponentRadio"), /*expectPrereqShown=*/false);
    if (QTest::currentTestFailed()) return;

    navigateToAPMSensorsPage();
    if (QTest::currentTestFailed()) return;

    runAPMFullAccelCal();
    if (QTest::currentTestFailed()) return;
    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    runAPMCompassCal();
    if (QTest::currentTestFailed()) return;
    waitForParamRefreshQuiet(vehicle);
    if (QTest::currentTestFailed()) return;

    QVERIFY2(QTest::qWaitFor([&] { return sensorsComp->setupComplete(); }, 5000),
             "Sensors setupComplete still false after accel + compass calibration");
    _verifySetupIndicator(QStringLiteral("setupComponentSensors"), /*expectSetupComplete=*/true);
    if (QTest::currentTestFailed()) return;

    QVERIFY2(!radioComp->setupComplete(), "Radio setupComplete despite no radio calibration");
    _verifySetupIndicator(QStringLiteral("setupComponentRadio"), /*expectSetupComplete=*/false);

    });
}
