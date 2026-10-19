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

using Frame = std::array<uint16_t, kChannels>;

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
std::optional<Frame> parse(const QByteArray& line);
Frame released();
QByteArray echo(Flying flying, int vehicle, Kind kind = Kind::Unknown, Sticks sticks = {});
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
    QList<Frame> feed(const QByteArray& bytes);

private:
    QByteArray _pending;
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
    };

    void _maintain();
    void _open(const QString& port);
    void _close();
    void _read();
    void _advance(std::optional<UsbRadioProtocol::Frame> fresh);
    void _send(int vehicle, bool standby, const UsbRadioProtocol::Frame& frame) const;
    void _setStatus(const QString& status);
    void _setState(const QString& state);
    void _setOffer(const QString& port, const QString& label);
    QString _wanted() const;
    bool _standby(int vehicle) const;
    std::optional<int> _target() const;
    Flight _flight() const;

    QTimer _timer;
    QElapsedTimer _clock;
    QSerialPort* _port = nullptr;
    QString _portName;
    std::optional<qint64> _openedAt;
    std::optional<qint64> _echoedAt;
    bool _wasStandby = false;
    UsbRadioProtocol::Lines _lines;
    UsbRadioProtocol::Radio _radio;
    QString _status;
    QString _state = UsbRadioProtocol::stateName(UsbRadioProtocol::State::Off);
    QHash<const LinkInterface*, UsbRadioProtocol::Sniffer> _sniffers;
    QSet<QString> _radioPorts;
    QString _offerPort;
    QString _offerLabel;
};
