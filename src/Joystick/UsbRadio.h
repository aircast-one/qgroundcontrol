#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QElapsedTimer>
#include <QtCore/QHash>
#include <QtCore/QList>
#include <QtCore/QLoggingCategory>
#include <QtCore/QObject>
#include <QtCore/QSet>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtCore/QVariantList>
#include <QtQmlIntegration/QtQmlIntegration>
#include <array>
#include <optional>

class QSerialPort;
class LinkInterface;
class Vehicle;

Q_DECLARE_LOGGING_CATEGORY(UsbRadioLog)

namespace UsbRadioProtocol {

constexpr int kChannels = 18;
constexpr qint64 kSendEveryMs = 40;
constexpr qint64 kStaleAfterMs = 500;
constexpr qint64 kReopenAfterMs = 1000;
constexpr qint64 kEchoEveryMs = 250;
constexpr int kReleaseFrames = 3;
constexpr qint64 kSilentHelpAfterMs = 1500;
constexpr int kBaud = 115200;
constexpr int kMaxLineBytes = 1024;
constexpr int kMinValues = 4;
constexpr int kMaxValues = 24;
constexpr int kOverrideMin = 800;
constexpr int kOverrideMax = 2200;
constexpr int kLowChannels = 8;
constexpr uint8_t kAutopilotComponent = 1;
constexpr uint8_t kStandbyComponent = 191;
constexpr int kRadioFrames = 3;
constexpr qsizetype kSniffBytes = 16 * 1024;
constexpr int kMaxEchoBytes = 96;
constexpr int kModeSlots = 6;
constexpr int kModeNameChars = 7;

using Frame = std::array<uint16_t, kChannels>;
using Line = std::array<uint16_t, kMaxValues>;

struct Layout
{
    int pitch = 18;
    int yaw = 20;
    int zoom = 21;
    int picture = 22;
    int palette = 24;
    int drop = 11;
    int arm = 12;
    int bay = 5;
    int tilt = 19;
    int look = 23;
    bool operator==(const Layout& other) const = default;
};

Layout without(const Layout& layout, int channel);
constexpr uint16_t kArmedPwm = 1750;
constexpr uint16_t kSteadyPwm = 3;
constexpr uint16_t kPressedPwm = 1500;
constexpr uint16_t kSwitchLowPwm = 1250;
constexpr uint16_t kSwitchHighPwm = 1750;
constexpr qint64 kCameraResendMs = 100;
constexpr float kPitchDown = -90.0f;
constexpr float kPitchAhead = 0.0f;
constexpr float kYawLimit = 180.0f;
constexpr float kYawSpeedDefault = 20.0f;
constexpr float kYawDeadbandPwm = 40.0f;
constexpr qint64 kMaxTurnStepMs = 200;
constexpr float kStickCentrePwm = 1500.0f;
constexpr float kStickHalfSpanPwm = 512.0f;
constexpr std::array<int, 3> kPictures = {0, 1, 3};
constexpr int kBayPositions = 6;

enum class Flying
{
    NoVehicle,
    Vehicle,
    Standby,
    Joystick
};

enum class Kind
{
    Unknown,
    Copter,
    Plane,
    Rover
};

struct Sticks
{
    int roll = 1;
    int pitch = 2;
    int throttle = 3;
    int yaw = 4;
    bool operator==(const Sticks& other) const = default;
};

enum class Verdict
{
    Undecided,
    Radio,
    NotRadio
};

enum class State
{
    Off,
    Live,
    Ready,
    Paused,
    Standby,
    Stopped
};

uint16_t pwm(int raw);
std::optional<Line> parse(const QByteArray& line);
Frame flightChannels(const Line& line);
Frame released();
float between(float low, float high, uint16_t pwm);
float turnRate(uint16_t pwm, float speed);

struct Move
{
    enum class Kind
    {
        Aim,
        Zoom,
        Picture,
        NextPalette,
        Drop,
        DropBay
    };
    Kind kind = Kind::Aim;
    float pitch = 0.0f;
    float yaw = 0.0f;
    float level = 0.0f;
    int value = 0;
    bool operator==(const Move& other) const = default;
};

struct Switches
{
    bool armed = false;
    bool operator==(const Switches& other) const = default;
};

std::optional<Switches> switches(const Line& line, const Layout& layout = {});
int bay(uint16_t pwm);
int bayOf(const Line& line, const Layout& layout = {});

struct BayServo
{
    int output = 0;
    int releasePwm = 0;
    bool operator==(const BayServo& other) const = default;
};

std::optional<BayServo> bayServo(int bay, const QList<BayServo>& servos);

enum class Payload
{
    Off,
    Safe,
    Armed,
    Refused
};

Payload payload(std::optional<Switches> switches, bool flown, bool gripper);
QByteArray payloadName(Payload payload);

struct Ready
{
    bool gimbal = true;
    bool camera = true;
    bool settings = true;
};

struct Aiming
{
    float yawSpeed = kYawSpeedDefault;
    bool lookReversed = false;
    std::optional<float> reportedYaw;
};

class CameraSticks
{
public:
    QList<Move> step(const Line& line, qint64 nowMs, const Layout& layout = {}, Ready ready = {},
                     const Aiming& aiming = {});

private:
    QHash<int, uint16_t> _acted;
    std::optional<qint64> _sentMs;
    std::optional<uint16_t> _dropBefore;
    std::optional<uint16_t> _bayBefore;
    std::optional<uint16_t> _armBefore;
    std::optional<int> _pitchSource;
    std::optional<float> _heldPitch;
    std::optional<float> _yaw;
    std::optional<qint64> _turnedMs;
    bool _yawCentred = false;
};

QByteArray echo(Flying flying, int vehicle, Kind kind = Kind::Unknown, Sticks sticks = {}, const Layout& layout = {},
                int modeChannel = 0, Payload payload = Payload::Off, const QStringList& modes = {});
QStringList namedOn(const Layout& layout, int channel);
QString clashText(int channel, const QStringList& names);
Kind kind(int mavType);
Sticks sticks(int roll, int pitch, int throttle, int yaw);
bool echoDue(std::optional<qint64> last, qint64 nowMs, bool live);
uint8_t component(bool standby);
bool standbyAirframe(bool ardupilot, int mavType);
QString busyText(const QString& port);
QString silentText(const QString& port, qint64 silentMs);
QString flightText(Flying flying, int vehicle);
QString toolsFolder(const QStringList& roots);
QString saveScript(const QString& folder);
State state(bool on, bool live, Flying flying);
QString stateName(State state);

class Lines
{
public:
    QList<Line> feed(const QByteArray& bytes);

private:
    QByteArray _pending;
    std::optional<qsizetype> _width;
};

class Sniffer
{
public:
    Verdict feed(const QByteArray& bytes);

private:
    Lines _lines;
    int _frames = 0;
    qsizetype _seen = 0;
};

struct Send
{
    int vehicle = 0;
    Frame frame{};
    bool operator==(const Send& other) const = default;
};

struct Step
{
    QList<Send> sends;
    std::optional<int> handedBack;
    bool operator==(const Step& other) const = default;
};

class Radio
{
public:
    bool live(qint64 nowMs) const;
    std::optional<qint64> heardAt() const;
    Step step(qint64 nowMs, std::optional<int> target, std::optional<Frame> fresh);

private:
    std::optional<std::pair<Frame, qint64>> _latest;
    std::optional<qint64> _sentAt;
    std::optional<int> _holding;
};

}  // namespace UsbRadioProtocol

