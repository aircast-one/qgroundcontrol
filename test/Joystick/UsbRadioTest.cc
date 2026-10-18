#include "UsbRadioTest.h"

#include <algorithm>
#include <ranges>

#include "MAVLinkLib.h"
#include "UsbRadio.h"

using namespace UsbRadioProtocol;

namespace {

const QByteArray kLine =
    QByteArrayLiteral("$-1024,1024,0,-512,-1024,-1024,0,0,0,0,-1024,0,0,0,0,-160,0,1024,8,-40,-160,0,0,-1024,\r\n");

Frame frameWith(const QList<std::pair<int, uint16_t>>& pairs)
{
    Frame frame{};
    std::ranges::transform(std::views::iota(1, kChannels + 1), frame.begin(), [&pairs](int channel) {
        const auto found = std::ranges::find_if(pairs, [channel](const auto& pair) { return pair.first == channel; });
        return found == pairs.end() ? static_cast<uint16_t>(1500) : found->second;
    });
    return frame;
}

}  // namespace

void UsbRadioTest::_scriptLineReadsAsTheRadioShowsItsChannels_test()
{
    const std::optional<Frame> read = parse(kLine);
    QVERIFY(read);
    QCOMPARE(QList<uint16_t>(read->begin(), read->begin() + 5), QList<uint16_t>({988, 2012, 1500, 1244, 988}));
    QCOMPARE((*read)[10], uint16_t(988));
    QCOMPARE((*read)[15], uint16_t(1420));
    QCOMPARE((*read)[17], uint16_t(2012));
    QCOMPARE(pwm(1536), uint16_t(2200));
    QVERIFY(!parse("$1500,1500\r\n"));
    QVERIFY(!parse("$0,0,0,x,0\r\n"));
    QVERIFY(!parse("GPGGA,0,0,0,0"));
    QCOMPARE((*parse("$0,0,0,0"))[4], uint16_t(UINT16_MAX));
}

void UsbRadioTest::_standbySendsToTheAircraftComputer_test()
{
    QCOMPARE(component(false), uint8_t(1));
    QCOMPARE(component(true), uint8_t(191));
    QVERIFY(standbyAirframe(true, MAV_TYPE_QUADROTOR));
    QVERIFY(standbyAirframe(true, MAV_TYPE_HEXAROTOR));
    QVERIFY(!standbyAirframe(true, MAV_TYPE_FIXED_WING));
    QVERIFY(!standbyAirframe(false, MAV_TYPE_QUADROTOR));
}

void UsbRadioTest::_radioIsToldWhatItsSticksAreDoing_test()
{
    QCOMPARE(echo(Flying::Vehicle, 1), QByteArray("@F,1\n"));
    QCOMPARE(echo(Flying::Standby, 3), QByteArray("@S,3\n"));
    QCOMPARE(echo(Flying::Joystick, 1), QByteArray("@J,1\n"));
    QCOMPARE(echo(Flying::NoVehicle, 0), QByteArray("@N\n"));
    QVERIFY(!parse(echo(Flying::Vehicle, 1)));
}

void UsbRadioTest::_radioHearsBackOnlyWhileItStreams_test()
{
    QVERIFY(!echoDue(std::nullopt, 0, false));
    QVERIFY(echoDue(std::nullopt, 1000, true));
    QVERIFY(!echoDue(1000, 1000 + kEchoEveryMs - 1, true));
    QVERIFY(echoDue(1000, 1000 + kEchoEveryMs, true));
}

void UsbRadioTest::_linesSplitAcrossReadsAreJoined_test()
{
    Lines lines;
    QVERIFY(lines.feed(kLine.left(30)).isEmpty());
    QCOMPARE(lines.feed(kLine.mid(30)).size(), 1);
    QVERIFY(lines.feed(QByteArray(kMaxLineBytes + 1, 'x')).isEmpty());
    QCOMPARE(lines.feed(QByteArray("\n") + kLine + kLine).size(), 2);
}

void UsbRadioTest::_radioFliesAtMostEverySendPeriodAndHandsBackWhenQuiet_test()
{
    Radio radio;
    const Frame sticks = frameWith({{3, 1100}});
    QCOMPARE(radio.step(0, 1, sticks).sends, QList<Send>({Send{1, sticks}}));
    QVERIFY(radio.step(20, 1, sticks).sends.isEmpty());
    QCOMPARE(radio.step(kSendEveryMs, 1, sticks).sends.size(), 1);
    QVERIFY(radio.step(kSendEveryMs + kStaleAfterMs, 1, std::nullopt) == Step{});
    const Step quiet = radio.step(kSendEveryMs + kStaleAfterMs + 1, 1, std::nullopt);
    QCOMPARE(quiet.handedBack, std::optional<int>(1));
    QCOMPARE(quiet.sends, QList<Send>(kReleaseFrames, Send{1, released()}));
    QCOMPARE(released()[0], uint16_t(0));
    QCOMPARE(released()[10], uint16_t(UINT16_MAX - 1));
    QVERIFY(radio.step(kSendEveryMs + kStaleAfterMs + 2, 1, std::nullopt) == Step{});
}

void UsbRadioTest::_losingTheVehicleHandsBackAndANewVehicleIsFlown_test()
{
    Radio radio;
    const Frame sticks = frameWith({});
    (void) radio.step(0, 1, sticks);
    const Step yielded = radio.step(50, std::nullopt, sticks);
    QCOMPARE(yielded.handedBack, std::optional<int>(1));
    QCOMPARE(yielded.sends.size(), kReleaseFrames);
    QVERIFY(radio.step(100, std::nullopt, sticks).sends.isEmpty());
    const Step other = radio.step(150, 2, sticks);
    QCOMPARE(other.handedBack, std::optional<int>());
    QCOMPARE(other.sends, QList<Send>({Send{2, sticks}}));
    const Step moved = radio.step(200, 1, sticks);
    QCOMPARE(moved.handedBack, std::optional<int>(2));
    QVERIFY(moved.sends.last() == (Send{1, sticks}));
}

UT_REGISTER_TEST_LIGHTWEIGHT(UsbRadioTest, TestLabel::Unit, TestLabel::Joystick)
