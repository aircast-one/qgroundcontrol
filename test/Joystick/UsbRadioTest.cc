#include "UsbRadioTest.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <algorithm>
#include <array>
#include <numeric>
#include <ranges>

#include "MAVLinkLib.h"
#include "UsbRadio.h"

using namespace UsbRadioProtocol;

namespace {

const Layout kR2d2;

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
    QCOMPARE(echo(Flying::Vehicle, 1), QByteArray("@F,1;18,20,21,22,24,11,12,5,19,23|off|\n"));
    QCOMPARE(echo(Flying::Standby, 3), QByteArray("@S,3;18,20,21,22,24,11,12,5,19,23|off|\n"));
    QCOMPARE(echo(Flying::Joystick, 1), QByteArray("@J,1;18,20,21,22,24,11,12,5,19,23|off|\n"));
    QCOMPARE(echo(Flying::NoVehicle, 0), QByteArray("@N;18,20,21,22,24,11,12,5,19,23|off|\n"));
    QVERIFY(!parse(echo(Flying::Vehicle, 1)));
}

void UsbRadioTest::_radioLearnsTheVehicleKindAndItsStickChannels_test()
{
    QCOMPARE(echo(Flying::Vehicle, 1, Kind::Copter, Sticks{}, kR2d2, 10),
             QByteArray("@F,1,C,1,2,3,4,10;18,20,21,22,24,11,12,5,19,23|off|\n"));
    QCOMPARE(echo(Flying::Standby, 2, Kind::Plane, Sticks{2, 1, 3, 4}),
             QByteArray("@S,2,P,2,1,3,4,0;18,20,21,22,24,11,12,5,19,23|off|\n"));
    QCOMPARE(echo(Flying::Joystick, 3, Kind::Rover, Sticks{}, kR2d2, 8),
             QByteArray("@J,3,R,1,2,3,4,8;18,20,21,22,24,11,12,5,19,23|off|\n"));
    QCOMPARE(echo(Flying::Vehicle, 4, Kind::Unknown, Sticks{2, 1, 3, 4}),
             QByteArray("@F,4;18,20,21,22,24,11,12,5,19,23|off|\n"));
    QCOMPARE(echo(Flying::NoVehicle, 0, Kind::Copter, Sticks{}), QByteArray("@N;18,20,21,22,24,11,12,5,19,23|off|\n"));
}

