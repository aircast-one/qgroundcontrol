#include "PX4AirframeSetupUITest.h"

#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

#include "MockLink.h"
#include "MultiVehicleManager.h"
#include "Vehicle.h"

UT_REGISTER_TEST(PX4AirframeSetupUITest, TestLabel::Integration)

void PX4AirframeSetupUITest::_navigateToAirframePanel()
{
    navigateToConfigureView();
    if (QTest::currentTestFailed()) return;

    QQuickItem *airframeBtn = findVisibleItem(_rootItem, QStringLiteral("setupComponentAirframe"), 2000);
    QVERIFY2(airframeBtn, "setupComponentAirframe button not found");

    QVERIFY2(waitForCondition([airframeBtn] { return airframeBtn->property("badgeVisible").toBool(); },
                              TestTimeout::mediumMs(), QStringLiteral("airframe setup badge")),
             "Airframe button does not show config required after SYS_AUTOSTART reset");

    clickSidebarButton(QStringLiteral("setupComponentAirframe"));
    if (QTest::currentTestFailed()) return;

    QQuickItem *applyButton = findVisibleItem(_rootItem, QStringLiteral("airframeSetup_applyButton"), 3000);
    QVERIFY2(applyButton, "Apply and Restart button not found after opening Airframe page");

    QVERIFY2(!applyButton->property("primary").toBool(),
             "Apply and Restart button shows as primary before an airframe is selected");

    QQuickItem *firstTypeBox = findVisibleItem(_rootItem, QStringLiteral("airframeTypeBox_0"), 3000);
    QVERIFY2(firstTypeBox, "First airframe type box not found");
    QCOMPARE(firstTypeBox->property("airframeTypeName").toString(), QStringLiteral("Quadrotor x"));

    for (int i = 0; ; i++) {
        QQuickItem *typeBox = findVisibleItem(_rootItem, QStringLiteral("airframeTypeBox_%1").arg(i), 0);
        if (!typeBox) {
            QVERIFY2(i > 0, "No airframe type boxes found");
            break;
        }
        QVERIFY2(!typeBox->property("airframeTypeSelected").toBool(),
                 qPrintable(QStringLiteral("Airframe type box selected with no airframe configured: %1")
                                .arg(typeBox->property("airframeTypeName").toString())));
    }

    QVERIFY2(!findVisibleItem(_rootItem, QStringLiteral("setupMessagePanel"), 0),
             "Prerequisite setup message panel shown instead of Airframe page");
}

void PX4AirframeSetupUITest::_verifyAirframePrereq(const QString &compObjectName, bool expectPrereqShown)
{
    clickSidebarButton(compObjectName);
    if (QTest::currentTestFailed()) return;

    QQuickItem *messagePanel = findVisibleItem(_rootItem, QStringLiteral("setupMessagePanel"), expectPrereqShown ? 3000 : 0);
    if (expectPrereqShown) {
        QVERIFY2(messagePanel,
                 qPrintable(QStringLiteral("Airframe prerequisite message panel not shown for: %1").arg(compObjectName)));
    } else {
        QVERIFY2(!messagePanel,
                 qPrintable(QStringLiteral("Prerequisite message panel unexpectedly shown for: %1").arg(compObjectName)));
    }
}

void PX4AirframeSetupUITest::_testNavigateToAirframe()
{
    runWithMockLink(
        [] { return MockLink::startPX4MockLink(); },
        [&](QPointer<MockLink> mockLink, Vehicle *vehicle) {
    mockLink->setResetSysAutostartOnParamReset(true);
    resetParamsToFirmwareDefaults(vehicle, QStringLiteral("SYS_AUTOSTART"));
    if (QTest::currentTestFailed()) return;

    _navigateToAirframePanel();
    });
}

void PX4AirframeSetupUITest::_testAirframePrereqPages()
{
    runWithMockLink(
        [] { return MockLink::startPX4MockLink(); },
        [&](QPointer<MockLink> mockLink, Vehicle *vehicle) {
    mockLink->setResetSysAutostartOnParamReset(true);
    resetParamsToFirmwareDefaults(vehicle, QStringLiteral("SYS_AUTOSTART"));
    if (QTest::currentTestFailed()) return;

    navigateToConfigureView();
    if (QTest::currentTestFailed()) return;

    struct PrereqCheck {
        const char *objectName;
        bool expectPrereqShown;
    };
    const PrereqCheck checks[] = {
        { .objectName = "setupComponentSensors",     .expectPrereqShown = true  },
        { .objectName = "setupComponentPower",       .expectPrereqShown = true  },
        { .objectName = "setupComponentSafety",      .expectPrereqShown = true  },
        { .objectName = "setupComponentPIDTuning",   .expectPrereqShown = true  },
        { .objectName = "setupComponentFlightModes", .expectPrereqShown = true  },
        { .objectName = "setupComponentRadio",       .expectPrereqShown = false },
        { .objectName = "setupComponentJoystick",    .expectPrereqShown = false },
        { .objectName = "setupComponentAirframe",    .expectPrereqShown = false },
    };
    for (const PrereqCheck &check : checks) {
        _verifyAirframePrereq(QLatin1String(check.objectName), check.expectPrereqShown);
        if (QTest::currentTestFailed()) return;
    }
    });
}

