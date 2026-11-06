#include "UsbRadioTest.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <algorithm>
#include <ranges>

#include "MAVLinkLib.h"
#include "UsbRadio.h"

using namespace UsbRadioProtocol;

namespace {

const QByteArray kLine =
    QByteArrayLiteral("$-1024,1024,0,-512,-1024,-1024,0,0,0,0,-1024,0,0,0,0,-160,0,1024,8,-40,-160,0,0,-1024,\r\n");

Line lineWith(const QList<std::pair<int, uint16_t>>& pairs)
{
    Line line{};
    std::ranges::transform(std::views::iota(1, kMaxValues + 1), line.begin(), [&pairs](int channel) {
        const auto found = std::ranges::find_if(pairs, [channel](const auto& pair) { return pair.first == channel; });
        return found == pairs.end() ? static_cast<uint16_t>(1500) : found->second;
    });
    return line;
}

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
    const std::optional<Line> read = parse(kLine);
    QVERIFY(read);
    QCOMPARE(QList<uint16_t>(read->begin(), read->begin() + 5), QList<uint16_t>({988, 2012, 1500, 1244, 988}));
    QCOMPARE((*read)[10], uint16_t(988));
    QCOMPARE((*read)[15], uint16_t(1420));
    QCOMPARE((*read)[17], uint16_t(2012));
    QCOMPARE((*read)[19], uint16_t(1480));
    QCOMPARE((*read)[23], uint16_t(988));
    QCOMPARE(flightChannels(*read)[17], uint16_t(2012));
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

void UsbRadioTest::_radioLearnsTheVehicleKindAndItsStickChannels_test()
{
    QCOMPARE(echo(Flying::Vehicle, 1, Kind::Copter, Sticks{}), QByteArray("@F,1,C,1,2,3,4\n"));
    QCOMPARE(echo(Flying::Standby, 2, Kind::Plane, Sticks{2, 1, 3, 4}), QByteArray("@S,2,P,2,1,3,4\n"));
    QCOMPARE(echo(Flying::Joystick, 3, Kind::Rover, Sticks{}), QByteArray("@J,3,R,1,2,3,4\n"));
    QCOMPARE(echo(Flying::Vehicle, 4, Kind::Unknown, Sticks{2, 1, 3, 4}), QByteArray("@F,4\n"));
    QCOMPARE(echo(Flying::NoVehicle, 0, Kind::Copter, Sticks{}), QByteArray("@N\n"));
}

void UsbRadioTest::_vehicleKindFollowsTheAirframe_test()
{
    QCOMPARE(kind(MAV_TYPE_QUADROTOR), Kind::Copter);
    QCOMPARE(kind(MAV_TYPE_HELICOPTER), Kind::Copter);
    QCOMPARE(kind(MAV_TYPE_DODECAROTOR), Kind::Copter);
    QCOMPARE(kind(MAV_TYPE_SUBMARINE), Kind::Copter);
    QCOMPARE(kind(MAV_TYPE_FIXED_WING), Kind::Plane);
    QCOMPARE(kind(MAV_TYPE_VTOL_TILTROTOR), Kind::Plane);
    QCOMPARE(kind(MAV_TYPE_GROUND_ROVER), Kind::Rover);
    QCOMPARE(kind(MAV_TYPE_SURFACE_BOAT), Kind::Rover);
    QCOMPARE(kind(MAV_TYPE_GENERIC), Kind::Unknown);
}

void UsbRadioTest::_unmappedSticksFallBackToTheFirstFourChannels_test()
{
    QCOMPARE(sticks(2, 1, 3, 4), (Sticks{2, 1, 3, 4}));
    QCOMPARE(sticks(0, 0, 0, 0), Sticks{});
    QCOMPARE(sticks(5, 0, 7, 0), (Sticks{5, 2, 7, 4}));
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

void UsbRadioTest::_aLineSplicedByARadioBufferOverrunIsDroppedNotReadShifted_test()
{
    const QList<QByteArray> fields = kLine.trimmed().split(',');
    const QByteArray spliced = fields.mid(0, 10).join(',') + ',' + fields.mid(11).join(',') + "\r\n";
    QVERIFY(parse(spliced).has_value());

    Lines lines;
    QCOMPARE(lines.feed(kLine).size(), 1);
    QVERIFY2(lines.feed(spliced).isEmpty(), "a line one value short of its neighbours would move CH12 into CH11");
    QVERIFY(lines.feed(kLine).isEmpty());
    QCOMPARE(lines.feed(kLine).size(), 1);
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

void UsbRadioTest::_statusSaysWhatToDoWhenTheRadioIsBusyOrSilent_test()
{
    QVERIFY(busyText(QStringLiteral("/dev/cu.radio")).contains(QStringLiteral("Chrome")));
    QVERIFY(busyText(QStringLiteral("/dev/cu.radio")).contains(QStringLiteral("Only one app can hold the radio")));
    QCOMPARE(silentText(QStringLiteral("/dev/cu.radio"), kSilentHelpAfterMs - 1),
             QStringLiteral("Waiting for the radio on /dev/cu.radio"));
    QVERIFY(silentText(QStringLiteral("/dev/cu.radio"), kSilentHelpAfterMs)
                .contains(QStringLiteral("SYS → Tools → Aircast USB radio")));
    QVERIFY(
        silentText(QStringLiteral("/dev/cu.radio"), kSilentHelpAfterMs).contains(QStringLiteral("USB Serial (VCP)")));
    QCOMPARE(flightText(Flying::Standby, 3), QStringLiteral("Network standby for vehicle 3"));
    QCOMPARE(flightText(Flying::Vehicle, 1), QStringLiteral("Flying vehicle 1"));
}

void UsbRadioTest::_radioCardIsFoundByItsToolsFolder_test()
{
    QTemporaryDir disk;
    QTemporaryDir card;
    QVERIFY(QDir(card.path()).mkpath(QStringLiteral("SCRIPTS/TOOLS")));
    QCOMPARE(toolsFolder({disk.path(), card.path()}), QDir(card.path()).filePath(QStringLiteral("SCRIPTS/TOOLS")));
    QCOMPARE(toolsFolder({disk.path()}), QString());
}

void UsbRadioTest::_savedScriptReplacesTheOldOneAndItsCompiledCopy_test()
{
    QTemporaryDir tools;
    const QDir dir(tools.path());
    QFile old(dir.filePath(QStringLiteral("aircast-usb.lua")));
    QVERIFY(old.open(QIODevice::WriteOnly));
    (void) old.write("old");
    old.close();
    QFile compiled(dir.filePath(QStringLiteral("aircast-usb.luac")));
    QVERIFY(compiled.open(QIODevice::WriteOnly));
    compiled.close();
    QCOMPARE(saveScript(tools.path()), QString());
    QFile saved(dir.filePath(QStringLiteral("aircast-usb.lua")));
    QVERIFY(saved.open(QIODevice::ReadOnly));
    QFile bundled(QStringLiteral(":/edgetx/aircast-usb.lua"));
    QVERIFY(bundled.open(QIODevice::ReadOnly));
    QCOMPARE(saved.readAll(), bundled.readAll());
    QVERIFY(!QFile::exists(dir.filePath(QStringLiteral("aircast-usb.luac"))));
    QVERIFY(QFileInfo(saved).isWritable());
    QCOMPARE(saveScript(tools.path()), QString());
}

void UsbRadioTest::_aPortStreamingTheScriptIsARadioAndMavlinkIsNot_test()
{
    Sniffer radio;
    QCOMPARE(radio.feed(kLine.left(40)), Verdict::Undecided);
    QCOMPARE(radio.feed(kLine.mid(40) + kLine), Verdict::Undecided);
    QCOMPARE(radio.feed(kLine), Verdict::Radio);

    mavlink_message_t message;
    (void) mavlink_msg_heartbeat_pack(1, 1, &message, MAV_TYPE_QUADROTOR, MAV_AUTOPILOT_ARDUPILOTMEGA, 0, 0, 0);
    QByteArray frame(MAVLINK_MAX_PACKET_LEN, Qt::Uninitialized);
    frame.resize(mavlink_msg_to_send_buffer(reinterpret_cast<uint8_t*>(frame.data()), &message));
    Sniffer autopilot;
    QCOMPARE(autopilot.feed(frame), Verdict::Undecided);
    QCOMPARE(autopilot.feed(frame.repeated(static_cast<qsizetype>(kSniffBytes / frame.size()) + 1)), Verdict::NotRadio);
}

void UsbRadioTest::_indicatorStateFollowsTheRadioAndWhoFlies_test()
{
    QCOMPARE(state(false, true, Flying::Vehicle), State::Off);
    QCOMPARE(state(true, false, Flying::Vehicle), State::Stopped);
    QCOMPARE(state(true, true, Flying::Vehicle), State::Live);
    QCOMPARE(state(true, true, Flying::NoVehicle), State::Ready);
    QCOMPARE(state(true, true, Flying::Joystick), State::Paused);
    QCOMPARE(state(true, true, Flying::Standby), State::Standby);
    QCOMPARE(stateName(State::Live), QStringLiteral("live"));
    QCOMPARE(stateName(State::Stopped), QStringLiteral("stopped"));
    QCOMPARE(stateName(State::Off), QStringLiteral("off"));
}

UT_REGISTER_TEST_LIGHTWEIGHT(UsbRadioTest, TestLabel::Unit, TestLabel::Joystick)

void UsbRadioTest::_cameraChannelsSteerTheGimbalZoomPictureAndPalette_test()
{
    CameraSticks sticks;
    QVERIFY2(sticks.step(lineWith({{kPitchChannel, 2012}, {kYawChannel, 1471}}), 0).isEmpty(),
             "plugging the radio in leaves the camera where it is");
    QCOMPARE(sticks.step(lineWith({{kPitchChannel, 2012}, {kYawChannel, 2012}}), 1000),
             (QList<Move>{Move{Move::Kind::Aim, 0.0f, kYawLimit}}));
    QCOMPARE(sticks.step(lineWith({{kPitchChannel, 988}, {kYawChannel, 988}}), 2000),
             (QList<Move>{Move{Move::Kind::Aim, kPitchDown, -kYawLimit}}));
    QVERIFY2(sticks.step(lineWith({{kPitchChannel, 988 + kSteadyPwm - 1}, {kYawChannel, 988}}), 2050).isEmpty(),
             "slider noise does not move the camera");
    QVERIFY2(sticks.step(lineWith({{kPitchChannel, 1500}, {kYawChannel, 988}}), 2000 + kCameraResendMs - 1).isEmpty(),
             "a moving slider is sent at most ten times a second");
    QCOMPARE(sticks.step(lineWith({{kPitchChannel, 1500}, {kYawChannel, 988}}), 2000 + kCameraResendMs).size(), 1);

    CameraSticks knob;
    (void) knob.step(lineWith({{kZoomChannel, 1422}}), 0);
    QCOMPARE(knob.step(lineWith({{kZoomChannel, 2012}}), 1000),
             (QList<Move>{Move{Move::Kind::Zoom, 0.0f, 0.0f, 100.0f}}));
    QCOMPARE(knob.step(lineWith({{kZoomChannel, 988}}), 2000), (QList<Move>{Move{Move::Kind::Zoom, 0.0f, 0.0f, 0.0f}}));

    CameraSticks picture;
    (void) picture.step(lineWith({{kPictureChannel, 2012}}), 0);
    QCOMPARE(picture.step(lineWith({{kPictureChannel, 1500}}), 10),
             (QList<Move>{Move{Move::Kind::Picture, 0.0f, 0.0f, 0.0f, 1}}));
    QCOMPARE(picture.step(lineWith({{kPictureChannel, 988}}), 20),
             (QList<Move>{Move{Move::Kind::Picture, 0.0f, 0.0f, 0.0f, 0}}));
    QVERIFY2(picture.step(lineWith({{kPictureChannel, 990}}), 30).isEmpty(),
             "a switch left where it is changes nothing");
    QCOMPARE(picture.step(lineWith({{kPictureChannel, 2012}}), 40),
             (QList<Move>{Move{Move::Kind::Picture, 0.0f, 0.0f, 0.0f, 3}}));

    CameraSticks palette;
    (void) palette.step(lineWith({{kPaletteChannel, 988}}), 0);
    QCOMPARE(palette.step(lineWith({{kPaletteChannel, 2012}}), 10), (QList<Move>{Move{Move::Kind::NextPalette}}));
    QVERIFY2(palette.step(lineWith({{kPaletteChannel, 2012}}), 20).isEmpty(), "holding it down does not keep stepping");
    QVERIFY(palette.step(lineWith({{kPaletteChannel, 988}}), 30).isEmpty());
    QCOMPARE(palette.step(lineWith({{kPaletteChannel, 2012}}), 40), (QList<Move>{Move{Move::Kind::NextPalette}}));

    CameraSticks unsent;
    Line nothing{};
    nothing.fill(UINT16_MAX);
    QVERIFY(unsent.step(nothing, 0).isEmpty());
    QVERIFY2(unsent.step(nothing, 1000).isEmpty(), "channels the radio did not send steer nothing");
}

void UsbRadioTest::_dropSwitchDropsOncePerFlipUpAndBomarmArmsIt_test()
{
    CameraSticks sticks;
    (void) sticks.step(lineWith({{kDropChannel, 988}, {kBayChannel, 988}}), 0);
    QCOMPARE(sticks.step(lineWith({{kDropChannel, 2012}, {kBayChannel, 988}}), 10),
             (QList<Move>{Move{Move::Kind::Drop, 0.0f, 0.0f, 0.0f, 1}}));
    QVERIFY(sticks.step(lineWith({{kDropChannel, 2012}}), 20).isEmpty());
    QVERIFY2(sticks.step(lineWith({{kDropChannel, 988}}), 30).isEmpty(), "flipping it back down is not a drop");
    QCOMPARE(sticks.step(lineWith({{kDropChannel, 2012}, {kBayChannel, 1602}}), 40),
             (QList<Move>{Move{Move::Kind::Drop, 0.0f, 0.0f, 0.0f, 4}}));
    CameraSticks pluggedInHigh;
    QVERIFY2(pluggedInHigh.step(lineWith({{kDropChannel, 2012}}), 0).isEmpty(),
             "a radio plugged in with the switch already up drops nothing");

    QCOMPARE(QList<int>({bayOf(988), bayOf(1193), bayOf(1398), bayOf(1602), bayOf(1807), bayOf(2012)}),
             QList<int>({1, 2, 3, 4, 5, 6}));
    const std::optional<Switches> up = switches(lineWith({{kArmChannel, 2012}, {kBayChannel, 1398}}));
    QVERIFY(up);
    QCOMPARE(*up, (Switches{true, 3}));
    QVERIFY2(!switches(lineWith({{kArmChannel, 1500}}))->armed, "the middle position is safe");
    QVERIFY(!switches(lineWith({{kArmChannel, 988}}))->armed);
    Line eleven{};
    eleven.fill(UINT16_MAX);
    std::fill_n(eleven.begin(), 11, static_cast<uint16_t>(1500));
    QVERIFY2(!switches(eleven), "a radio without CH12 arms nothing");
}