void UsbRadioTest::_radioHearsThePayloadStateAndTheFlightModeNames_test()
{
    const QStringList copter{QStringLiteral("Loiter"), QStringLiteral("Alt Hold"), QStringLiteral("PosHold"),
                             QStringLiteral("RTL"),    QStringLiteral("Auto"),     QStringLiteral("Land")};
    QCOMPARE(echo(Flying::Vehicle, 1, Kind::Copter, Sticks{}, kR2d2, 10, Payload::Safe, copter),
             QByteArray("@F,1,C,1,2,3,4,10;18,20,21,22,24,11,12,5,19,23|safe|Loiter,AltHold,PosHold,RTL,Auto,Land\n"));
    QCOMPARE(echo(Flying::Vehicle, 1, Kind::Copter, Sticks{}, kR2d2, 10, Payload::Armed, copter),
             QByteArray("@F,1,C,1,2,3,4,10;18,20,21,22,24,11,12,5,19,23|armed|Loiter,AltHold,PosHold,RTL,Auto,Land\n"));
    QCOMPARE(
        echo(Flying::Joystick, 1, Kind::Copter, Sticks{}, kR2d2, 10, Payload::Refused, copter),
        QByteArray("@J,1,C,1,2,3,4,10;18,20,21,22,24,11,12,5,19,23|refused|Loiter,AltHold,PosHold,RTL,Auto,Land\n"));
    QCOMPARE(echo(Flying::Vehicle, 1, Kind::Copter, Sticks{}, kR2d2, 10, Payload::Off, {}),
             QByteArray("@F,1,C,1,2,3,4,10;18,20,21,22,24,11,12,5,19,23|off|\n"));
    QCOMPARE(echo(Flying::NoVehicle, 0), QByteArray("@N;18,20,21,22,24,11,12,5,19,23|off|\n"));

    const QStringList plane{QStringLiteral("Stabilize"),     QStringLiteral("Altitude Hold"),
                            QStringLiteral("Position Hold"), QStringLiteral("Smart RTL"),
                            QStringLiteral("Auto"),          QStringLiteral("Land")};
    const QByteArray capped = echo(Flying::Vehicle, 1, Kind::Copter, Sticks{}, kR2d2, 10, Payload::Refused, plane);
    QVERIFY(capped.size() < kMaxEchoBytes);
    QCOMPARE(capped,
             QByteArray("@F,1,C,1,2,3,4,10;18,20,21,22,24,11,12,5,19,23|refused|Stabili,Altitud,Positio,SmartRT,"
                        "Auto\n"));
    QVERIFY2(echo(Flying::Vehicle, 1, Kind::Copter, Sticks{}, kR2d2, 10, Payload::Safe, plane).endsWith("Auto,Land\n"),
             "a shorter payload word leaves room for every mode");
    QVERIFY(!parse(capped));

    QCOMPARE(payload(std::nullopt, true, true), Payload::Off);
    QCOMPARE(payload(Switches{false}, true, true), Payload::Safe);
    QCOMPARE(payload(Switches{true}, true, true), Payload::Armed);
    QCOMPARE(payload(Switches{true}, false, true), Payload::Refused);
    QCOMPARE(payload(Switches{true}, true, false), Payload::Refused);
    QCOMPARE(payloadName(Payload::Refused), QByteArray("refused"));
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
    QVERIFY2(sticks.step(lineWith({{kR2d2.pitch, 2012}, {kR2d2.yaw, 1471}}), 0).isEmpty(),
             "plugging the radio in leaves the camera where it is");
    QCOMPARE(sticks.step(lineWith({{kR2d2.pitch, 2012}, {kR2d2.yaw, 2012}}), 1000),
             (QList<Move>{Move{Move::Kind::Aim, 0.0f, kYawLimit}}));
    QCOMPARE(sticks.step(lineWith({{kR2d2.pitch, 988}, {kR2d2.yaw, 988}}), 2000),
             (QList<Move>{Move{Move::Kind::Aim, kPitchDown, -kYawLimit}}));
    QVERIFY2(sticks.step(lineWith({{kR2d2.pitch, 988 + kSteadyPwm - 1}, {kR2d2.yaw, 988}}), 2050).isEmpty(),
             "slider noise does not move the camera");
    QVERIFY2(sticks.step(lineWith({{kR2d2.pitch, 1500}, {kR2d2.yaw, 988}}), 2000 + kCameraResendMs - 1).isEmpty(),
             "a moving slider is sent at most ten times a second");
    QCOMPARE(sticks.step(lineWith({{kR2d2.pitch, 1500}, {kR2d2.yaw, 988}}), 2000 + kCameraResendMs).size(), 1);

    CameraSticks knob;
    (void) knob.step(lineWith({{kR2d2.zoom, 1422}}), 0);
    QCOMPARE(knob.step(lineWith({{kR2d2.zoom, 2012}}), 1000),
             (QList<Move>{Move{Move::Kind::Zoom, 0.0f, 0.0f, 100.0f}}));
    QCOMPARE(knob.step(lineWith({{kR2d2.zoom, 988}}), 2000), (QList<Move>{Move{Move::Kind::Zoom, 0.0f, 0.0f, 0.0f}}));

    CameraSticks picture;
    (void) picture.step(lineWith({{kR2d2.picture, 2012}}), 0);
    QCOMPARE(picture.step(lineWith({{kR2d2.picture, 1500}}), 10),
             (QList<Move>{Move{Move::Kind::Picture, 0.0f, 0.0f, 0.0f, 1}}));
    QCOMPARE(picture.step(lineWith({{kR2d2.picture, 988}}), 20),
             (QList<Move>{Move{Move::Kind::Picture, 0.0f, 0.0f, 0.0f, 0}}));
    QVERIFY2(picture.step(lineWith({{kR2d2.picture, 990}}), 30).isEmpty(), "a switch left where it is changes nothing");
    QCOMPARE(picture.step(lineWith({{kR2d2.picture, 2012}}), 40),
             (QList<Move>{Move{Move::Kind::Picture, 0.0f, 0.0f, 0.0f, 3}}));

    CameraSticks palette;
    (void) palette.step(lineWith({{kR2d2.palette, 988}}), 0);
    QCOMPARE(palette.step(lineWith({{kR2d2.palette, 2012}}), 10), (QList<Move>{Move{Move::Kind::NextPalette}}));
    QVERIFY2(palette.step(lineWith({{kR2d2.palette, 2012}}), 20).isEmpty(), "holding it down does not keep stepping");
    QVERIFY(palette.step(lineWith({{kR2d2.palette, 988}}), 30).isEmpty());
    QCOMPARE(palette.step(lineWith({{kR2d2.palette, 2012}}), 40), (QList<Move>{Move{Move::Kind::NextPalette}}));

    CameraSticks unsent;
    Line nothing{};
    nothing.fill(UINT16_MAX);
    QVERIFY(unsent.step(nothing, 0).isEmpty());
    QVERIFY2(unsent.step(nothing, 1000).isEmpty(), "channels the radio did not send steer nothing");
}

