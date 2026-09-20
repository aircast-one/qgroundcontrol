#include "QGCTileFetchReply.h"

#include <QtCore/QFile>
#include <QtNetwork/QNetworkAccessManager>
#include <QtNetwork/QSslError>

#include <chrono>

#include "ElevationMapProvider.h"
#include "MapProvider.h"
#include "QGCCacheTile.h"
#include "QGCLoggingCategory.h"
#include "QGCMapEngine.h"
#include "QGCMapTasks.h"
#include "QGCMapUrlEngine.h"
#include "QGCNetworkHelper.h"
#include "QGCTileCache.h"

QGC_LOGGING_CATEGORY(QGCTileFetchReplyLog, "QtLocationPlugin.QGCTileFetchReply")

namespace {
// Keep pooled sockets warm across sparse tile/terrain fetches; Qt 6.11 otherwise reaps idle ones after 2 min.
constexpr int kConnectionCacheExpirySecs = 300;
constexpr std::chrono::seconds kTcpKeepAliveIdle{60};
constexpr std::chrono::seconds kTcpKeepAliveInterval{30};
constexpr int kTcpKeepAliveProbeCount = 3;

#if defined Q_OS_MACOS
constexpr const char *kUserAgent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.5; rv:125.0) Gecko/20100101 Firefox/125.0";
#elif defined Q_OS_WIN
constexpr const char *kUserAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:100.0) Gecko/20100101 Firefox/112.0";
#elif defined Q_OS_ANDROID
constexpr const char *kUserAgent = "Mozilla/5.0 (Android 13; Tablet; rv:68.0) Gecko/68.0 Firefox/112.0";
#elif defined Q_OS_LINUX
constexpr const char *kUserAgent = "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/112.0";
#else
constexpr const char *kUserAgent = "Qt Location based application";
#endif
}  // namespace

QByteArray QGCTileFetchReply::_bingNoTileImage;
QByteArray QGCTileFetchReply::_badTile;

QGCTileFetchReply::QGCTileFetchReply(QNetworkAccessManager *networkManager, const QNetworkRequest &request,
                                     int mapId, int x, int y, int zoom, QObject *parent)
    : QObject(parent)
    , _networkManager(networkManager)
    , _request(request)
    , _mapId(mapId)
    , _x(x)
    , _y(y)
    , _zoom(zoom)
{
    qCDebug(QGCTileFetchReplyLog) << this;
}

QGCTileFetchReply::~QGCTileFetchReply()
{
    qCDebug(QGCTileFetchReplyLog) << this;
}

QNetworkRequest QGCTileFetchReply::networkRequest(int mapId, int x, int y, int zoom)
{
    const SharedMapProvider mapProvider = UrlFactory::getMapProviderFromQtMapId(mapId);
    if (!mapProvider) {
        return QNetworkRequest();
    }

    QNetworkRequest request;
    request.setUrl(mapProvider->getTileURL(x, y, zoom));
    request.setPriority(QNetworkRequest::NormalPriority);
    request.setTransferTimeout(10000);

    request.setRawHeader(QByteArrayLiteral("Accept"), QByteArrayLiteral("*/*"));
    request.setHeader(QNetworkRequest::UserAgentHeader, kUserAgent);
    const QByteArray referrer = mapProvider->getReferrer().toUtf8();
    if (!referrer.isEmpty()) {
        request.setRawHeader(QByteArrayLiteral("Referer"), referrer);
    }
    const QByteArray token = mapProvider->getToken();
    if (!token.isEmpty()) {
        request.setRawHeader(QByteArrayLiteral("User-Token"), token);
    }
    request.setRawHeader(QByteArrayLiteral("Connection"), QByteArrayLiteral("keep-alive"));
    request.setAttribute(QNetworkRequest::ConnectionCacheExpiryTimeoutSecondsAttribute, kConnectionCacheExpirySecs);
    request.setTcpKeepAliveIdleTimeBeforeProbes(kTcpKeepAliveIdle);
    request.setTcpKeepAliveIntervalBetweenProbes(kTcpKeepAliveInterval);
    request.setTcpKeepAliveProbeCount(kTcpKeepAliveProbeCount);

    request.setAttribute(QNetworkRequest::CacheLoadControlAttribute, QNetworkRequest::PreferCache);
    request.setAttribute(QNetworkRequest::BackgroundRequestAttribute, true);
    request.setAttribute(QNetworkRequest::CacheSaveControlAttribute, true);
    request.setAttribute(QNetworkRequest::RedirectPolicyAttribute, QNetworkRequest::NoLessSafeRedirectPolicy);
    request.setAttribute(QNetworkRequest::Http2AllowedAttribute, true);
    request.setAttribute(QNetworkRequest::DoNotBufferUploadDataAttribute, false);

    return request;
}

bool QGCTileFetchReply::init()
{
    if (_initialized) {
        return true;
    }

    _initialized = true;

    _initDataFromResources();

    QGCFetchTileTask *task = QGCTileCache::createFetchTileTask(UrlFactory::getProviderTypeFromQtMapId(_mapId), _x, _y, _zoom);
    if (!task) {
        qCWarning(QGCTileFetchReplyLog) << "Failed to create fetch tile task";
        _initialized = false;
        return false;
    }
    (void) connect(task, &QGCFetchTileTask::tileFetched, this, &QGCTileFetchReply::_cacheReply);
    (void) connect(task, &QGCMapTask::error, this, &QGCTileFetchReply::_cacheError);
    if (!getQGCMapEngine()->addTask(task)) {
        task->deleteLater();
        _initialized = false;
        return false;
    }

    return true;
}

