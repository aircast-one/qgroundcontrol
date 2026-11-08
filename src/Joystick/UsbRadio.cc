#include "UsbRadio.h"

#include <QtCore/QApplicationStatic>
#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QStorageInfo>
#include <QtSerialPort/QSerialPort>
#include <algorithm>
#include <cmath>
#include <numeric>
#include <ranges>

#include "AutoConnectSettings.h"
#include "Fact.h"
#include "FirmwarePlugin.h"
#include "FlyViewSettings.h"
#include "GimbalController.h"
#include "JoystickManager.h"
#include "LinkConfiguration.h"
#include "LinkInterface.h"
#include "MAVLinkProtocol.h"
#include "MavlinkCameraControlInterface.h"
#include "MultiVehicleManager.h"
#include "ParameterManager.h"
#include "QGCApplication.h"
#include "QGCCameraManager.h"
#include "QGCLoggingCategory.h"
#include "QGCMAVLink.h"
#include "QGCSerialPortInfo.h"
#include "SettingsManager.h"
#include "Vehicle.h"
#include "VehicleLinkManager.h"

QGC_LOGGING_CATEGORY(UsbRadioLog, "Joystick.UsbRadio")

Q_APPLICATION_STATIC(UsbRadio, _usbRadio);

namespace UsbRadioProtocol {

uint16_t pwm(int raw)
{
    return static_cast<uint16_t>(std::clamp(raw / 2 + 1500, kOverrideMin, kOverrideMax));
}

namespace {

std::optional<QList<int>> values(const QByteArray& line)
{
    const QByteArray trimmed = line.trimmed();
    if (!trimmed.startsWith('$')) {
        return std::nullopt;
    }
    const QList<QByteArray> fields = trimmed.mid(1).split(',');
    const QList<QByteArray> present = std::accumulate(
        fields.cbegin(), fields.cend(), QList<QByteArray>{}, [](QList<QByteArray> kept, const QByteArray& field) {
            return field.trimmed().isEmpty() ? kept : (kept << field.trimmed());
        });
    const bool numeric = std::ranges::all_of(present, [](const QByteArray& field) {
        bool ok = false;
        (void) field.toInt(&ok);
        return ok;
    });
    if (!numeric || present.size() < kMinValues || present.size() > kMaxValues) {
        return std::nullopt;
    }
    QList<int> read;
    std::ranges::transform(present, std::back_inserter(read), [](const QByteArray& field) { return field.toInt(); });
    return read;
}

Line lineOf(const QList<int>& read)
{
    Line line{};
    std::ranges::transform(std::views::iota(0, kMaxValues), line.begin(), [&read](int at) {
        return at < read.size() ? pwm(read[at]) : static_cast<uint16_t>(UINT16_MAX);
    });
    return line;
}

std::optional<uint16_t> channel(const Line& line, int number)
{
    if (number < 1 || number > static_cast<int>(line.size())) {
        return std::nullopt;
    }
    const uint16_t value = line[number - 1];
    return value == UINT16_MAX ? std::nullopt : std::optional<uint16_t>(value);
}

int position(uint16_t pwm)
{
    return pwm < kSwitchLowPwm ? 0 : pwm <= kSwitchHighPwm ? 1 : 2;
}

}  // namespace

std::optional<Line> parse(const QByteArray& line)
{
    const std::optional<QList<int>> read = values(line);
    return read ? std::optional<Line>(lineOf(*read)) : std::nullopt;
}

Frame flightChannels(const Line& line)
{
    Frame frame{};
    std::copy_n(line.begin(), kChannels, frame.begin());
    return frame;
}

float between(float low, float high, uint16_t pwm)
{
    const float share = std::clamp((static_cast<float>(pwm) - 988.0f) / 1024.0f, 0.0f, 1.0f);
    return low + share * (high - low);
}

Layout without(const Layout& layout, int channel)
{
    const auto off = [channel](int at) { return channel > 0 && at == channel ? 0 : at; };
    return Layout{off(layout.pitch), off(layout.yaw), off(layout.zoom), off(layout.picture), off(layout.palette),
                  off(layout.drop),  off(layout.arm), off(layout.bay),  off(layout.tilt),    off(layout.look)};
}

std::optional<Switches> switches(const Line& line, const Layout& layout)
{
    const std::optional<uint16_t> arm = channel(line, layout.arm);
    return arm ? std::optional<Switches>(Switches{*arm >= kArmedPwm}) : std::nullopt;
}

int bay(uint16_t pwm)
{
    return static_cast<int>(std::lround(between(0.0f, kBayPositions - 1.0f, pwm))) + 1;
}

int bayOf(const Line& line, const Layout& layout)
{
    const std::optional<uint16_t> pwm = channel(line, layout.bay);
    return pwm ? bay(*pwm) : 0;
}

std::optional<BayServo> bayServo(int bay, const QList<BayServo>& servos)
{
    return bay >= 1 && bay <= servos.size() && servos[bay - 1].output > 0 ? std::optional<BayServo>(servos[bay - 1])
                                                                          : std::nullopt;
}

Payload payload(std::optional<Switches> switches, bool flown, bool gripper)
{
    return !switches          ? Payload::Off
           : !switches->armed ? Payload::Safe
           : flown && gripper ? Payload::Armed
                              : Payload::Refused;
}

QByteArray payloadName(Payload payload)
{
    switch (payload) {
        case Payload::Armed:
            return QByteArrayLiteral("armed");
        case Payload::Safe:
            return QByteArrayLiteral("safe");
        case Payload::Refused:
            return QByteArrayLiteral("refused");
        case Payload::Off:
        default:
            return QByteArrayLiteral("off");
    }
}

QList<Move> CameraSticks::step(const Line& line, qint64 nowMs, const Layout& layout, Ready ready)
{
    const bool due = !_sentMs || (nowMs - *_sentMs) >= kCameraResendMs;
    const auto delta = [this, &line](int number) {
        const std::optional<uint16_t> now = channel(line, number);
        return now && _acted.contains(number) ? std::abs(int(*now) - int(_acted[number])) : 0;
    };
    const auto moved = [&delta](int number) { return delta(number) >= kSteadyPwm; };
    const auto pressed = [this, &line](int number) {
        const std::optional<uint16_t> now = channel(line, number);
        return now && *now > kPressedPwm && _acted.contains(number) && _acted[number] <= kPressedPwm;
    };
    const auto flipped = [this, &line](int number) {
        const std::optional<uint16_t> now = channel(line, number);
        return now && _acted.contains(number) && position(_acted[number]) != position(*now);
    };
    const bool remembered = _pitchSource && (*_pitchSource == layout.pitch || *_pitchSource == layout.tilt);
    const int source = moved(layout.tilt) && delta(layout.tilt) > delta(layout.pitch) ? layout.tilt
                       : moved(layout.pitch)                                          ? layout.pitch
                       : remembered                                                   ? *_pitchSource
                       : layout.pitch > 0                                             ? layout.pitch
                                                                                      : layout.tilt;
    const std::optional<uint16_t> pitch = channel(line, source);
    const std::optional<uint16_t> yaw = channel(line, layout.yaw);
    const std::optional<uint16_t> zoom = channel(line, layout.zoom);
    const std::optional<uint16_t> picture = channel(line, layout.picture);
    const std::optional<uint16_t> look = channel(line, layout.look);
    const bool aimed =
        ready.gimbal && pitch && yaw && due && (moved(layout.pitch) || moved(layout.tilt) || moved(layout.yaw));
    const bool looked = ready.gimbal && look && flipped(layout.look) && position(*look) != 1;
    const bool zoomed = ready.camera && zoom && due && moved(layout.zoom);
    const bool switched = ready.settings && flipped(layout.picture);
    const bool palette = ready.settings && pressed(layout.palette);
    const auto high = [](std::optional<uint16_t> pwm) { return pwm && *pwm > kPressedPwm; };
    const std::optional<uint16_t> drop = channel(line, layout.drop);
    const std::optional<uint16_t> dropWas =
        _acted.contains(layout.drop) ? std::optional<uint16_t>(_acted[layout.drop]) : std::nullopt;
    const bool dropped = high(drop) && high(dropWas) && _dropBefore && *_dropBefore <= kPressedPwm;
    const auto was = [this](int number) {
        return _acted.contains(number) ? std::optional<uint16_t>(_acted[number]) : std::nullopt;
    };
    const auto armedPwm = [](std::optional<uint16_t> pwm) { return pwm && *pwm >= kArmedPwm; };
    const std::optional<uint16_t> arm = channel(line, layout.arm);
    const std::optional<uint16_t> armWas = was(layout.arm);
    const std::optional<uint16_t> bayNow = channel(line, layout.bay);
    const std::optional<uint16_t> bayWas = was(layout.bay);
    const bool pressedBay = bayNow && bayWas && _bayBefore && armedPwm(arm) && armedPwm(armWas) &&
                            armedPwm(_armBefore) && bay(*bayWas) == bay(*bayNow) && bay(*_bayBefore) != bay(*bayNow);
    const auto refresh = [this, &line, &layout, &ready, aimed, zoomed](int number) {
        const std::optional<uint16_t> now = channel(line, number);
        const bool keep =
            !_acted.contains(number) || (number == layout.pitch || number == layout.yaw || number == layout.tilt ? aimed
                                         : number == layout.zoom                                ? zoomed
                                         : number == layout.picture || number == layout.palette ? ready.settings
                                         : number == layout.look                                ? ready.gimbal
                                                                                                : true);
        if (now && keep) {
            _acted[number] = *now;
        }
    };
    const bool pitched = moved(layout.pitch) || moved(layout.tilt);
    QList<Move> moves;
    if (aimed) {
        const float aimPitch = pitched || !_heldPitch ? between(kPitchDown, kPitchAhead, *pitch) : *_heldPitch;
        moves << Move{Move::Kind::Aim, aimPitch, between(-kYawLimit, kYawLimit, *yaw)};
        _pitchSource = source;
        _heldPitch = aimPitch;
    }
    if (looked) {
        const float lookPitch = position(*look) == 2 ? kPitchDown : kPitchAhead;
        moves << Move{Move::Kind::Aim, lookPitch, yaw ? between(-kYawLimit, kYawLimit, *yaw) : 0.0f};
        _heldPitch = lookPitch;
    }
    if (zoomed) {
        moves << Move{Move::Kind::Zoom, 0.0f, 0.0f, between(0.0f, 100.0f, *zoom)};
    }
    if (switched) {
        moves << Move{Move::Kind::Picture, 0.0f, 0.0f, 0.0f, kPictures[position(*picture)]};
    }
    if (palette) {
        moves << Move{Move::Kind::NextPalette};
    }
    if (dropped && !pressedBay) {
        moves << Move{Move::Kind::Drop};
    }
    if (pressedBay) {
        moves << Move{Move::Kind::DropBay, 0.0f, 0.0f, 0.0f, bay(*bayNow)};
    }
    _dropBefore = drop ? dropWas : _dropBefore;
    _bayBefore = bayNow ? bayWas : _bayBefore;
    _armBefore = arm ? armWas : _armBefore;
    std::ranges::for_each(std::array{layout.drop, layout.arm, layout.bay, layout.pitch, layout.yaw, layout.zoom,
                                     layout.picture, layout.palette, layout.tilt, layout.look},
                          refresh);
    if (aimed || zoomed) {
        _sentMs = nowMs;
    }
    return moves;
}

Frame released()
{
    Frame frame{};
    std::ranges::transform(std::views::iota(1, kChannels + 1), frame.begin(), [](int channel) {
        return channel <= kLowChannels ? static_cast<uint16_t>(0) : static_cast<uint16_t>(UINT16_MAX - 1);
    });
    return frame;
}

namespace {

QByteArray layoutTail(const Layout& layout)
{
    const std::array channels{layout.pitch, layout.yaw, layout.zoom, layout.picture, layout.palette,
                              layout.drop,  layout.arm, layout.bay,  layout.tilt,    layout.look};
    return std::accumulate(channels.cbegin(), channels.cend(), QByteArray(";"), [](const QByteArray& tail, int number) {
        return tail + (tail.size() > 1 ? "," : "") + QByteArray::number(number);
    });
}

QByteArray modesTail(qsizetype room, const QStringList& modes)
{
    const QStringList shortened = std::accumulate(
        modes.cbegin(), modes.cend(), QStringList{},
        [](QStringList kept, const QString& mode) { return kept << QString(mode).remove(' ').left(kModeNameChars); });
    const auto joined = [&shortened](int count) { return shortened.mid(0, count).join(',').toLatin1(); };
    const auto counts = std::views::iota(0, static_cast<int>(shortened.size()) + 1) | std::views::reverse;
    const auto fits = [&joined, room](int count) { return joined(count).size() <= std::max<qsizetype>(room, 0); };
    return joined(*std::ranges::find_if(counts, fits));
}

}  // namespace

QByteArray echo(Flying flying, int vehicle, Kind kind, Sticks sticks, const Layout& layout, int modeChannel,
                Payload payload, const QStringList& modes)
{
    const char state = flying == Flying::Vehicle    ? 'F'
                       : flying == Flying::Standby  ? 'S'
                       : flying == Flying::Joystick ? 'J'
                                                    : 'N';
    const QByteArray head = QByteArrayLiteral("@") + state + ',' + QByteArray::number(vehicle);
    const char letter = kind == Kind::Copter ? 'C' : kind == Kind::Plane ? 'P' : kind == Kind::Rover ? 'R' : '\0';
    const QByteArray flight = state == 'N' ? QByteArrayLiteral("@N") + layoutTail(layout)
                              : letter == '\0'
                                  ? head + layoutTail(layout)
                                  : head + ',' + letter + ',' + QByteArray::number(sticks.roll) + ',' +
                                        QByteArray::number(sticks.pitch) + ',' + QByteArray::number(sticks.throttle) +
                                        ',' + QByteArray::number(sticks.yaw) + ',' + QByteArray::number(modeChannel) +
                                        layoutTail(without(layout, modeChannel));
    const QByteArray body = flight + '|' + payloadName(payload) + '|';
    return body + modesTail(kMaxEchoBytes - 1 - body.size() - 1, modes) + '\n';
}

QStringList namedOn(const Layout& layout, int channel)
{
    const std::array<std::pair<int, const char*>, 10> functions{{{layout.pitch, "camera pitch"},
                                                                 {layout.yaw, "camera yaw"},
                                                                 {layout.zoom, "zoom"},
                                                                 {layout.picture, "picture"},
                                                                 {layout.palette, "palette"},
                                                                 {layout.drop, "drop"},
                                                                 {layout.arm, "arm"},
                                                                 {layout.bay, "bay"},
                                                                 {layout.tilt, "tilt"},
                                                                 {layout.look, "look"}}};
    return std::accumulate(
        functions.cbegin(), functions.cend(), QStringList{},
        [channel](QStringList names, const std::pair<int, const char*>& function) {
            return channel > 0 && function.first == channel ? names << QString::fromLatin1(function.second) : names;
        });
}

QString clashText(int channel, const QStringList& names)
{
    return QCoreApplication::translate("UsbRadio",
                                       "The drone reads flight modes on CH%1, so the radio's %2 switch is off. Move "
                                       "flight modes or the switch to another channel.")
        .arg(channel)
        .arg(names.join(QStringLiteral(" and ")));
}

Kind kind(int mavType)
{
    const MAV_TYPE type = static_cast<MAV_TYPE>(mavType);
    const QGCMAVLink::VehicleClass_t vehicleClass = QGCMAVLink::vehicleClass(type);
    const bool copter = vehicleClass == QGCMAVLink::VehicleClassMultiRotor ||
                        vehicleClass == QGCMAVLink::VehicleClassSub || type == MAV_TYPE_DECAROTOR ||
                        type == MAV_TYPE_DODECAROTOR;
    const bool plane =
        vehicleClass == QGCMAVLink::VehicleClassFixedWing || vehicleClass == QGCMAVLink::VehicleClassVTOL;
    return copter                                              ? Kind::Copter
           : plane                                             ? Kind::Plane
           : vehicleClass == QGCMAVLink::VehicleClassRoverBoat ? Kind::Rover
                                                               : Kind::Unknown;
}

Sticks sticks(int roll, int pitch, int throttle, int yaw)
{
    const Sticks fallback;
    const auto pick = [](int mapped, int otherwise) { return mapped > 0 ? mapped : otherwise; };
    return Sticks{pick(roll, fallback.roll), pick(pitch, fallback.pitch), pick(throttle, fallback.throttle),
                  pick(yaw, fallback.yaw)};
}

bool echoDue(std::optional<qint64> last, qint64 nowMs, bool live)
{
    return live && (!last || (nowMs - *last) >= kEchoEveryMs);
}

uint8_t component(bool standby)
{
    return standby ? kStandbyComponent : kAutopilotComponent;
}

bool standbyAirframe(bool ardupilot, int mavType)
{
    static constexpr std::array<int, 8> kMulticopters = {MAV_TYPE_QUADROTOR,   MAV_TYPE_COAXIAL,   MAV_TYPE_HELICOPTER,
                                                         MAV_TYPE_HEXAROTOR,   MAV_TYPE_OCTOROTOR, MAV_TYPE_TRICOPTER,
                                                         MAV_TYPE_DODECAROTOR, MAV_TYPE_DECAROTOR};
    return ardupilot && std::ranges::find(kMulticopters, mavType) != kMulticopters.end();
}

QList<Line> Lines::feed(const QByteArray& bytes)
{
    _pending += bytes;
    const qsizetype end = _pending.lastIndexOf('\n');
    const QByteArray complete = end < 0 ? QByteArray() : _pending.left(end + 1);
    _pending = end < 0 ? _pending : _pending.mid(end + 1);
    if (_pending.size() > kMaxLineBytes) {
        _pending.clear();
    }
    const QList<QByteArray> lines = complete.split('\n');
    return std::accumulate(lines.cbegin(), lines.cend(), QList<Line>{},
                           [this](QList<Line> frames, const QByteArray& line) {
                               const std::optional<QList<int>> read = values(line);
                               if (!read) {
                                   return frames;
                               }
                               const bool steady = !_width || *_width == read->size();
                               _width = read->size();
                               return steady ? (frames << lineOf(*read)) : frames;
                           });
}

QString busyText(const QString& port)
{
    return QCoreApplication::translate("UsbRadio",
                                       "%1 is open in another app: Chrome with the Aircast web app, or another "
                                       "QGroundControl. Only one app can hold the radio. Close it there and the radio "
                                       "connects by itself.")
        .arg(port);
}

QString silentText(const QString& port, qint64 silentMs)
{
    return silentMs < kSilentHelpAfterMs
               ? QCoreApplication::translate("UsbRadio", "Waiting for the radio on %1").arg(port)
               : QCoreApplication::translate("UsbRadio",
                                             "No sticks from the radio on %1. On the radio open SYS → Tools → Aircast "
                                             "USB radio, and choose USB Serial (VCP) when you plug it in.")
                     .arg(port);
}

QString flightText(Flying flying, int vehicle)
{
    return flying == Flying::Vehicle ? QCoreApplication::translate("UsbRadio", "Flying vehicle %1").arg(vehicle)
           : flying == Flying::Standby
               ? QCoreApplication::translate("UsbRadio", "Network standby for vehicle %1").arg(vehicle)
           : flying == Flying::Joystick
               ? QCoreApplication::translate("UsbRadio", "Receiving; a joystick is flying vehicle %1").arg(vehicle)
               : QCoreApplication::translate("UsbRadio", "Receiving; no vehicle connected");
}

QString toolsFolder(const QStringList& roots)
{
    const auto card = std::ranges::find_if(
        roots, [](const QString& root) { return QDir(root).exists(QStringLiteral("SCRIPTS/TOOLS")); });
    return card == roots.end() ? QString() : QDir(*card).filePath(QStringLiteral("SCRIPTS/TOOLS"));
}

QString saveScript(const QString& folder)
{
    const QDir dir(folder);
    const QString target = dir.filePath(QStringLiteral("aircast-usb.lua"));
    (void) QFile::remove(target);
    (void) QFile::remove(dir.filePath(QStringLiteral("aircast-usb.luac")));
    if (!QFile::copy(QStringLiteral(":/edgetx/aircast-usb.lua"), target)) {
        return QCoreApplication::translate("UsbRadio", "Could not write %1").arg(target);
    }
    (void) QFile::setPermissions(
        target, QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ReadGroup | QFileDevice::ReadOther);
    return QString();
}

State state(bool on, bool live, Flying flying)
{
    return !on                          ? State::Off
           : !live                      ? State::Stopped
           : flying == Flying::Vehicle  ? State::Live
           : flying == Flying::Standby  ? State::Standby
           : flying == Flying::Joystick ? State::Paused
                                        : State::Ready;
}

QString stateName(State state)
{
    switch (state) {
        case State::Live:
            return QStringLiteral("live");
        case State::Ready:
            return QStringLiteral("ready");
        case State::Paused:
            return QStringLiteral("paused");
        case State::Standby:
            return QStringLiteral("standby");
        case State::Stopped:
            return QStringLiteral("stopped");
        case State::Off:
        default:
            return QStringLiteral("off");
    }
}

Verdict Sniffer::feed(const QByteArray& bytes)
{
    _seen += bytes.size();
    _frames += static_cast<int>(_lines.feed(bytes).size());
    return _frames >= kRadioFrames ? Verdict::Radio : (_seen >= kSniffBytes ? Verdict::NotRadio : Verdict::Undecided);
}

std::optional<qint64> Radio::heardAt() const
{
    return _latest ? std::optional<qint64>(_latest->second) : std::nullopt;
}

bool Radio::live(qint64 nowMs) const
{
    return _latest && (nowMs - _latest->second) <= kStaleAfterMs;
}

Step Radio::step(qint64 nowMs, std::optional<int> target, std::optional<Frame> fresh)
{
    if (fresh) {
        _latest = std::make_pair(*fresh, nowMs);
    }
    const int flying = live(nowMs) ? target.value_or(0) : 0;
    const int holding = _holding.value_or(0);
    const std::optional<int> handedBack =
        (holding != 0 && flying != holding) ? std::optional<int>(holding) : std::nullopt;
    const bool due = fresh && (!_sentAt || (nowMs - *_sentAt) >= kSendEveryMs);
    const std::optional<Send> send =
        (flying != 0 && _latest && due) ? std::optional<Send>(Send{flying, _latest->first}) : std::nullopt;
    if (send) {
        _sentAt = nowMs;
    }
    _holding = send ? std::optional<int>(send->vehicle) : (handedBack ? std::nullopt : _holding);
    const QList<Send> releases =
        handedBack ? QList<Send>(kReleaseFrames, Send{*handedBack, released()}) : QList<Send>{};
    return Step{send ? (releases + QList<Send>{*send}) : releases, handedBack};
}

}  // namespace UsbRadioProtocol