void UsbRadioTest::_dropSwitchDropsOncePerFlipUpAndBomarmArmsIt_test()
{
    CameraSticks sticks;
    (void) sticks.step(lineWith({{kR2d2.drop, 988}}), 0);
    QVERIFY2(sticks.step(lineWith({{kR2d2.drop, 2012}}), 10).isEmpty(),
             "the first line with the switch up only arms the drop");
    QCOMPARE(sticks.step(lineWith({{kR2d2.drop, 2012}}), 20), (QList<Move>{Move{Move::Kind::Drop}}));
    QVERIFY(sticks.step(lineWith({{kR2d2.drop, 2012}}), 30).isEmpty());
    QVERIFY2(sticks.step(lineWith({{kR2d2.drop, 988}}), 40).isEmpty(), "flipping it back down is not a drop");
    (void) sticks.step(lineWith({{kR2d2.drop, 2012}}), 50);
    QCOMPARE(sticks.step(lineWith({{kR2d2.drop, 2012}}), 60), (QList<Move>{Move{Move::Kind::Drop}}));
    CameraSticks glitched;
    const std::array<uint16_t, 5> glitch{988, 2012, 988, 2012, 988};
    const QList<Move> glitchMoves =
        std::accumulate(glitch.cbegin(), glitch.cend(), QList<Move>{}, [&glitched](QList<Move> moves, uint16_t drop) {
            return moves + glitched.step(lineWith({{kR2d2.drop, drop}}), 0);
        });
    QVERIFY2(glitchMoves.isEmpty(), "one line with the switch up between lines with it down drops nothing");
    CameraSticks pluggedInHigh;
    QVERIFY2(pluggedInHigh.step(lineWith({{kR2d2.drop, 2012}}), 0).isEmpty(),
             "a radio plugged in with the switch already up drops nothing");

    const std::optional<Switches> up = switches(lineWith({{kR2d2.arm, 2012}}));
    QVERIFY(up);
    QCOMPARE(*up, (Switches{true}));
    QVERIFY2(!switches(lineWith({{kR2d2.arm, 1500}}))->armed, "the middle position is safe");
    QVERIFY(!switches(lineWith({{kR2d2.arm, 988}}))->armed);
    Line eleven{};
    eleven.fill(UINT16_MAX);
    std::fill_n(eleven.begin(), 11, static_cast<uint16_t>(1500));
    QVERIFY2(!switches(eleven), "a radio without CH12 arms nothing");
}