void PX4AirframeSetupUITest::_testApplyAirframe()
{
    runWithMockLink(
        [] { return MockLink::startPX4MockLink(); },
        [&](QPointer<MockLink> mockLink, Vehicle *vehicle) {
    mockLink->setResetSysAutostartOnParamReset(true);
    resetParamsToFirmwareDefaults(vehicle, QStringLiteral("SYS_AUTOSTART"));
    if (QTest::currentTestFailed()) return;

    _navigateToAirframePanel();
    if (QTest::currentTestFailed()) return;

    QQuickItem *combo = findVisibleItem(_rootItem, QStringLiteral("Quadrotor xComboBox"), 3000);
    QVERIFY2(combo, "Quadrotor x combo box not found");
    const QVariantList comboModel = combo->property("model").toList();
    QVERIFY2(!comboModel.isEmpty(), "Quadrotor x combo model is empty");
    QObject *firstAirframe = comboModel.first().value<QObject*>();
    QVERIFY2(firstAirframe, "First Quadrotor x airframe entry is not a QObject");
    const int expectedAutostartId = firstAirframe->property("autostartId").toInt();
    QVERIFY2(expectedAutostartId != 0, "First Quadrotor x airframe has no autostart id");

    QQuickItem *typeBox = findVisibleItem(_rootItem, QStringLiteral("airframeTypeBox_0"), 3000);
    QVERIFY2(typeBox, "airframeTypeBox_0 not found");
    QVERIFY(_clickItemAt(typeBox, 0.5, 0.5, QStringLiteral("airframeTypeBox_0")));

    QVERIFY2(QTest::qWaitFor([&] { return typeBox->property("airframeTypeSelected").toBool(); }, 3000),
             "Quadrotor x type box not selected after clicking it");

    QQuickItem *applyButton = findVisibleItem(_rootItem, QStringLiteral("airframeSetup_applyButton"), 3000);
    QVERIFY2(applyButton, "Apply and Restart button not found");
    QVERIFY2(QTest::qWaitFor([&] { return applyButton->property("primary").toBool(); }, 3000),
             "Apply and Restart button not primary after selecting an airframe");

    QSignalSpy spyCmdResult(vehicle, &Vehicle::mavCommandResult);
    QVERIFY2(spyCmdResult.isValid(), "Failed to create mavCommandResult spy");

    expectAppMessage(QRegularExpression(QStringLiteral("Reboot vehicle for changes to take effect")));

    QVERIFY2(clickButton(QStringLiteral("airframeSetup_applyButton")), "Failed to click Apply and Restart");
    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("popupDialog_acceptButton"), 3000),
             "Apply confirmation dialog not shown");
    QVERIFY2(acceptDialog(), "Failed to confirm Apply dialog");

    QVERIFY2(QTest::qWaitFor([&] {
                 return mockLink && mockLink->paramValue(MAV_COMP_ID_AUTOPILOT1, QStringLiteral("SYS_AUTOSTART")).toInt() == expectedAutostartId;
             }, 10000),
             "SYS_AUTOSTART never reached the expected autostart id on MockLink");
    QVERIFY2(QTest::qWaitFor([&] {
                 return mockLink && mockLink->paramValue(MAV_COMP_ID_AUTOPILOT1, QStringLiteral("SYS_AUTOCONFIG")).toInt() == 1;
             }, 10000),
             "SYS_AUTOCONFIG never set to 1 on MockLink");

    const auto rebootAccepted = [&] {
        for (const QList<QVariant> &args : spyCmdResult) {
            if ((args.at(2).toInt() == MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN) && (args.at(3).toInt() == MAV_RESULT_ACCEPTED)) {
                return true;
            }
        }
        return false;
    };
    QVERIFY2(QTest::qWaitFor(rebootAccepted, 10000),
             "MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN never accepted by MockLink");

    QVERIFY2(QTest::qWaitFor([&] { return MultiVehicleManager::instance()->activeVehicle() == nullptr; }, 10000),
             "Vehicle never disconnected after Apply and Restart");

    QVERIFY2(rejectDialog(5000), "Vehicle reboot-required dialog never shown");
    verifyExpectedLogMessage();
    });
}
