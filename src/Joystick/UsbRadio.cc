#include "UsbRadio.h"

#include <QtCore/QApplicationStatic>
#include <QtSerialPort/QSerialPort>
#include <algorithm>
#include <numeric>
#include <ranges>

#include "AutoConnectSettings.h"
#include "JoystickManager.h"
#include "LinkConfiguration.h"
#include "LinkInterface.h"
#include "MAVLinkProtocol.h"
#include "MultiVehicleManager.h"
#include "QGCLoggingCategory.h"
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

std::optional<Frame> parse(const QByteArray& line)
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
    Frame frame{};
    std::ranges::transform(std::views::iota(0, kChannels), frame.begin(), [&present](int at) {
        return at < present.size() ? pwm(present[at].toInt()) : static_cast<uint16_t>(UINT16_MAX);
    });
    return frame;
}

Frame released()
{
    Frame frame{};
    std::ranges::transform(std::views::iota(1, kChannels + 1), frame.begin(), [](int channel) {
        return channel <= kLowChannels ? static_cast<uint16_t>(0) : static_cast<uint16_t>(UINT16_MAX - 1);
    });
    return frame;
}

QByteArray echo(Flying flying, int vehicle)
{
    switch (flying) {
        case Flying::Vehicle:
            return QByteArrayLiteral("@F,") + QByteArray::number(vehicle) + '\n';
        case Flying::Standby:
            return QByteArrayLiteral("@S,") + QByteArray::number(vehicle) + '\n';
        case Flying::Joystick:
            return QByteArrayLiteral("@J,") + QByteArray::number(vehicle) + '\n';
        case Flying::NoVehicle:
            break;
    }
    return QByteArrayLiteral("@N\n");
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

QList<Frame> Lines::feed(const QByteArray& bytes)
{
    _pending += bytes;
    const qsizetype end = _pending.lastIndexOf('\n');
    const QByteArray complete = end < 0 ? QByteArray() : _pending.left(end + 1);
    _pending = end < 0 ? _pending : _pending.mid(end + 1);
    if (_pending.size() > kMaxLineBytes) {
        _pending.clear();
    }
    const QList<QByteArray> lines = complete.split('\n');
    return std::accumulate(lines.cbegin(), lines.cend(), QList<Frame>{},
                           [](QList<Frame> frames, const QByteArray& line) {
                               const std::optional<Frame> frame = parse(line);
                               return frame ? (frames << *frame) : frames;
                           });
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
    const std::optional<int> flying = live(nowMs) ? target : std::nullopt;
    const std::optional<int> handedBack = (_holding && flying != _holding) ? _holding : std::nullopt;
    const bool due = fresh && (!_sentAt || (nowMs - *_sentAt) >= kSendEveryMs);
    const std::optional<Send> send =
        (flying && _latest && due) ? std::optional<Send>(Send{*flying, _latest->first}) : std::nullopt;
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

QVariantList UsbRadio::ports() const
{
    const QList<QGCSerialPortInfo> available = QGCSerialPortInfo::availablePorts();
    const QVariantList listed = std::accumulate(
        available.cbegin(), available.cend(), QVariantList{}, [](QVariantList rows, const QGCSerialPortInfo& info) {
            const QString port = info.systemLocation().trimmed();
            const QString product = info.description().trimmed();
            const QString label =
                product.isEmpty() ? info.portName() : QStringLiteral("%1 (%2)").arg(product, info.portName());
            return rows << QVariantMap{{QStringLiteral("port"), port}, {QStringLiteral("label"), label}};
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
                       ? tr("%1 is open in another app. Close it there (another QGroundControl, a serial link to it) "
                            "and the radio connects by itself.")
                             .arg(port)
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
}

void UsbRadio::_read()
{
    const QList<Frame> frames = _lines.feed(_port->readAll());
    if (!frames.isEmpty()) {
        _advance(frames.last());
    }
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
    return Flight{flying, vehicle->id()};
}

std::optional<int> UsbRadio::_target() const
{
    const Flight flight = _flight();
    return (flight.flying == Flying::Vehicle || flight.flying == Flying::Standby) ? std::optional<int>(flight.vehicle)
                                                                                  : std::nullopt;
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
        (void) _port->write(echo(flight.flying, flight.vehicle));
    }
    const QString quiet =
        _portName.isEmpty() ? tr("Off") : tr("Waiting for the radio's Aircast USB script on %1").arg(_portName);
    const QString flyingText = flight.flying == Flying::Vehicle   ? tr("Flying vehicle %1").arg(flight.vehicle)
                               : flight.flying == Flying::Standby ? tr("4G standby for vehicle %1").arg(flight.vehicle)
                               : flight.flying == Flying::Joystick
                                   ? tr("Receiving; a joystick is flying vehicle %1").arg(flight.vehicle)
                                   : tr("Receiving; no vehicle connected");
    if (!_port || _port->isOpen()) {
        _setStatus(live ? flyingText : quiet);
    }
}

void UsbRadio::_setStatus(const QString& status)
{
    if (status != _status) {
        _status = status;
        emit statusChanged();
    }
}
