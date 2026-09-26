#pragma once

#include <QtCore/QLoggingCategory>
#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtQmlIntegration/QtQmlIntegration>

class QJSEngine;
class QNetworkAccessManager;
class QNetworkReply;
class QQmlEngine;
class QTimer;

Q_DECLARE_LOGGING_CATEGORY(AircastAccountLog)

class AircastAccount : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

    Q_PROPERTY(QString apiBase READ apiBase WRITE setApiBase NOTIFY apiBaseChanged)
    Q_PROPERTY(bool signedIn READ signedIn NOTIFY tokensChanged)
    Q_PROPERTY(bool signingIn READ signingIn NOTIFY signInChanged)
    Q_PROPERTY(QString userCode READ userCode NOTIFY signInChanged)
    Q_PROPERTY(QString verificationUrl READ verificationUrl NOTIFY signInChanged)
    Q_PROPERTY(QString status READ status NOTIFY signInChanged)

public:
    static AircastAccount* instance();
#ifndef QGC_HEADLESS_CORE
    static AircastAccount* create(QQmlEngine* qmlEngine, QJSEngine* jsEngine);
#endif

    QString apiBase() const { return _apiBase; }

    void setApiBase(const QString& apiBase);

    bool signedIn() const { return !token(_apiBase).isEmpty(); }

    bool signingIn() const { return !_deviceCode.isEmpty(); }

    QString userCode() const { return _userCode; }

    QString verificationUrl() const { return _verificationUrl; }

    QString status() const { return _status; }

    QString token(const QString& apiBase) const;

    Q_INVOKABLE void signIn();
    Q_INVOKABLE void cancelSignIn();
    Q_INVOKABLE void signOut();

    static constexpr const char* kClientId = "aircast-qgc";

signals:
    void apiBaseChanged();
    void tokensChanged();
    void signInChanged();

private:
    explicit AircastAccount(QObject* parent = nullptr);

    void _poll();
    void _onCodeReply(QNetworkReply* reply);
    void _onTokenReply(QNetworkReply* reply);
    void _setStatus(const QString& status);
    void _finish(const QString& status);
    static QString _settingsKey(const QString& apiBase);

    QNetworkAccessManager* _network = nullptr;
    QTimer* _pollTimer = nullptr;
    QString _apiBase;
    QString _deviceCode;
    QString _userCode;
    QString _verificationUrl;
    QString _status;
};