class UsbRadio : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("")

    Q_PROPERTY(QString status READ status NOTIFY statusChanged)
    Q_PROPERTY(bool receiving READ receiving NOTIFY statusChanged)
    Q_PROPERTY(QString state READ state NOTIFY stateChanged)
    Q_PROPERTY(QString offerPort READ offerPort NOTIFY offerChanged)
    Q_PROPERTY(QString offerLabel READ offerLabel NOTIFY offerChanged)

public:
    explicit UsbRadio(QObject* parent = nullptr);
    ~UsbRadio() override;

    static UsbRadio* instance();

    void init();

    QString status() const { return _status; }

    bool receiving() const { return _radio.live(_clock.elapsed()); }

    QString state() const { return _state; }

    QString offerPort() const { return _offerPort; }

    QString offerLabel() const { return _offerLabel; }

    bool isRadioPort(const QString& port) const;
    UsbRadioProtocol::Verdict sniff(const LinkInterface* link, const QByteArray& bytes);
    void forgetLink(const LinkInterface* link);
    void radioSeen(const QString& port);
    void portsPresent(const QStringList& ports);

    Q_INVOKABLE void useOffer(const QString& port);
    Q_INVOKABLE void dismissOffer(const QString& port);

    Q_INVOKABLE QVariantList ports() const;
    Q_INVOKABLE QString scriptFolder() const;
    Q_INVOKABLE QString saveScript(const QString& folder) const;

signals:
    void statusChanged();
    void stateChanged();
    void offerChanged();

private:
    struct Flight
    {
        UsbRadioProtocol::Flying flying = UsbRadioProtocol::Flying::NoVehicle;
        int vehicle = 0;
        UsbRadioProtocol::Kind kind = UsbRadioProtocol::Kind::Unknown;
        UsbRadioProtocol::Sticks sticks;
        int modeChannel = 0;
    };

    void _maintain();
    void _open(const QString& port);
    void _close();
    void _read();
    void _steer(const UsbRadioProtocol::Line& line);
    void _warnOfClash(int vehicle, const UsbRadioProtocol::Layout& layout, int modeChannel);
    void _apply(Vehicle* vehicle, const UsbRadioProtocol::Move& move, bool armed, int selectedBay) const;
    void _release(Vehicle* vehicle, int bay, bool pressed) const;
    QList<UsbRadioProtocol::BayServo> _bayServos(const Vehicle* vehicle) const;
    void _advance(std::optional<UsbRadioProtocol::Frame> fresh);
    void _send(int vehicle, bool standby, const UsbRadioProtocol::Frame& frame) const;
    void _setStatus(const QString& status);
    void _setState(const QString& state);
    void _setOffer(const QString& port, const QString& label);
    QString _wanted() const;
    UsbRadioProtocol::Layout _layout() const;
    bool _standby(int vehicle) const;
    std::optional<int> _target() const;
    Flight _flight() const;
    UsbRadioProtocol::Payload _payload(const Flight& flight) const;
    QStringList _modes() const;

    QTimer _timer;
    QElapsedTimer _clock;
    QSerialPort* _port = nullptr;
    QString _portName;
    std::optional<qint64> _openedAt;
    std::optional<qint64> _echoedAt;
    bool _wasStandby = false;
    UsbRadioProtocol::Lines _lines;
    std::optional<UsbRadioProtocol::Line> _line;
    UsbRadioProtocol::CameraSticks _camera;
    UsbRadioProtocol::Radio _radio;
    std::optional<std::pair<int, int>> _clash;
    QString _status;
    QString _state = UsbRadioProtocol::stateName(UsbRadioProtocol::State::Off);
    QHash<const LinkInterface*, UsbRadioProtocol::Sniffer> _sniffers;
    QSet<QString> _radioPorts;
    QString _offerPort;
    QString _offerLabel;
};
