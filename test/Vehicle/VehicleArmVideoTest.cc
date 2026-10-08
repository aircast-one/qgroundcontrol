#include "VehicleArmVideoTest.h"

#include <QtCore/QRegularExpression>
#include <QtCore/QScopeGuard>
#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

#include "Fact.h"
#include "MockLink.h"
#include "MultiVehicleManager.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "Vehicle.h"
#include "VideoManager.h"
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

    QSignalSpy hasVideoChanged(VideoManager::instance(), &VideoManager::hasVideoChanged);
    mockLink()->setArmed(true);
    QVERIFY_TRUE_WAIT(vehicle()->armed(), TestTimeout::mediumMs());
    QVERIFY(video->streamEnabled()->rawValue().toBool());
    QVERIFY(!hasVideoChanged.isEmpty());

    mockLink()->setArmed(false);
    QVERIFY_TRUE_WAIT(!vehicle()->armed(), TestTimeout::mediumMs());
    QVERIFY(!video->streamEnabled()->rawValue().toBool());
}

void VehicleArmVideoTest::_aVehicleArmedBeforeItIsActiveTurnsVideoOn()
{
    VideoSettings *const video = SettingsManager::instance()->videoSettings();
    MultiVehicleManager *const vehicles = MultiVehicleManager::instance();
    const QVariant savedStream = video->streamEnabled()->rawValue();
    const auto restore = qScopeGuard([this, video, vehicles, savedStream] {
        video->streamEnabled()->setRawValue(savedStream);
        vehicles->setActiveVehicle(vehicle());
        (void) QTest::qWaitFor([this, vehicles] { return vehicles->activeVehicle() == vehicle(); }, TestTimeout::mediumMs());
    });
    video->streamEnabled()->setRawValue(false);
    vehicles->setActiveVehicle(nullptr);
    QVERIFY_TRUE_WAIT(!vehicles->activeVehicle(), TestTimeout::mediumMs());

    mockLink()->setArmed(true);
    QVERIFY_TRUE_WAIT(vehicle()->armed(), TestTimeout::mediumMs());
    QVERIFY(video->streamEnabled()->rawValue().toBool());
}

void VehicleArmVideoTest::_anotherVehicleArmingLeavesVideoAlone()
{
    VideoSettings *const video = SettingsManager::instance()->videoSettings();
    MultiVehicleManager *const vehicles = MultiVehicleManager::instance();
    const QVariant savedStream = video->streamEnabled()->rawValue();
    const QVariant savedDisable = video->disableWhenDisarmed()->rawValue();
    expectAppMessage(QRegularExpression(QStringLiteral("Connected to Vehicle")));
    MockLink *const otherLink = MockLink::startPX4MockLink();
    QVERIFY(otherLink);
    const auto restore = qScopeGuard([video, vehicles, savedStream, savedDisable, otherLink] {
        video->streamEnabled()->setRawValue(savedStream);
        video->disableWhenDisarmed()->setRawValue(savedDisable);
        otherLink->disconnect();
        (void) QTest::qWaitFor([vehicles] { return vehicles->vehicles()->count() == 1; }, TestTimeout::longMs());
        UnitTest::settleEventLoopForCleanup();
    });
    QVERIFY_TRUE_WAIT(vehicles->vehicles()->count() == 2, TestTimeout::longMs());
    verifyExpectedLogMessage();
    Vehicle *const other = qobject_cast<Vehicle *>(vehicles->vehicles()->get(1));
    QVERIFY(other && (other != vehicle()));
    QCOMPARE(vehicles->activeVehicle(), vehicle());

    video->disableWhenDisarmed()->setRawValue(true);
    video->streamEnabled()->setRawValue(false);
    otherLink->setArmed(true);
    QVERIFY_TRUE_WAIT(other->armed(), TestTimeout::mediumMs());
    QVERIFY(!video->streamEnabled()->rawValue().toBool());

    video->streamEnabled()->setRawValue(true);
    otherLink->setArmed(false);
    QVERIFY_TRUE_WAIT(!other->armed(), TestTimeout::mediumMs());
    QVERIFY(video->streamEnabled()->rawValue().toBool());
}