using namespace UsbRadioProtocol;

namespace {

QString portLabel(const QGCSerialPortInfo& info)
{
    const QString product = info.description().trimmed();
    return product.isEmpty() ? info.portName() : QStringLiteral("%1 (%2)").arg(product, info.portName());
}

}  // namespace

UsbRadio::UsbRadio(QObject* parent) : QObject(parent)
{
    _clock.start();
    _timer.setInterval(50);
    (void) connect(&_timer, &QTimer::timeout, this, &UsbRadio::_maintain);
}

UsbRadio::~UsbRadio()
{
    _close();
}

UsbRadio* UsbRadio::instance()
{
    return _usbRadio();
}

void UsbRadio::init()
{
    _timer.start();
    _maintain();
}

QString UsbRadio::scriptFolder() const
{
    const QList<QStorageInfo> volumes = QStorageInfo::mountedVolumes();
    QStringList roots(volumes.size());
    std::ranges::transform(volumes, roots.begin(), &QStorageInfo::rootPath);
    return toolsFolder(roots);
}

QString UsbRadio::saveScript(const QString& folder) const
{
    return UsbRadioProtocol::saveScript(folder);
}

QVariantList UsbRadio::ports() const
{
    const QList<QGCSerialPortInfo> available = QGCSerialPortInfo::availablePorts();
    const QVariantList listed = std::accumulate(
        available.cbegin(), available.cend(), QVariantList{}, [](QVariantList rows, const QGCSerialPortInfo& info) {
            return rows << QVariantMap{{QStringLiteral("port"), info.systemLocation().trimmed()},
                                       {QStringLiteral("label"), portLabel(info)}};
        });
    const QString current = _wanted();
    const bool missing = !current.isEmpty() && std::ranges::none_of(listed, [&current](const QVariant& row) {
        return row.toMap().value(QStringLiteral("port")).toString() == current;
    });
    const QVariantList off{QVariantMap{{QStringLiteral("port"), QString()}, {QStringLiteral("label"), tr("Off")}}};
    const QVariantList gone =
        missing ? QVariantList{QVariantMap{{QStringLiteral("port"), current},
                                           {QStringLiteral("label"), tr("%1 (not plugged in)").arg(current)}}}
                : QVariantList{};
    return off + listed + gone;
}

