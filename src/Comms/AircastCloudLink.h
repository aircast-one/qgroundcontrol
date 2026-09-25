#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QLoggingCategory>
#include <QtCore/QString>
#include <QtCore/QUrl>
#include <QtNetwork/QAbstractSocket>

#include "LinkConfiguration.h"
#include "LinkInterface.h"

class QTimer;
class QWebSocket;

Q_DECLARE_LOGGING_CATEGORY(AircastCloudLinkLog)

class AircastCloudConfiguration : public LinkConfiguration
{
    Q_OBJECT

    Q_PROPERTY(QString apiBase READ apiBase WRITE setApiBase NOTIFY apiBaseChanged)
    Q_PROPERTY(QString deviceId READ deviceId WRITE setDeviceId NOTIFY deviceIdChanged)

public:
    explicit AircastCloudConfiguration(const QString& name, QObject* parent = nullptr);
    explicit AircastCloudConfiguration(const AircastCloudConfiguration* copy, QObject* parent = nullptr);
    ~AircastCloudConfiguration() override;

    LinkType type() const override { return LinkConfiguration::TypeAircastCloud; }

    void copyFrom(const LinkConfiguration* source) override;
    void loadSettings(QSettings& settings, const QString& root) override;
    void saveSettings(QSettings& settings, const QString& root) const override;

    QString settingsURL() const override { return QStringLiteral("AircastCloudSettings.qml"); }

    QString settingsTitle() const override { return tr("Aircast Cloud Link Settings"); }

    QString summary() const override { return tr("Aircast cloud · %1").arg(_deviceId); }

    QString apiBase() const { return _apiBase; }

    void setApiBase(const QString& apiBase);

    QString deviceId() const { return _deviceId; }

    void setDeviceId(const QString& deviceId);

    QUrl relayUrl() const;

signals:
    void apiBaseChanged();
    void deviceIdChanged();

private:
    QString _apiBase;
    QString _deviceId;
};

class AircastCloudLink : public LinkInterface
{
    Q_OBJECT

public:
    explicit AircastCloudLink(SharedLinkConfigurationPtr& config, QObject* parent = nullptr);
    ~AircastCloudLink() override;

    bool isConnected() const override;
    void disconnect() override;

    bool isSecureConnection() const override { return true; }

    bool isRelayed() const override { return true; }

    static constexpr char kControlPrefix = 0x00;

private slots:
    void _writeBytes(const QByteArray& bytes) override;
    void _onConnected();
    void _onDisconnected();
    void _onBinaryMessage(const QByteArray& message);
    void _onError(QAbstractSocket::SocketError error);
    void _open();

private:
    bool _connect() override;
    void _scheduleReconnect();

    const AircastCloudConfiguration* _cloudConfig = nullptr;
    QWebSocket* _socket = nullptr;
    QTimer* _reconnectTimer = nullptr;
    bool _wanted = false;
    bool _reportedError = false;
    int _failures = 0;
};