void QGCTileFetchReply::abort()
{
    emit aborted();
}

void QGCTileFetchReply::_setError(Error error, const QString &errorString)
{
    _error = error;
    _errorString = errorString;
    qCWarning(QGCTileFetchReplyLog) << error << errorString;
    _mapImageData = _badTile;
    _mapImageFormat = QStringLiteral("png");
    _cached = false;
    _setFinished();
}

void QGCTileFetchReply::_setFinished()
{
    if (_finished) {
        return;
    }
    _finished = true;
    emit finished();
}

void QGCTileFetchReply::_initDataFromResources()
{
    if (_bingNoTileImage.isEmpty()) {
        QFile file(":/res/BingNoTileBytes.dat");
        if (file.open(QFile::ReadOnly)) {
            _bingNoTileImage = file.readAll();
            file.close();
        }
    }

    if (_badTile.isEmpty()) {
        QFile file(":/res/images/notile.png");
        if (file.open(QFile::ReadOnly)) {
            _badTile = file.readAll();
            file.close();
        }
    }
}

void QGCTileFetchReply::_networkReplyFinished()
{
    QNetworkReply *const reply = qobject_cast<QNetworkReply *>(sender());
    if (!reply) {
        _setError(UnknownError, tr("Unexpected Error"));
        return;
    }
    reply->deleteLater();

    if (reply->error() != QNetworkReply::NoError) {
        return;
    }

    if (!reply->isOpen()) {
        _setError(ParseError, tr("Empty Reply"));
        return;
    }

    const int statusCode = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
    if (!QGCNetworkHelper::isHttpSuccess(statusCode)) {
        _setError(CommunicationError, reply->attribute(QNetworkRequest::HttpReasonPhraseAttribute).toString());
        return;
    }

    QByteArray image = reply->readAll();
    if (image.isEmpty()) {
        _setError(ParseError, tr("Image is Empty"));
        return;
    }

    const SharedMapProvider mapProvider = UrlFactory::getMapProviderFromQtMapId(_mapId);
    if (!mapProvider) {
        _setError(UnknownError, tr("Invalid Map Provider"));
        return;
    }

    if (mapProvider->isBingProvider() && (image == _bingNoTileImage)) {
        _setError(CommunicationError, tr("Bing Tile Above Zoom Level"));
        return;
    }

    if (mapProvider->isElevationProvider()) {
        const SharedElevationProvider elevationProvider = std::dynamic_pointer_cast<const ElevationProvider>(mapProvider);
        image = elevationProvider->serialize(image);
        if (image.isEmpty()) {
            _setError(ParseError, tr("Failed to Serialize Terrain Tile"));
            return;
        }
    }
    _mapImageData = image;

    const QString format = mapProvider->getImageFormat(image);
    if (format.isEmpty()) {
        _setError(ParseError, tr("Unknown Format"));
        return;
    }
    _mapImageFormat = format;

    QGCTileCache::cacheTile(mapProvider->getMapName(), _x, _y, _zoom, image, format);

    _setFinished();
}

void QGCTileFetchReply::_networkReplyError(QNetworkReply::NetworkError error)
{
    if (error != QNetworkReply::OperationCanceledError) {
        const QNetworkReply *const reply = qobject_cast<const QNetworkReply *>(sender());
        if (!reply) {
            _setError(CommunicationError, tr("Invalid Reply"));
        } else {
            _setError(CommunicationError, reply->errorString());
        }
    } else {
        _setFinished();
    }
}

void QGCTileFetchReply::_networkReplySslErrors(const QList<QSslError> &errors)
{
    QString errorString;
    for (const QSslError &error : errors) {
        if (!errorString.isEmpty()) {
            (void) errorString.append('\n');
        }
        (void) errorString.append(error.errorString());
    }

    if (!errorString.isEmpty()) {
        _setError(CommunicationError, errorString);
    }
}

void QGCTileFetchReply::_cacheReply(QGCCacheTile *tile)
{
    if (tile) {
        _mapImageData = tile->img;
        _mapImageFormat = tile->format;
        _cached = true;
        delete tile;
        _setFinished();
    } else {
        _setError(UnknownError, tr("Invalid Cache Tile"));
    }
}

void QGCTileFetchReply::_cacheError(QGCMapTask::TaskType type, QStringView errorString)
{
    Q_UNUSED(errorString);
    Q_UNUSED(type);

    if (!QGCNetworkHelper::isInternetAvailable()) {
        _setError(CommunicationError, tr("Network Not Available"));
        return;
    }

    _request.setOriginatingObject(this);

    QNetworkReply *const reply = _networkManager->get(_request);
    reply->setParent(this);
    QGCNetworkHelper::ignoreSslErrorsIfNeeded(reply);

    (void) connect(reply, &QNetworkReply::finished, this, &QGCTileFetchReply::_networkReplyFinished);
    (void) connect(reply, &QNetworkReply::errorOccurred, this, &QGCTileFetchReply::_networkReplyError);
    (void) connect(reply, &QNetworkReply::sslErrors, this, &QGCTileFetchReply::_networkReplySslErrors);
    (void) connect(this, &QGCTileFetchReply::aborted, reply, &QNetworkReply::abort);
}