bool UsbRadio::isRadioPort(const QString& port) const
{
    const QString trimmed = port.trimmed();
    return !trimmed.isEmpty() && (trimmed == _wanted() || _radioPorts.contains(trimmed));
}

Verdict UsbRadio::sniff(const LinkInterface* link, const QByteArray& bytes)
{
    const Verdict verdict = _sniffers[link].feed(bytes);
    if (verdict != Verdict::Undecided) {
        (void) _sniffers.remove(link);
    }
    return verdict;
}

void UsbRadio::forgetLink(const LinkInterface* link)
{
    (void) _sniffers.remove(link);
}

void UsbRadio::radioSeen(const QString& port)
{
    const QString trimmed = port.trimmed();
    if (trimmed.isEmpty() || _radioPorts.contains(trimmed)) {
        return;
    }
    _radioPorts.insert(trimmed);
    qCInfo(UsbRadioLog) << "A radio is streaming its sticks on" << trimmed;
    if (!_wanted().isEmpty()) {
        return;
    }
    const QList<QGCSerialPortInfo> available = QGCSerialPortInfo::availablePorts();
    const auto found = std::ranges::find_if(
        available, [&trimmed](const QGCSerialPortInfo& info) { return info.systemLocation().trimmed() == trimmed; });
    _setOffer(trimmed, found == available.end() ? trimmed : portLabel(*found));
}

