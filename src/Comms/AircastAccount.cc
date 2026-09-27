#include "AircastAccount.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QSettings>
#include <QtCore/QTimer>
#include <QtCore/QUrl>
#include <QtCore/QUrlQuery>

#include "QGCLoggingCategory.h"
#ifndef QGC_HEADLESS_CORE
#include <QtGui/QDesktopServices>
#include <QtQml/QJSEngine>
#endif
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QNetworkReply>
#include <QtNetwork/QNetworkRequest>

QGC_LOGGING_CATEGORY(AircastAccountLog, "Comms.AircastAccount")

namespace {
constexpr int kDefaultPollSecs = 5;
const QString kSettingsGroup = QStringLiteral("AircastAccount");
const QString kDeviceCodeGrant = QStringLiteral("urn:ietf:params:oauth:grant-type:device_code");
const QString kTokenExchangeGrant = QStringLiteral("urn:ietf:params:oauth:grant-type:token-exchange");
const QString kAccessTokenType = QStringLiteral("urn:ietf:params:oauth:token-type:access_token");
const QString kViewScope = QStringLiteral("sfu:view");
constexpr int kViewTokenRefreshMarginSecs = 60;
constexpr int kViewTokenDefaultTtlSecs = 300;

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

#ifndef QGC_HEADLESS_CORE
AircastAccount* AircastAccount::create(QQmlEngine* qmlEngine, QJSEngine* jsEngine)
{
    Q_UNUSED(qmlEngine);
    Q_UNUSED(jsEngine);
    AircastAccount* account = instance();
    QJSEngine::setObjectOwnership(account, QJSEngine::CppOwnership);
    return account;
}
#endif

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

void AircastAccount::viewToken(const QString& deviceId, QObject* context, ViewTokenCallback done)
{
    const CachedToken cached = _viewTokens.value(deviceId);
    if (!cached.token.isEmpty() &&
        QDateTime::currentDateTimeUtc().addSecs(kViewTokenRefreshMarginSecs) < cached.expiresAt) {
        done(cached.token);
        return;
    }
    const QString session = token(_apiBase);
    if (session.isEmpty() || deviceId.isEmpty()) {
        done(QString());
        return;
    }
    QNetworkRequest request = jsonRequest(_apiBase, QStringLiteral("/v1/oauth2/session-token"));
    request.setRawHeader("Authorization", "Bearer " + session.toUtf8());
    QNetworkReply* reply = _network->post(request, QByteArray());
    (void) connect(reply, &QNetworkReply::finished, context, [this, reply, deviceId, context, done]() {
        reply->deleteLater();
        const QString firstParty =
            QJsonDocument::fromJson(reply->readAll()).object().value(QStringLiteral("access_token")).toString();
        if (reply->error() != QNetworkReply::NoError || firstParty.isEmpty()) {
            qCWarning(AircastAccountLog) << "session token request failed" << reply->errorString();
            done(QString());
            return;
        }
        _exchangeForViewToken(deviceId, firstParty, context, done);
    });
}

void AircastAccount::_exchangeForViewToken(const QString& deviceId, const QString& firstPartyToken, QObject* context,
                                           ViewTokenCallback done)
{
    QUrlQuery form;
    form.addQueryItem(QStringLiteral("grant_type"), kTokenExchangeGrant);
    form.addQueryItem(QStringLiteral("client_id"), QString::fromLatin1(kClientId));
    form.addQueryItem(QStringLiteral("scope"), kViewScope);
    form.addQueryItem(QStringLiteral("subject_token"), firstPartyToken);
    form.addQueryItem(QStringLiteral("subject_token_type"), kAccessTokenType);
    form.addQueryItem(QStringLiteral("device_id"), deviceId);
    QNetworkRequest request = jsonRequest(_apiBase, QStringLiteral("/v1/oauth2/token"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, QStringLiteral("application/x-www-form-urlencoded"));
    QNetworkReply* reply = _network->post(request, form.toString(QUrl::FullyEncoded).toUtf8());
    (void) connect(reply, &QNetworkReply::finished, context, [this, reply, deviceId, done]() {
        reply->deleteLater();
        const QJsonObject obj = QJsonDocument::fromJson(reply->readAll()).object();
        const QString view = obj.value(QStringLiteral("access_token")).toString();
        if (reply->error() != QNetworkReply::NoError || view.isEmpty()) {
            qCWarning(AircastAccountLog) << "view token exchange failed" << reply->errorString()
                                         << obj.value(QStringLiteral("error_description")).toString();
            done(QString());
            return;
        }
        const int ttl = obj.value(QStringLiteral("expires_in")).toInt(kViewTokenDefaultTtlSecs);
        _viewTokens.insert(deviceId, CachedToken{view, QDateTime::currentDateTimeUtc().addSecs(ttl)});
        done(view);
    });
}

void AircastAccount::setApiBase(const QString& apiBase)
{
    const QString clean = apiBase.trimmed();
    if (clean == _apiBase) {
        return;
    }
    _apiBase = clean;
    _viewTokens.clear();
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
    _viewTokens.clear();
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
