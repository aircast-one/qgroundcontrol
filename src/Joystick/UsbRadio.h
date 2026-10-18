#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QElapsedTimer>
#include <QtCore/QList>
#include <QtCore/QLoggingCategory>
#include <QtCore/QObject>
#include <QtCore/QTimer>
#include <QtCore/QVariantList>
#include <QtQmlIntegration/QtQmlIntegration>
#include <array>
#include <optional>

class QSerialPort;

Q_DECLARE_LOGGING_CATEGORY(UsbRadioLog)

namespace UsbRadioProtocol {

constexpr int kChannels = 18;
constexpr qint64 kSendEveryMs = 40;
constexpr qint64 kStaleAfterMs = 500;
constexpr qint64 kReopenAfterMs = 1000;
constexpr qint64 kEchoEveryMs = 250;
constexpr int kReleaseFrames = 3;
constexpr int kBaud = 115200;
constexpr int kMaxLineBytes = 1024;
constexpr int kMinValues = 4;
constexpr int kMaxValues = 24;
constexpr int kOverrideMin = 800;
constexpr int kOverrideMax = 2200;
constexpr int kLowChannels = 8;
constexpr uint8_t kAutopilotComponent = 1;
constexpr uint8_t kStandbyComponent = 191;

using Frame = std::array<uint16_t, kChannels>;

enum class Flying
{
    NoVehicle,
    Vehicle,
    Standby,
    Joystick
};

uint16_t pwm(int raw);
std::optional<Frame> parse(const QByteArray& line);
Frame released();
QByteArray echo(Flying flying, int vehicle);
bool echoDue(std::optional<qint64> last, qint64 nowMs, bool live);
uint8_t component(bool standby);
bool standbyAirframe(bool ardupilot, int mavType);

class Lines
{
public:
    QList<Frame> feed(const QByteArray& bytes);

private:
    QByteArray _pending;
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

public:
    explicit UsbRadio(QObject* parent = nullptr);
    ~UsbRadio() override;

    static UsbRadio* instance();

    void init();

    QString status() const { return _status; }

    bool receiving() const { return _radio.live(_clock.elapsed()); }

    Q_INVOKABLE QVariantList ports() const;

signals:
    void statusChanged();

private:
    struct Flight
    {
        UsbRadioProtocol::Flying flying = UsbRadioProtocol::Flying::NoVehicle;
        int vehicle = 0;
    };

    void _maintain();
    void _open(const QString& port);
    void _close();
    void _read();
    void _advance(std::optional<UsbRadioProtocol::Frame> fresh);
    void _send(int vehicle, bool standby, const UsbRadioProtocol::Frame& frame) const;
    void _setStatus(const QString& status);
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
};