void UsbRadio::portsPresent(const QStringList& ports)
{
    const QSet<QString> present =
        std::accumulate(ports.cbegin(), ports.cend(), QSet<QString>{}, [](QSet<QString> kept, const QString& port) {
            kept.insert(port.trimmed());
            return kept;
        });
    _radioPorts.intersect(present);
    if (!_offerPort.isEmpty() && !present.contains(_offerPort)) {
        _setOffer(QString(), QString());
    }
}

void UsbRadio::useOffer(const QString& port)
{
    if (port.trimmed().isEmpty()) {
        return;
    }
    SettingsManager::instance()->autoConnectSettings()->usbRadioPort()->setRawValue(port.trimmed());
    _setOffer(QString(), QString());
}

void UsbRadio::dismissOffer(const QString& port)
{
    if (port.trimmed() == _offerPort) {
        _setOffer(QString(), QString());
    }
}

void UsbRadio::_setOffer(const QString& port, const QString& label)
{
    if (port != _offerPort || label != _offerLabel) {
        _offerPort = port;
        _offerLabel = label;
        emit offerChanged();
    }
}

void UsbRadio::_setState(const QString& state)
{
    if (state != _state) {
        _state = state;
        emit stateChanged();
    }
}

UsbRadioProtocol::Layout UsbRadio::_layout() const
{
    AutoConnectSettings* const settings = SettingsManager::instance()->autoConnectSettings();
    const auto number = [](Fact* fact) { return fact->rawValue().toInt(); };
    return UsbRadioProtocol::Layout{
        number(settings->usbRadioPitchChannel()),   number(settings->usbRadioYawChannel()),
        number(settings->usbRadioZoomChannel()),    number(settings->usbRadioPictureChannel()),
        number(settings->usbRadioPaletteChannel()), number(settings->usbRadioDropChannel()),
        number(settings->usbRadioArmChannel()),     number(settings->usbRadioBayChannel()),
        number(settings->usbRadioTiltChannel()),    number(settings->usbRadioLookChannel())};
}

