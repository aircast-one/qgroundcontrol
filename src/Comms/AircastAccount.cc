#include "AircastAccount.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QSettings>
#include <QtCore/QTimer>
#include <QtCore/QUrl>

#include "QGCLoggingCategory.h"
#ifndef QGC_HEADLESS_CORE
#include <QtGui/QDesktopServices>
#endif
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkReply>
#include <QtNetwork/QNetworkRequest>
#include <QtQml/QJSEngine>

QGC_LOGGING_CATEGORY(AircastAccountLog, "Comms.AircastAccount")

namespace {
constexpr int kDefaultPollSecs = 5;
const QString kSettingsGroup = QStringLiteral("AircastAccount");
const QString kDeviceCodeGrant = QStringLiteral("urn:ietf:params:oauth:grant-type:device_code");

QNetworkRequest jsonRequest(const QString& apiBase, const QString& path)
{
    QString base = apiBase;
    while (base.endsWith(QLatin1Char('/'))) {
        base.chop(1);
    }
    QNetworkRequest request(QUrl(base + path));
    request.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/json"));
    return request;
}
}  // namespace

AircastAccount::AircastAccount(QObject* parent)
    : QObject(parent), _network(new QNetworkAccessManager(this)), _pollTimer(new QTimer(this))
{
    _pollTimer->setSingleShot(true);
    (void) connect(_pollTimer, &QTimer::timeout, this, &AircastAccount::_poll);
    QSettings settings;
    _apiBase = settings.value(kSettingsGroup + QStringLiteral("/apiBase")).toString();
}

AircastAccount* AircastAccount::instance()
{
    static AircastAccount* account = new AircastAccount(QCoreApplication::instance());
    return account;
}

AircastAccount* AircastAccount::create(QQmlEngine* qmlEngine, QJSEngine* jsEngine)
{
    Q_UNUSED(qmlEngine);
    Q_UNUSED(jsEngine);
    AircastAccount* account = instance();
    QJSEngine::setObjectOwnership(account, QJSEngine::CppOwnership);
    return account;
}

QString AircastAccount::_settingsKey(const QString& apiBase)
{
    return kSettingsGroup + QStringLiteral("/tokens/") + QUrl(apiBase).host();
}

QString AircastAccount::token(const QString& apiBase) const
{
    if (QUrl(apiBase).host().isEmpty()) {
        return {};
    }
    QSettings settings;
    return settings.value(_settingsKey(apiBase)).toString();
}

void AircastAccount::setApiBase(const QString& apiBase)
{
    const QString clean = apiBase.trimmed();
    if (clean == _apiBase) {
        return;
    }
    _apiBase = clean;
    QSettings settings;
    settings.setValue(kSettingsGroup + QStringLiteral("/apiBase"), _apiBase);
    emit apiBaseChanged();
    emit tokensChanged();
}

void AircastAccount::signIn()
{
    if (QUrl(_apiBase).host().isEmpty()) {
        _setStatus(tr("Set up from an Aircast device first — it tells QGroundControl which account server to use."));
        return;
    }
    _pollTimer->stop();
    _deviceCode.clear();
    _setStatus(tr("Asking the Aircast account server for a sign-in code…"));
    const QJsonObject body{{QStringLiteral("client_id"), QString::fromLatin1(kClientId)}};
    QNetworkReply* reply = _network->post(jsonRequest(_apiBase, QStringLiteral("/v1/oauth2/cli/code")),
                                          QJsonDocument(body).toJson(QJsonDocument::Compact));
    (void) connect(reply, &QNetworkReply::finished, this, [this, reply]() { _onCodeReply(reply); });
}

void AircastAccount::_onCodeReply(QNetworkReply* reply)
{
    reply->deleteLater();
    const QJsonObject obj = QJsonDocument::fromJson(reply->readAll()).object();
    if (reply->error() != QNetworkReply::NoError || obj.value(QStringLiteral("device_code")).toString().isEmpty()) {
        qCWarning(AircastAccountLog) << "sign-in code request failed" << reply->errorString();
        _finish(tr("Couldn't start sign-in: %1").arg(reply->errorString()));
        return;
    }
    _deviceCode = obj.value(QStringLiteral("device_code")).toString();
    _userCode = obj.value(QStringLiteral("user_code")).toString();
    _verificationUrl = obj.value(QStringLiteral("verification_uri_complete")).toString();
    const int interval = obj.value(QStringLiteral("interval")).toInt(kDefaultPollSecs);
    _pollTimer->setInterval(std::max(interval, 1) * 1000);
    _setStatus(tr("Approve code %1 in the browser to sign in.").arg(_userCode));
#ifndef QGC_HEADLESS_CORE
    (void) QDesktopServices::openUrl(QUrl(_verificationUrl));
#endif
    _pollTimer->start();
}

void AircastAccount::_poll()
{
    if (_deviceCode.isEmpty()) {
        return;
    }
    const QJsonObject body{
        {QStringLiteral("grant_type"), kDeviceCodeGrant},
        {QStringLiteral("device_code"), _deviceCode},
        {QStringLiteral("client_id"), QString::fromLatin1(kClientId)},
    };
    QNetworkReply* reply = _network->post(jsonRequest(_apiBase, QStringLiteral("/v1/oauth2/cli/token")),
                                          QJsonDocument(body).toJson(QJsonDocument::Compact));
    (void) connect(reply, &QNetworkReply::finished, this, [this, reply]() { _onTokenReply(reply); });
}

void AircastAccount::_onTokenReply(QNetworkReply* reply)
{
    reply->deleteLater();
    if (_deviceCode.isEmpty()) {
        return;
    }
    const QJsonObject obj = QJsonDocument::fromJson(reply->readAll()).object();
    const QString token = obj.value(QStringLiteral("access_token")).toString();
    if (!token.isEmpty()) {
        QSettings settings;
        settings.setValue(_settingsKey(_apiBase), token);
        _finish(tr("Signed in."));
        emit tokensChanged();
        return;
    }
    const QString error = obj.value(QStringLiteral("error")).toString();
    if (error == QStringLiteral("authorization_pending") || error == QStringLiteral("slow_down") ||
        (error.isEmpty() && reply->error() != QNetworkReply::NoError)) {
        _pollTimer->start();
        return;
    }
    qCWarning(AircastAccountLog) << "sign-in refused" << error
                                 << obj.value(QStringLiteral("error_description")).toString();
    _finish(error == QStringLiteral("expired_token")
                ? tr("The sign-in code expired — start again.")
                : tr("Sign-in failed: %1").arg(obj.value(QStringLiteral("error_description")).toString()));
}

void AircastAccount::cancelSignIn()
{
    _finish(QString());
}

void AircastAccount::signOut()
{
    QSettings settings;
    settings.remove(_settingsKey(_apiBase));
    _finish(tr("Signed out."));
    emit tokensChanged();
}

void AircastAccount::_setStatus(const QString& status)
{
    _status = status;
    emit signInChanged();
}

void AircastAccount::_finish(const QString& status)
{
    _pollTimer->stop();
    _deviceCode.clear();
    _userCode.clear();
    _verificationUrl.clear();
    _setStatus(status);
}