void UsbRadioTest::_cameraAndPayloadChannelsAreSettingsWithTheR2d2LayoutAsDefault_test()
{
    QCOMPARE(kR2d2, (Layout{18, 20, 21, 22, 24, 11, 12, 5, 19, 23}));

    const Layout moved{13, 14, 15, 0, 0, 0, 0, 0, 0, 0};
    CameraSticks sticks;
    (void) sticks.step(lineWith({{13, 1500}, {14, 1500}, {kR2d2.pitch, 1500}}), 0, moved);
    QCOMPARE(sticks.step(lineWith({{13, 2012}, {14, 1500}, {kR2d2.pitch, 1500}}), 1000, moved).size(), 1);
    QVERIFY2(sticks.step(lineWith({{13, 2012}, {14, 1500}, {kR2d2.pitch, 988}}), 2000, moved).isEmpty(),
             "with pitch moved to CH13 the R2D2 pitch channel steers nothing");

    CameraSticks off;
    (void) off.step(lineWith({{kR2d2.drop, 988}}), 0, moved);
    (void) off.step(lineWith({{kR2d2.drop, 2012}}), 10, moved);
    QVERIFY2(off.step(lineWith({{kR2d2.drop, 2012}}), 20, moved).isEmpty(), "a function set to channel 0 is off");
    QVERIFY(!switches(lineWith({{kR2d2.arm, 2012}}), moved));

    QCOMPARE(echo(Flying::Vehicle, 1, Kind::Copter, Sticks{}, moved),
             QByteArray("@F,1,C,1,2,3,4,0;13,14,15,0,0,0,0,0,0,0|off|\n"));
}

void UsbRadioTest::_aSliderMovedBeforeTheGimbalIsFoundAimsOnceItIs_test()
{
    CameraSticks sticks;
    const Ready nothing{false, false, false};
    (void) sticks.step(lineWith({{kR2d2.pitch, 1500}, {kR2d2.yaw, 1500}, {kR2d2.zoom, 1500}}), 0, kR2d2, nothing);
    QVERIFY2(sticks.step(lineWith({{kR2d2.pitch, 988}, {kR2d2.yaw, 1500}, {kR2d2.zoom, 2012}}), 1000, kR2d2, nothing)
                 .isEmpty(),
             "with no gimbal or camera found yet nothing is sent");
    const QList<Move> found =
        sticks.step(lineWith({{kR2d2.pitch, 988}, {kR2d2.yaw, 1500}, {kR2d2.zoom, 2012}}), 2000, kR2d2, Ready{});
    QCOMPARE(found.size(), 2);
    QCOMPARE(found[0].kind, Move::Kind::Aim);
    QCOMPARE(found[0].pitch, -90.0f);
    QCOMPARE(found[1].kind, Move::Kind::Zoom);
}

void UsbRadioTest::_aFunctionOnTheFlightModeChannelIsOff_test()
{
    QCOMPARE(without(kR2d2, 5), (Layout{18, 20, 21, 22, 24, 11, 12, 0, 19, 23}));
    QCOMPARE(without(kR2d2, 10), kR2d2);
    QCOMPARE(without(kR2d2, 0), kR2d2);
    QCOMPARE(echo(Flying::Vehicle, 1, Kind::Copter, Sticks{}, kR2d2, 5),
             QByteArray("@F,1,C,1,2,3,4,5;18,20,21,22,24,11,12,0,19,23|off|\n"));
    QVERIFY2(switches(lineWith({{kR2d2.arm, 2012}, {5, 1500}}), without(kR2d2, 5)) == std::optional(Switches{true}),
             "a vehicle reading flight modes on CH5 takes the bay switch off it, and BOMARM still arms");
    QCOMPARE(namedOn(kR2d2, 5), QStringList({QStringLiteral("bay")}));
    QCOMPARE(namedOn(kR2d2, 19), QStringList({QStringLiteral("tilt")}));
    QCOMPARE(namedOn(kR2d2, 23), QStringList({QStringLiteral("look")}));
    QVERIFY(namedOn(kR2d2, 10).isEmpty());
    QVERIFY(namedOn(kR2d2, 0).isEmpty());
    QCOMPARE(
        clashText(5, {QStringLiteral("bay")}),
        QStringLiteral("The drone reads flight modes on CH5, so the radio's bay switch is off. Move flight modes or "
                       "the switch to another channel."));
}