QString UsbRadio::_wanted() const
{
    return SettingsManager::instance()->autoConnectSettings()->usbRadioPort()->rawValue().toString().trimmed();
}

void UsbRadio::_maintain()
{
    const QString wanted = _wanted();
    const qint64 now = _clock.elapsed();
    const bool alive = _port && _port->isOpen() && _port->error() == QSerialPort::NoError;
    const bool settled = (wanted == _portName) && (alive || (_openedAt && (now - *_openedAt) < kReopenAfterMs));
    if (!settled && !(wanted.isEmpty() && _portName.isEmpty())) {
        _close();
        if (!wanted.isEmpty()) {
            _open(wanted);
        }
    }
    _advance(std::nullopt);
}

void UsbRadio::_open(const QString& port)
{
    _portName = port;
    _openedAt = _clock.elapsed();
    _port = new QSerialPort(port, this);
    _port->setBaudRate(kBaud);
    (void) connect(_port, &QSerialPort::readyRead, this, &UsbRadio::_read);
    if (!_port->open(QIODevice::ReadWrite)) {
        qCDebug(UsbRadioLog) << "Cannot open" << port << _port->errorString();
        _setStatus(_port->error() == QSerialPort::PermissionError
                       ? busyText(port)
                       : tr("Cannot open %1: %2").arg(port, _port->errorString()));
        return;
    }
    qCInfo(UsbRadioLog) << "Listening for the radio on" << port;
}

