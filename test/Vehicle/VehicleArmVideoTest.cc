#include "VehicleArmVideoTest.h"

#include <QtCore/QScopeGuard>
#include <QtTest/QTest>

#include "Fact.h"
#include "MockLink.h"
#include "SettingsManager.h"
#include "Vehicle.h"
#include "VideoSettings.h"

UT_REGISTER_TEST(VehicleArmVideoTest, TestLabel::Integration, TestLabel::Vehicle)

void VehicleArmVideoTest::_armingTurnsVideoBackOn()
{
    VideoSettings *const video = SettingsManager::instance()->videoSettings();
    const QVariant savedStream = video->streamEnabled()->rawValue();
    const QVariant savedDisable = video->disableWhenDisarmed()->rawValue();
    const auto restore = qScopeGuard([video, savedStream, savedDisable] {
        video->streamEnabled()->setRawValue(savedStream);
        video->disableWhenDisarmed()->setRawValue(savedDisable);
    });
    video->disableWhenDisarmed()->setRawValue(true);
    video->streamEnabled()->setRawValue(false);

    mockLink()->setArmed(true);
    QVERIFY_TRUE_WAIT(vehicle()->armed(), TestTimeout::mediumMs());
    QVERIFY(video->streamEnabled()->rawValue().toBool());

    mockLink()->setArmed(false);
    QVERIFY_TRUE_WAIT(!vehicle()->armed(), TestTimeout::mediumMs());
    QVERIFY(!video->streamEnabled()->rawValue().toBool());
}