void UsbRadioTest::_theTiltKnobAimsLikeThePitchSlider_test()
{
    CameraSticks sticks;
    (void) sticks.step(lineWith({{kR2d2.pitch, 1500}, {kR2d2.tilt, 1500}, {kR2d2.yaw, 1500}}), 0);
    QCOMPARE(sticks.step(lineWith({{kR2d2.pitch, 1500}, {kR2d2.tilt, 988}, {kR2d2.yaw, 2012}}), 1000),
             (QList<Move>{Move{Move::Kind::Aim, kPitchDown, kYawLimit}}));
    QCOMPARE(sticks.step(lineWith({{kR2d2.pitch, 1500}, {kR2d2.tilt, 988}, {kR2d2.yaw, 988}}), 2000),
             (QList<Move>{Move{Move::Kind::Aim, kPitchDown, -kYawLimit}}));
    QCOMPARE(sticks.step(lineWith({{kR2d2.pitch, 2012}, {kR2d2.tilt, 988}, {kR2d2.yaw, 988}}), 3000),
             (QList<Move>{Move{Move::Kind::Aim, kPitchAhead, -kYawLimit}}));
    QCOMPARE(sticks.step(lineWith({{kR2d2.pitch, 2000}, {kR2d2.tilt, 1500}, {kR2d2.yaw, 988}}), 4000),
             (QList<Move>{Move{Move::Kind::Aim, -45.0f, -kYawLimit}}));
    QVERIFY2(sticks.step(lineWith({{kR2d2.pitch, 2000 + kSteadyPwm - 1}, {kR2d2.tilt, 1500}, {kR2d2.yaw, 988}}), 5000)
                 .isEmpty(),
             "knob noise does not move the camera");
    const Layout noSlider{0, 20, 21, 22, 24, 11, 12, 5, 19, 23};
    CameraSticks knobOnly;
    (void) knobOnly.step(lineWith({{kR2d2.tilt, 1500}, {kR2d2.yaw, 1500}}), 0, noSlider);
    QCOMPARE(knobOnly.step(lineWith({{kR2d2.tilt, 1500}, {kR2d2.yaw, 2012}}), 1000, noSlider),
             (QList<Move>{Move{Move::Kind::Aim, -45.0f, kYawLimit}}));
}

void UsbRadioTest::_theLookSwitchAimsDownOrAheadWhenItFlips_test()
{
    CameraSticks sticks;
    QVERIFY2(sticks.step(lineWith({{kR2d2.look, 2012}, {kR2d2.yaw, 2012}}), 0).isEmpty(),
             "plugging the radio in with the switch up aims nothing");
    QCOMPARE(sticks.step(lineWith({{kR2d2.look, 988}, {kR2d2.yaw, 2012}}), 10),
             (QList<Move>{Move{Move::Kind::Aim, kPitchAhead, kYawLimit}}));
    QVERIFY2(sticks.step(lineWith({{kR2d2.look, 1500}, {kR2d2.yaw, 2012}}), 20).isEmpty(),
             "the middle position leaves the camera where it is");
    QCOMPARE(sticks.step(lineWith({{kR2d2.look, 2012}, {kR2d2.yaw, 2012}}), 30),
             (QList<Move>{Move{Move::Kind::Aim, kPitchDown, kYawLimit}}));
    QVERIFY2(sticks.step(lineWith({{kR2d2.look, 2010}, {kR2d2.yaw, 2012}}), 40).isEmpty(),
             "a switch left where it is changes nothing");
    const Layout noYaw{18, 0, 21, 22, 24, 11, 12, 5, 19, 23};
    CameraSticks yawOff;
    (void) yawOff.step(lineWith({{kR2d2.look, 1500}, {kR2d2.yaw, 2012}}), 0, noYaw);
    QCOMPARE(yawOff.step(lineWith({{kR2d2.look, 988}, {kR2d2.yaw, 2012}}), 10, noYaw),
             (QList<Move>{Move{Move::Kind::Aim, kPitchAhead, 0.0f}}));
    CameraSticks noGimbal;
    const Ready nothing{false, false, false};
    (void) noGimbal.step(lineWith({{kR2d2.look, 1500}}), 0, kR2d2, nothing);
    QVERIFY(noGimbal.step(lineWith({{kR2d2.look, 2012}}), 10, kR2d2, nothing).isEmpty());
    QCOMPARE(noGimbal.step(lineWith({{kR2d2.look, 2012}}), 20, kR2d2, Ready{}),
             (QList<Move>{Move{Move::Kind::Aim, kPitchDown, 0.0f}}));
}