void UsbRadio::_close()
{
    if (_port) {
        _port->close();
        _port->deleteLater();
        _port = nullptr;
    }
    _portName.clear();
    _openedAt.reset();
    _lines = Lines{};
    _line.reset();
}

void UsbRadio::_read()
{
    const QList<Line> lines = _lines.feed(_port->readAll());
    if (!lines.isEmpty()) {
        _line = lines.last();
        _advance(flightChannels(lines.last()));
        _steer(lines.last());
    }
}

void UsbRadio::_steer(const Line& line)
{
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    if (!vehicle || !_target()) {
        return;
    }
    const Flight flight = _flight();
    const Layout settings = _layout();
    _warnOfClash(flight.vehicle, settings, flight.modeChannel);
    const Layout layout = without(settings, flight.modeChannel);
    const std::optional<Switches> armed = switches(line, layout);
    MavlinkCameraControlInterface* const camera =
        vehicle->cameraManager() ? vehicle->cameraManager()->currentCameraInstance() : nullptr;
    const Ready ready{vehicle->gimbalController() && vehicle->gimbalController()->activeGimbal(), camera != nullptr,
                      camera && camera->paramComplete()};
    const QList<Move> moves = _camera.step(line, _clock.elapsed(), layout, ready);
    const int selectedBay = bayOf(line, layout);
    std::ranges::for_each(moves, [this, vehicle, &armed, selectedBay](const Move& move) {
        _apply(vehicle, move, armed && armed->armed, selectedBay);
    });
}

