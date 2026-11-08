#include "AircastLinkFactGroupTest.h"

#include <QtTest/QTest>

#include <algorithm>

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

mavlink_message_t uplink(int32_t kind, uint8_t componentId = MAV_COMP_ID_ONBOARD_COMPUTER)
{
    mavlink_named_value_int_t value{};
    strncpy(value.name, "UPLINK", sizeof(value.name));
    value.value = kind;

    mavlink_message_t message{};
    mavlink_msg_named_value_int_encode(1, componentId, &message, &value);
    return message;
}

QStringList switches(const AircastLinkFactGroup &group)
{
    const QVariantList uplinkSwitches = group.uplinkSwitches();
    QStringList result;
    std::transform(uplinkSwitches.cbegin(), uplinkSwitches.cend(), std::back_inserter(result), [](const QVariant &entry) {
        const QVariantMap uplinkSwitch = entry.toMap();
        return uplinkSwitch[QStringLiteral("from")].toString() + QStringLiteral(" > ") + uplinkSwitch[QStringLiteral("to")].toString();
    });
    return result;
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

void AircastLinkFactGroupTest::_uplinkSwitchesAreChangesNewestFirst()
{
    AircastLinkFactGroup group;

    group.handleMessage(nullptr, uplink(4));
    group.handleMessage(nullptr, uplink(4));
    QVERIFY(switches(group).isEmpty());

    group.handleMessage(nullptr, uplink(3));
    group.handleMessage(nullptr, cellularStatus(87, 100));
    group.handleMessage(nullptr, uplink(2));
    group.handleMessage(nullptr, uplink(2));

    QCOMPARE(switches(group), (QStringList{QStringLiteral("LTE > Wi-Fi"), QStringLiteral("Starlink > LTE")}));
    QCOMPARE(group.uplinkSwitches().first().toMap()[QStringLiteral("clock")].toString().size(), 8);
}

void AircastLinkFactGroupTest::_uplinkSwitchesKeepTheLastFive()
{
    AircastLinkFactGroup group;

    for (int i = 0; i < 31; i++) {
        group.handleMessage(nullptr, uplink(i % 2 ? 2 : 3));
    }

    QCOMPARE(group.uplinkSwitches().size(), AircastLinkFactGroup::kUplinkSwitchesKept);
    QCOMPARE(switches(group).first(), QStringLiteral("Wi-Fi > LTE"));
}

void AircastLinkFactGroupTest::_uplinkAloneShowsTheLink()
{
    AircastLinkFactGroup group;

    group.handleMessage(nullptr, uplink(2, MAV_COMP_ID_AUTOPILOT1));
    QVERIFY(!group.telemetryAvailable());
    QVERIFY(!group.uplinkKnown());

    group.handleMessage(nullptr, uplink(3));

    QVERIFY(group.telemetryAvailable());
    QVERIFY(group.uplinkKnown());
    QVERIFY(!group.cellular());
    QCOMPARE(group.uplink()->enumStringValue(), QStringLiteral("LTE"));
    QCOMPARE(group.quality()->rawValue().toInt(), AircastLinkFactGroup::kQualityUnknown);
    QCOMPARE(group.linkText(), QStringLiteral("LTE --"));
}

void AircastLinkFactGroupTest::_anyModemReportMeansCellular()
{
    AircastLinkFactGroup group;
    QVERIFY(!group.cellular());

    group.handleMessage(nullptr, uplink(4));
    QVERIFY(!group.cellular());
    QCOMPARE(group.linkText(), QStringLiteral("Starlink"));

    group.handleMessage(nullptr, cellularStatus(UINT8_MAX, 100, CELLULAR_NETWORK_RADIO_TYPE_NONE, CELLULAR_STATUS_FLAG_SEARCHING));
    QVERIFY(group.cellular());
    QCOMPARE(group.quality()->rawValue().toInt(), AircastLinkFactGroup::kQualityUnknown);
}

void AircastLinkFactGroupTest::_linkTextNamesTheUplink()
{
    AircastLinkFactGroup group;
    QCOMPARE(group.linkText(), QStringLiteral("--"));

    group.handleMessage(nullptr, cellularStatus(87, 100));
    QCOMPARE(group.linkText(), QStringLiteral("87%"));

    group.handleMessage(nullptr, uplink(3));
    QCOMPARE(group.linkText(), QStringLiteral("LTE 87%"));

    group.handleMessage(nullptr, uplink(4));
    QCOMPARE(group.linkText(), QStringLiteral("Starlink"));

    group.handleMessage(nullptr, uplink(2));
    QCOMPARE(group.linkText(), QStringLiteral("Wi-Fi"));

    group.handleMessage(nullptr, uplink(1));
    QCOMPARE(group.linkText(), QStringLiteral("Ethernet"));

    group.handleMessage(nullptr, uplink(0));
    QCOMPARE(group.linkText(), QStringLiteral("No uplink"));
}

UT_REGISTER_TEST(AircastLinkFactGroupTest, TestLabel::Unit, TestLabel::Vehicle)
