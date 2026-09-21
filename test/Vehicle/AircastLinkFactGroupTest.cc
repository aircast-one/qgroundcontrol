#include "AircastLinkFactGroupTest.h"

#include <QtTest/QTest>

#include "AircastLinkFactGroup.h"
#include "MAVLinkLib.h"

namespace {

mavlink_message_t cellularStatus(uint8_t quality, uint32_t uploadKiBps, uint8_t type = CELLULAR_NETWORK_RADIO_TYPE_LTE, uint8_t status = CELLULAR_STATUS_FLAG_CONNECTED)
{
    mavlink_cellular_status_t cellular{};
    cellular.quality = quality;
    cellular.link_rx_rate = uploadKiBps;
    cellular.link_tx_rate = UINT32_MAX;
    cellular.type = type;
    cellular.status = status;
    cellular.mcc = UINT16_MAX;
    cellular.mnc = UINT16_MAX;

    mavlink_message_t message{};
    mavlink_msg_cellular_status_encode(1, MAV_COMP_ID_ONBOARD_COMPUTER, &message, &cellular);
    return message;
}

} // namespace

void AircastLinkFactGroupTest::_cellularStatusFillsTheFacts()
{
    AircastLinkFactGroup group;
    QVERIFY(!group.telemetryAvailable());

    group.handleMessage(nullptr, cellularStatus(73, 488));

    QVERIFY(group.telemetryAvailable());
    QCOMPARE(group.quality()->rawValue().toInt(), 73);
    QCOMPARE(group.radioType()->enumStringValue(), QStringLiteral("LTE"));
    QCOMPARE(group.status()->enumStringValue(), QStringLiteral("connected"));
    QCOMPARE(group.videoBitrate()->rawValue().toUInt(), 3998u);
    QCOMPARE(group.qualityHistory(), QVariantList{QVariant(73)});
    QCOMPARE(group.bitrateHistory(), QVariantList{QVariant(3998u)});
}

void AircastLinkFactGroupTest::_unknownQualityIsAGapInTheHistory()
{
    AircastLinkFactGroup group;

    group.handleMessage(nullptr, cellularStatus(40, 100));
    group.handleMessage(nullptr, cellularStatus(UINT8_MAX, 100, CELLULAR_NETWORK_RADIO_TYPE_NONE, CELLULAR_STATUS_FLAG_UNKNOWN));

    QCOMPARE(group.qualityHistory(), (QVariantList{QVariant(40), QVariant(-1)}));
    QCOMPARE(group.status()->enumStringValue(), QStringLiteral("unknown"));
}

void AircastLinkFactGroupTest::_historyKeepsAnHourAtFiveSeconds()
{
    AircastLinkFactGroup group;

    for (int i = 0; i < AircastLinkFactGroup::kHistoryLength + 5; i++) {
        group.handleMessage(nullptr, cellularStatus(static_cast<uint8_t>(i % 100), 10));
    }

    QCOMPARE(group.qualityHistory().size(), AircastLinkFactGroup::kHistoryLength);
    QCOMPARE(group.qualityHistory().first().toInt(), 5);
}

UT_REGISTER_TEST(AircastLinkFactGroupTest, TestLabel::Unit, TestLabel::Vehicle)