void UsbRadio::_warnOfClash(int vehicle, const Layout& layout, int modeChannel)
{
    const QStringList names = namedOn(layout, modeChannel);
    const std::optional<std::pair<int, int>> clash =
        names.isEmpty() ? std::nullopt : std::optional<std::pair<int, int>>(std::pair{vehicle, modeChannel});
    const bool fresh = clash && clash != _clash;
    _clash = clash;
    if (fresh) {
        qgcApp()->showAppMessage(clashText(modeChannel, names));
    }
}

void UsbRadio::_apply(Vehicle* vehicle, const Move& move, bool armed, int selectedBay) const
{
    MavlinkCameraControlInterface* const camera =
        vehicle->cameraManager() ? vehicle->cameraManager()->currentCameraInstance() : nullptr;
    const auto setting = [camera](const char* name) -> Fact* {
        const QString id = QLatin1String(name);
        return camera && camera->paramComplete() && camera->factExists(id) ? camera->getFact(id) : nullptr;
    };
    switch (move.kind) {
        case Move::Kind::Aim:
            if (vehicle->gimbalController()) {
                vehicle->gimbalController()->sendPitchBodyYaw(move.pitch, move.yaw, false);
            }
            return;
        case Move::Kind::Zoom:
            if (camera) {
                camera->setZoomLevel(move.level);
            }
            return;
        case Move::Kind::Picture:
            if (Fact* const picture = setting("IR_PIP")) {
                picture->setRawValue(move.value);
            }
            return;
        case Move::Kind::NextPalette:
            if (Fact* const palette = setting("IR_PALETTE")) {
                const QVariantList offered = palette->enumValues();
                const qsizetype at = offered.indexOf(palette->rawValue());
                if (!offered.isEmpty()) {
                    palette->setRawValue(offered[(at + 1) % offered.size()]);
                }
            }
            return;
        case Move::Kind::Drop:
            if (!armed) {
                qCInfo(UsbRadioLog) << "The drop switch was flipped while BOMARM is safe; nothing dropped";
                return;
            }
            _release(vehicle, selectedBay, false);
            return;
        case Move::Kind::DropBay:
            if (armed) {
                _release(vehicle, move.value, true);
            }
            return;
    }
}

void UsbRadio::_release(Vehicle* vehicle, int bay, bool pressed) const
{
    const QList<BayServo> servos = _bayServos(vehicle);
    if (const std::optional<BayServo> servo = bayServo(bay, servos)) {
        vehicle->sendMavCommand(vehicle->defaultComponentId(), MAV_CMD_DO_SET_SERVO, true, servo->output,
                                servo->releasePwm);
        return;
    }
    if (!servos.isEmpty() || pressed) {
        qCInfo(UsbRadioLog) << "The drone has no servo for bay" << bay << "; nothing dropped";
        return;
    }
    if (vehicle->hasGripper()) {
        vehicle->sendGripperAction(GRIPPER_ACTION_RELEASE);
    }
}

QList<BayServo> UsbRadio::_bayServos(const Vehicle* vehicle) const
{
    FlyViewSettings* const settings = SettingsManager::instance()->flyViewSettings();
    if (!vehicle->apmFirmware() || settings->payloadRelease()->rawValue().toInt() != 1) {
        return {};
    }
    const auto fact = [settings](int bay, const char* part) {
        return settings->property(qPrintable(QStringLiteral("payloadBay%1%2").arg(bay).arg(QLatin1String(part))))
            .value<Fact*>()
            ->rawValue()
            .toInt();
    };
    QList<BayServo> servos;
    std::ranges::transform(std::views::iota(1, settings->payloadBays()->rawValue().toInt() + 1),
                           std::back_inserter(servos),
                           [&fact](int bay) { return BayServo{fact(bay, "Output"), fact(bay, "ReleasePwm")}; });
    return servos;
}

bool UsbRadio::_standby(int vehicleId) const
{
    const Vehicle* const vehicle = MultiVehicleManager::instance()->getVehicleById(vehicleId);
    if (!vehicle) {
        return false;
    }
    return SettingsManager::instance()->autoConnectSettings()->usbRadioStandby()->rawValue().toBool() &&
           standbyAirframe(vehicle->apmFirmware(), vehicle->vehicleType());
}

UsbRadio::Flight UsbRadio::_flight() const
{
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    if (!vehicle) {
        return Flight{};
    }
    const SharedLinkInterfacePtr link = vehicle->vehicleLinkManager()->primaryLink().lock();
    if (!link || link->linkConfiguration()->isHighLatency()) {
        return Flight{};
    }
    const bool joystick = JoystickManager::instance()->activeJoystick() &&
                          JoystickManager::instance()->activeJoystickEnabledForActiveVehicle();
    const Flying flying = joystick ? Flying::Joystick : (_standby(vehicle->id()) ? Flying::Standby : Flying::Vehicle);
    ParameterManager* const parameters = vehicle->parameterManager();
    const bool px4 = vehicle->px4Firmware();
    const QString prefix = px4 ? QStringLiteral("RC_MAP_") : QStringLiteral("RCMAP_");
    const auto parameter = [parameters](const QString& name) {
        return parameters && parameters->parameterExists(ParameterManager::defaultComponentId, name)
                   ? parameters->getParameter(ParameterManager::defaultComponentId, name)->rawValue().toInt()
                   : 0;
    };
    const auto mapped = [&parameter, &prefix](const char* axis) { return parameter(prefix + QLatin1String(axis)); };
    const int modeChannel =
        px4 ? parameter(QStringLiteral("RC_MAP_FLTMODE"))
            : std::max(parameter(QStringLiteral("FLTMODE_CH")), parameter(QStringLiteral("MODE_CH")));
    return Flight{flying, vehicle->id(), kind(vehicle->vehicleType()),
                  sticks(mapped("ROLL"), mapped("PITCH"), mapped("THROTTLE"), mapped("YAW")), modeChannel};
}