namespace {

QList<Move> stepAll(CameraSticks& sticks, const QList<std::pair<uint16_t, std::pair<uint16_t, uint16_t>>>& lines)
{
    return std::accumulate(lines.cbegin(), lines.cend(), QList<Move>{}, [&sticks](QList<Move> moves, const auto& at) {
        const auto [arm, sixPos] = at.second;
        return moves + sticks.step(lineWith({{kR2d2.arm, arm}, {kR2d2.bay, sixPos}, {kR2d2.drop, at.first}}), 0);
    });
}

}  // namespace

void UsbRadioTest::_withBomarmArmedPressingASixPosPositionDropsThatBayOnce_test()
{
    CameraSticks sticks;
    const QList<Move> moves = stepAll(sticks, {{988, {2012, 1398}},
                                               {988, {2012, 1398}},
                                               {988, {2012, 1398}},
                                               {988, {2012, 988}},
                                               {988, {2012, 988}},
                                               {988, {2012, 988}},
                                               {988, {2012, 1602}},
                                               {988, {2012, 1602}}});
    QCOMPARE(moves, (QList<Move>{Move{Move::Kind::DropBay, 0.0f, 0.0f, 0.0f, 1},
                                 Move{Move::Kind::DropBay, 0.0f, 0.0f, 0.0f, 4}}));
}

void UsbRadioTest::_aSixPosPressAndTheDropSwitchTogetherDropTheBayOnce_test()
{
    CameraSticks sticks;
    const QList<Move> moves = stepAll(
        sticks,
        {{988, {2012, 988}}, {988, {2012, 988}}, {2012, {2012, 1398}}, {2012, {2012, 1398}}, {2012, {2012, 1398}}});
    QCOMPARE(moves, (QList<Move>{Move{Move::Kind::DropBay, 0.0f, 0.0f, 0.0f, 3}}));
}

void UsbRadioTest::_whileThePayloadIsSafeTheSixPosOnlyPicksTheBay_test()
{
    CameraSticks sticks;
    const QList<Move> moves = stepAll(sticks, {{988, {988, 1398}},
                                               {988, {988, 1398}},
                                               {988, {988, 988}},
                                               {988, {988, 988}},
                                               {988, {2012, 988}},
                                               {988, {2012, 988}},
                                               {988, {2012, 988}},
                                               {988, {988, 988}},
                                               {988, {2012, 1602}},
                                               {988, {2012, 1602}},
                                               {988, {2012, 1602}}});
    QVERIFY2(moves.isEmpty(), "a position picked while safe, or together with arming, does not drop");
}

void UsbRadioTest::_aBayDropsOnItsOwnServoOnlyWhereTheDroneHasOne_test()
{
    const std::array<uint16_t, 6> sixPos{988, 1193, 1398, 1602, 1807, 2012};
    QList<int> positions;
    std::ranges::transform(sixPos, std::back_inserter(positions), [](uint16_t pwm) { return bay(pwm); });
    QCOMPARE(positions, (QList<int>{1, 2, 3, 4, 5, 6}));
    QCOMPARE(bayOf(lineWith({{kR2d2.bay, 1398}})), 3);
    QCOMPARE(bayOf(lineWith({{kR2d2.bay, 1398}}), without(kR2d2, kR2d2.bay)), 0);
    const QList<BayServo> servos{{5, 1900}, {6, 1900}, {0, 1900}, {8, 1950}};
    QCOMPARE(bayServo(2, servos), (std::optional<BayServo>(BayServo{6, 1900})));
    QCOMPARE(bayServo(4, servos), (std::optional<BayServo>(BayServo{8, 1950})));
    QVERIFY2(!bayServo(3, servos), "a bay with output 0 has no servo");
    QVERIFY2(!bayServo(5, servos), "the 6POS has more positions than the drone has bays");
    QVERIFY(!bayServo(1, {}));
}
