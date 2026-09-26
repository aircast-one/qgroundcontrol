#include "PX4VehicleConfigUITest.h"

#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtTest/QTest>

#include "MockLink.h"

#include <QtCore/QPointer>
#include "Vehicle.h"

UT_REGISTER_TEST(PX4VehicleConfigUITest, TestLabel::Integration)

void PX4VehicleConfigUITest::_testNavigateVehicleConfig()
{
    runWithMockLink(
        [] { return MockLink::startPX4MockLink(); },
        [&](QPointer<MockLink> /*mockLink*/, Vehicle *vehicle) {
    navigateToConfigureView();
    if (QTest::currentTestFailed()) return;

    QVERIFY2(clickButton(QStringLiteral("setupSummaryButton")), "Failed to click Summary button");

    clickThroughAllComponentsAllLocales(vehicle);

    });
}

void PX4VehicleConfigUITest::_cycleAxisButtons(const QStringList &axisNames)
{
    for (const QString &name : axisNames) {
        const QString objectName = QStringLiteral("pidTuning_axisButton_") + name;
        QQuickItem *const button = findVisibleItem(_rootItem, objectName, 2000);
        QVERIFY2(button, qPrintable(QStringLiteral("Axis button not found: %1").arg(objectName)));
        QVERIFY2(clickButton(objectName),
                 qPrintable(QStringLiteral("Failed to click axis button: %1").arg(objectName)));
        QVERIFY2(waitForCondition([button] { return button->property("checked").toBool(); }, TestTimeout::shortMs(),
                                  QStringLiteral("%1 selected").arg(objectName)),
                 qPrintable(QStringLiteral("Axis button not selected: %1").arg(objectName)));
    }
}

void PX4VehicleConfigUITest::_testDisconnectWithPIDTuningOpen()
{
    runWithMockLink(
        [] { return MockLink::startPX4MockLink(); },
        [&](QPointer<MockLink> /*mockLink*/, Vehicle * /*vehicle*/) {
    navigateToConfigureView();
    if (QTest::currentTestFailed()) return;

    clickSidebarButton(QStringLiteral("setupComponentPIDTuning"));
    if (QTest::currentTestFailed()) return;

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("setupPanelLoader"), 2000),
             "setupPanelLoader not found after opening PID Tuning");

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("pidTuning_tab_RateController"), 2000),
             "Rate Controller tab not found");
    QVERIFY2(clickButton(QStringLiteral("pidTuning_tab_RateController")), "Failed to click Rate Controller tab");
    _cycleAxisButtons({QStringLiteral("Roll"), QStringLiteral("Pitch"), QStringLiteral("Yaw"), QStringLiteral("Roll")});
    if (QTest::currentTestFailed()) return;

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("pidTuning_tab_AttitudeController"), 2000),
             "Attitude Controller tab not found");
    QVERIFY2(clickButton(QStringLiteral("pidTuning_tab_AttitudeController")), "Failed to click Attitude Controller tab");
    _cycleAxisButtons({QStringLiteral("Roll"), QStringLiteral("Pitch"), QStringLiteral("Yaw"), QStringLiteral("Roll")});
    if (QTest::currentTestFailed()) return;

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("pidTuning_tab_VelocityController"), 2000),
             "Velocity Controller tab not found");
    QVERIFY2(clickButton(QStringLiteral("pidTuning_tab_VelocityController")), "Failed to click Velocity Controller tab");
    _cycleAxisButtons({QStringLiteral("Horizontal"), QStringLiteral("Vertical"), QStringLiteral("Horizontal")});
    if (QTest::currentTestFailed()) return;

    QVERIFY2(findVisibleItem(_rootItem, QStringLiteral("pidTuning_tab_PositionController"), 2000),
             "Position Controller tab not found");
    QVERIFY2(clickButton(QStringLiteral("pidTuning_tab_PositionController")), "Failed to click Position Controller tab");
    _cycleAxisButtons({QStringLiteral("Horizontal"), QStringLiteral("Vertical"), QStringLiteral("Horizontal")});
    if (QTest::currentTestFailed()) return;

    });
}