std::optional<int> UsbRadio::_target() const
{
    const Flight flight = _flight();
    return (flight.flying == Flying::Vehicle || flight.flying == Flying::Standby) ? std::optional<int>(flight.vehicle)
                                                                                  : std::nullopt;
}

Payload UsbRadio::_payload(const Flight& flight) const
{
    const Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    const std::optional<Switches> armed =
        vehicle && _line ? switches(*_line, without(_layout(), flight.modeChannel)) : std::nullopt;
    return payload(armed, _target().has_value(), vehicle && (vehicle->hasGripper() || !_bayServos(vehicle).isEmpty()));
}

QStringList UsbRadio::_modes() const
{
    Vehicle* const vehicle = MultiVehicleManager::instance()->activeVehicle();
    ParameterManager* const parameters = vehicle ? vehicle->parameterManager() : nullptr;
    const auto exists = [parameters](const QString& name) {
        return parameters && parameters->parameterExists(ParameterManager::defaultComponentId, name);
    };
    if (!vehicle || !vehicle->apmFirmware() ||
        (!exists(QStringLiteral("FLTMODE1")) && !exists(QStringLiteral("MODE1")))) {
        return {};
    }
    const QString pattern = exists(QStringLiteral("MODE1")) ? QStringLiteral("MODE%1") : QStringLiteral("FLTMODE%1");
    QStringList names(kModeSlots);
    std::ranges::transform(std::views::iota(1, kModeSlots + 1), names.begin(), [&](int slot) {
        const QString name = pattern.arg(slot);
        return exists(name)
                   ? vehicle->firmwarePlugin()->flightMode(
                         MAV_MODE_FLAG_CUSTOM_MODE_ENABLED,
                         parameters->getParameter(ParameterManager::defaultComponentId, name)->rawValue().toUInt())
                   : QString();
    });
    return names;
}

void UsbRadio::_send(int vehicleId, bool standby, const Frame& frame) const
{
    Vehicle* const vehicle = MultiVehicleManager::instance()->getVehicleById(vehicleId);
    if (!vehicle) {
        return;
    }
    const SharedLinkInterfacePtr link = vehicle->vehicleLinkManager()->primaryLink().lock();
    if (!link) {
        return;
    }
    mavlink_message_t message;
    (void) mavlink_msg_rc_channels_override_pack_chan(
        static_cast<uint8_t>(MAVLinkProtocol::instance()->getSystemId()),
        static_cast<uint8_t>(MAVLinkProtocol::getComponentId()), link->mavlinkChannel(), &message,
        static_cast<uint8_t>(vehicleId), component(standby), frame[0], frame[1], frame[2], frame[3], frame[4], frame[5],
        frame[6], frame[7], frame[8], frame[9], frame[10], frame[11], frame[12], frame[13], frame[14], frame[15],
        frame[16], frame[17]);
    (void) vehicle->sendMessageOnLinkThreadSafe(link.get(), message);
}

void UsbRadio::_advance(std::optional<Frame> fresh)
{
    const qint64 now = _clock.elapsed();
    const std::optional<int> target = _target();
    const Step step = _radio.step(now, target, fresh);
    const bool live = _radio.live(now);
    const bool standbyNow = target && _standby(*target);
    const bool moved = (_wasStandby != standbyNow) && live && target;
    if (moved) {
        std::ranges::for_each(std::views::iota(0, kReleaseFrames),
                              [this, &target](int) { _send(*target, _wasStandby, released()); });
    }
    _wasStandby = standbyNow;
    std::ranges::for_each(step.sends,
                          [this, standbyNow](const Send& send) { _send(send.vehicle, standbyNow, send.frame); });
    if (step.handedBack) {
        qCInfo(UsbRadioLog) << "Handed RC channels back for vehicle" << *step.handedBack
                            << (live ? "" : "(radio stopped sending)");
    }
    const Flight flight = _flight();
    if (echoDue(_echoedAt, now, live) && _port && _port->isOpen()) {
        _echoedAt = now;
        (void) _port->write(echo(flight.flying, flight.vehicle, flight.kind, flight.sticks, _layout(),
                                 flight.modeChannel, _payload(flight), _modes()));
    }
    const qint64 quietSince = std::max(_openedAt.value_or(now), _radio.heardAt().value_or(0));
    const QString quiet = _portName.isEmpty() ? tr("Off") : silentText(_portName, now - quietSince);
    if (!_port || _port->isOpen()) {
        _setStatus(live ? flightText(flight.flying, flight.vehicle) : quiet);
    }
    _setState(UsbRadioProtocol::stateName(UsbRadioProtocol::state(!_portName.isEmpty(), live, flight.flying)));
}

void UsbRadio::_setStatus(const QString& status)
{
    if (status != _status) {
        _status = status;
        emit statusChanged();
    }
}
