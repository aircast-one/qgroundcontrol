#include "APMVehicleConfigUITest.h"

#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtTest/QTest>

#include "MockLink.h"
#include "Vehicle.h"

#include <QtCore/QPointer>

UT_REGISTER_TEST(APMVehicleConfigUITest, TestLabel::Integration)

void APMVehicleConfigUITest::init()
{
    if (!apmFirmwareSupported()) {
        QSKIP("ArduPilot support not registered in this build");
    }
    VehicleConfigUITestBase::init();
}

void APMVehicleConfigUITest::_runNavigateVehicleConfig(
    const std::function<MockLink *()> &factory, const QString &vehicleName)
{
    runWithMockLink(factory, [&](QPointer<MockLink> /*mockLink*/, Vehicle *vehicle) {
    navigateToConfigureView();
    if (QTest::currentTestFailed()) return;

    QVERIFY2(clickButton(QStringLiteral("setupSummaryButton")),
             qPrintable(QStringLiteral("%1: Failed to click Summary button").arg(vehicleName)));

    clickThroughAllComponentsAllLocales(vehicle, vehicleName);

    });
}

void APMVehicleConfigUITest::_testArduCopter()
{
    _runNavigateVehicleConfig(
        [] { return MockLink::startAPMArduCopterMockLink(); },
        QStringLiteral("ArduCopter"));
}

void APMVehicleConfigUITest::_testArduPlane()
{
    _runNavigateVehicleConfig(
        [] { return MockLink::startAPMArduPlaneMockLink(); },
        QStringLiteral("ArduPlane"));
}

void APMVehicleConfigUITest::_testArduSub()
{
    QSKIP("ArduSub Tuning page has parameter mismatches with MockLink – skipping pending fix");
}

void APMVehicleConfigUITest::_testArduRover()
{
    _runNavigateVehicleConfig(
        [] { return MockLink::startAPMArduRoverMockLink(); },
        QStringLiteral("ArduRover"));
}
