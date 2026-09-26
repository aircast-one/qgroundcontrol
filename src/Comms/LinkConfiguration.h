#pragma once

#include <QtCore/QDeadlineTimer>
#include <QtCore/QElapsedTimer>
#include <QtCore/QSettings>
#include <QtCore/QString>
#include <QtQmlIntegration/QtQmlIntegration>

class LinkInterface;

/// \brief Interface holding link specific settings.
///
class LinkConfiguration : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("")
    Q_MOC_INCLUDE("LinkInterface.h")

    Q_PROPERTY(QString          name            READ name           WRITE setName           NOTIFY nameChanged)
    Q_PROPERTY(LinkInterface    *link           READ link                                   NOTIFY linkChanged)
    Q_PROPERTY(bool             linkActive      READ linkActive                             NOTIFY linkActiveChanged)
    Q_PROPERTY(LinkType         linkType        READ type                                   CONSTANT)
    Q_PROPERTY(bool             dynamic         READ isDynamic      WRITE setDynamic        NOTIFY dynamicChanged)
    Q_PROPERTY(bool             autoConnect     READ isAutoConnect  WRITE setAutoConnect    NOTIFY autoConnectChanged)
    Q_PROPERTY(QString          settingsURL     READ settingsURL                            CONSTANT)
    Q_PROPERTY(QString          settingsTitle   READ settingsTitle                          CONSTANT)
    Q_PROPERTY(bool             highLatency     READ isHighLatency  WRITE setHighLatency    NOTIFY highLatencyChanged)
    Q_PROPERTY(QString          summary         READ summary                                NOTIFY summaryChanged)
    Q_PROPERTY(QString          lastError       READ lastError                              NOTIFY lastErrorChanged)
    Q_PROPERTY(bool             heardVehicle    READ heardVehicle                           NOTIFY heardVehicleChanged)
    Q_PROPERTY(ErrorRemedy      lastErrorRemedy READ lastErrorRemedy                        NOTIFY lastErrorChanged)

public:
    LinkConfiguration(const QString &name, QObject *parent = nullptr);
    LinkConfiguration(const LinkConfiguration *copy, QObject *parent = nullptr);
    virtual ~LinkConfiguration();

    QString name() const { return _name; }
    void setName(const QString &name);

    LinkInterface *link() const { return _link.lock().get(); }
    bool heardVehicle() const;
    void setLink(const std::shared_ptr<LinkInterface> link);

    virtual QString summary() const { return QString(); }

    enum ErrorRemedy {
        RemedyRetry,
        RemedyEditAddress,
    };
    Q_ENUM(ErrorRemedy)

    QString lastError() const { return _lastError; }
    ErrorRemedy lastErrorRemedy() const { return _lastErrorRemedy; }
    void setLastError(const QString &error, ErrorRemedy remedy = RemedyRetry);

    /// True while the link is connected or being kept connected by auto-reconnect.
    /// Stays true across reconnect attempts so UI doesn't flicker between retries.
    bool linkActive() const { return (link() != nullptr) || (_autoConnect && _autoConnectStarted && !_suppressAutoReconnect); }

    /// Is this a dynamic configuration?
    ///     @return True if not persisted
    bool isDynamic() const { return _dynamic; }

    void setDynamic(bool dynamic = true);

    bool isForwarding() const { return _forwarding; }

    void setForwarding(bool forwarding = true) { _forwarding = forwarding; };

    bool isAutoConnect() const { return _autoConnect; }

    virtual void setAutoConnect(bool autoc = true);

    bool suppressAutoReconnect() const { return _suppressAutoReconnect; }
    void setSuppressAutoReconnect(bool suppress) {
        if (_suppressAutoReconnect != suppress) { _suppressAutoReconnect = suppress; emit linkActiveChanged(); }
    }

    bool autoConnectStarted() const { return _autoConnectStarted; }
    void setAutoConnectStarted(bool started) {
        if (_autoConnectStarted != started) { _autoConnectStarted = started; emit linkActiveChanged(); }
    }

    bool reconnectReady() const { return _nextReconnect.hasExpired(); }
    void noteReconnectAttempt() {
        const int exp = qMin(_reconnectAttempts, 16);
        _reconnectAttempts = qMin(_reconnectAttempts + 1, 17);
        _nextReconnect = QDeadlineTimer(qMin(_reconnectBaseMs << exp, _reconnectMaxMs));
    }
    void resetReconnectBackoff() { _reconnectAttempts = 0; _nextReconnect = QDeadlineTimer(); }
    void noteConnected() { _connectedTimer.start(); }
    /// Reset backoff only if the link stayed up long enough to count as working.
    void noteDisconnected() {
        if (_connectedTimer.isValid() && (_connectedTimer.elapsed() >= _reconnectStableMs)) {
            resetReconnectBackoff();
        }
        _connectedTimer.invalidate();
    }

    /// Is this a High Latency configuration?
    ///     @return True if this is an High Latency configuration (link with large delays).
    bool isHighLatency() const { return _highLatency; }

    void setHighLatency(bool hl = false);

    virtual void copyFrom(const LinkConfiguration *source);

    enum LinkType {
#ifndef QGC_NO_SERIAL_LINK
        TypeSerial,
#endif
        TypeUdp,        ///< UDP Link
        TypeTcp,        ///< TCP Link
        TypeBluetooth,  ///< Bluetooth Link
#ifdef QT_DEBUG
        TypeMock,
#endif
        TypeLogReplay,
        TypeAircastCloud,
        TypeLast
    };
    Q_ENUM(LinkType)

    virtual LinkType type() const = 0;

    virtual void loadSettings(QSettings &settings, const QString &root) = 0;

    virtual void saveSettings(QSettings &settings, const QString &root) const = 0;

    virtual QString settingsURL() const = 0;

    virtual QString settingsTitle() const = 0;

    static LinkConfiguration *createSettings(int type, const QString &name);

    static LinkConfiguration *duplicateSettings(const LinkConfiguration *source);

    static QString settingsRoot() { return QStringLiteral("LinkConfigurations"); }

signals:
    void nameChanged(const QString &name);
    void linkChanged();
    void heardVehicleChanged();
    void linkActiveChanged();
    void dynamicChanged();
    void autoConnectChanged();
    void highLatencyChanged();
    void lastErrorChanged();
    void summaryChanged();

protected:
    std::weak_ptr<LinkInterface> _link;

private:
    QString _name;
    QString _lastError;
    ErrorRemedy _lastErrorRemedy = RemedyRetry;
    bool _dynamic = false;
    bool _forwarding = false;
    bool _autoConnect = false;
    bool _highLatency = false;
    bool _suppressAutoReconnect = false; ///< User disconnected; skip auto-reconnect until manually reconnected (runtime only)
    bool _autoConnectStarted = false;    ///< Link was started at boot or manually connected; gates timer reconnect (runtime only)
    int _reconnectAttempts = 0;          ///< Consecutive failed auto-reconnect attempts (runtime only)
    QDeadlineTimer _nextReconnect;       ///< Earliest time the next auto-reconnect may run; default-expired = ready now
    QElapsedTimer _connectedTimer;       ///< Measures how long the current link has stayed connected (runtime only)

    static constexpr int _reconnectBaseMs = 1000;
    static constexpr int _reconnectMaxMs = 5000;
    static constexpr int _reconnectStableMs = 2000; ///< Min connected duration to count as a working link
};

typedef std::shared_ptr<LinkConfiguration> SharedLinkConfigurationPtr;
typedef std::weak_ptr<LinkConfiguration> WeakLinkConfigurationPtr;
