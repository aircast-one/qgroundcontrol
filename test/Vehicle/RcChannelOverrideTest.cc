/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "RcChannelOverrideTest.h"
#include "MockLink.h"
#include "MultiVehicleManager.h"
#include "Vehicle.h"

#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

// Counts the RC_CHANNELS_OVERRIDE messages whose channel 13 carries `value`.
static int countOverridesForChan13(const QList<QVariantList>& writes, uint16_t value)
{
    int count = 0;
    mavlink_message_t message;
    mavlink_status_t status;

    for (const QVariantList& write : writes) {
        const QByteArray bytes = write.at(0).toByteArray();
        for (char byte : bytes) {
            if (mavlink_parse_char(MAVLINK_COMM_2, static_cast<uint8_t>(byte), &message, &status)) {
                if (message.msgid == MAVLINK_MSG_ID_RC_CHANNELS_OVERRIDE) {
                    mavlink_rc_channels_override_t decoded;
                    mavlink_msg_rc_channels_override_decode(&message, &decoded);
                    count += decoded.chan13_raw == value ? 1 : 0;
                }
            }
        }
    }
    return count;
}

// Pulls the last RC_CHANNELS_OVERRIDE out of everything the GCS wrote to the link.
static bool lastOverrideSent(const QList<QVariantList>& writes, mavlink_rc_channels_override_t& overrideOut)
{
    bool found = false;
    mavlink_message_t message;
    mavlink_status_t status;

    for (const QVariantList& write : writes) {
        const QByteArray bytes = write.at(0).toByteArray();
        for (char byte : bytes) {
            if (mavlink_parse_char(MAVLINK_COMM_1, static_cast<uint8_t>(byte), &message, &status)) {
                if (message.msgid == MAVLINK_MSG_ID_RC_CHANNELS_OVERRIDE) {
                    mavlink_msg_rc_channels_override_decode(&message, &overrideOut);
                    found = true;
                }
            }
        }
    }
    return found;
}

void RcChannelOverrideTest::_overrideHoldsOneChannelAndReleasesTheRest()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    QSignalSpy spy(_mockLink, &MockLink::writeBytesQueuedSignal);
    vehicle->setRcChannelOverride(13, 1750);
    QVERIFY(spy.wait());

    mavlink_rc_channels_override_t sent{};
    QVERIFY(lastOverrideSent(spy, sent));
    QCOMPARE(sent.chan13_raw, static_cast<uint16_t>(1750));
    // Every channel the user did not configure must be left to the vehicle, not zeroed.
    QCOMPARE(sent.chan1_raw, static_cast<uint16_t>(UINT16_MAX));
    QCOMPARE(sent.chan18_raw, static_cast<uint16_t>(UINT16_MAX));
}

void RcChannelOverrideTest::_clearHandsTheChannelBackToTheTransmitter()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    vehicle->setRcChannelOverride(13, 1750);

    QSignalSpy spy(_mockLink, &MockLink::writeBytesQueuedSignal);
    vehicle->clearRcChannelOverrides();
    QVERIFY(spy.wait());

    mavlink_rc_channels_override_t sent{};
    QVERIFY(lastOverrideSent(spy, sent));
    QCOMPARE(sent.chan13_raw, static_cast<uint16_t>(0));
}

void RcChannelOverrideTest::_outOfRangeChannelIsRejected()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    QSignalSpy spy(_mockLink, &MockLink::writeBytesQueuedSignal);
    vehicle->setRcChannelOverride(19, 1750);

    mavlink_rc_channels_override_t sent{};
    QVERIFY(!lastOverrideSent(spy, sent));
}

// A single release packet can be dropped, leaving the channel overridden until the vehicle's own
// RC_OVERRIDE_TIME expires. The release repeats for a few ticks, then the resend loop must stop.
void RcChannelOverrideTest::_releaseIsRepeatedThenStops()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    vehicle->setRcChannelOverride(13, 1750);

    QSignalSpy spy(_mockLink, &MockLink::writeBytesQueuedSignal);
    vehicle->clearRcChannelOverrides();

    // One immediate release plus one per release tick.
    QTRY_COMPARE_WITH_TIMEOUT(countOverridesForChan13(spy, 0), 4, 5000);

    // The link keeps carrying other traffic, so a further write proves time passed; no more
    // releases may ride along with it.
    QVERIFY(spy.wait(2000));
    QCOMPARE(countOverridesForChan13(spy, 0), 4);
}

// An on-screen control can ask for anything its configured min/max allow, but only a real RC
// pulse width may go on the wire; an unclamped value would be sent to the vehicle verbatim.
void RcChannelOverrideTest::_pwmIsClampedToTheRcRange()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    QSignalSpy highSpy(_mockLink, &MockLink::writeBytesQueuedSignal);
    vehicle->setRcChannelOverride(13, 9000);
    QVERIFY(highSpy.wait());

    mavlink_rc_channels_override_t sent{};
    QVERIFY(lastOverrideSent(highSpy, sent));
    QCOMPARE(sent.chan13_raw, static_cast<uint16_t>(2200));

    QSignalSpy lowSpy(_mockLink, &MockLink::writeBytesQueuedSignal);
    vehicle->setRcChannelOverride(13, -500);
    QVERIFY(lowSpy.wait());

    QVERIFY(lastOverrideSent(lowSpy, sent));
    QCOMPARE(sent.chan13_raw, static_cast<uint16_t>(800));
}

// Releasing when no channel is held must stay silent. Sending zeros regardless would hand the
// vehicle a release for channels this ground station never took, on every call.
void RcChannelOverrideTest::_releasingWithNothingHeldSendsNothing()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);
    QVERIFY(!vehicle->rcChannelOverrideActive());

    QSignalSpy spy(_mockLink, &MockLink::writeBytesQueuedSignal);
    vehicle->clearRcChannelOverrides();

    // The link keeps carrying heartbeats, so wait for traffic and then confirm none of it was
    // an override.
    QVERIFY(spy.wait(2000));

    mavlink_rc_channels_override_t sent{};
    QVERIFY(!lastOverrideSent(spy, sent));
    QVERIFY(!vehicle->rcChannelOverrideActive());
}

// Letting go and immediately grabbing again is ordinary use of a momentary control. If the
// release countdown carried on regardless it would hand the channel back mid-hold, seconds
// after the operator had already taken it again.
void RcChannelOverrideTest::_grabbingAgainDuringReleaseCancelsIt()
{
    _connectMockLinkNoInitialConnectSequence();
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    vehicle->setRcChannelOverride(13, 1750);
    vehicle->clearRcChannelOverrides();
    vehicle->setRcChannelOverride(13, 1850);
    QVERIFY(vehicle->rcChannelOverrideActive());

    // Well past the release countdown, the channel must still be held at the new value and no
    // release must have gone out after it.
    QSignalSpy spy(_mockLink, &MockLink::writeBytesQueuedSignal);
    QTRY_VERIFY_WITH_TIMEOUT(countOverridesForChan13(spy, 1850) > 2, 5000);
    QCOMPARE(countOverridesForChan13(spy, 0), 0);
    QVERIFY(vehicle->rcChannelOverrideActive());

    vehicle->clearRcChannelOverrides();
}
