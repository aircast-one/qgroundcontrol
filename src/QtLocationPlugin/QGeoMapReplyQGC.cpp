#include "QGeoMapReplyQGC.h"

#include <QtLocation/private/qgeotilespec_p.h>

#include "QGCLoggingCategory.h"
#include "QGCTileFetchReply.h"

QGC_LOGGING_CATEGORY(QGeoTiledMapReplyQGCLog, "QtLocationPlugin.QGeoTiledMapReplyQGC")

QGeoTiledMapReplyQGC::QGeoTiledMapReplyQGC(QNetworkAccessManager *networkManager, const QNetworkRequest &request, const QGeoTileSpec &spec, QObject *parent)
    : QGeoTiledMapReply(spec, parent)
    , _fetch(new QGCTileFetchReply(networkManager, request, spec.mapId(), spec.x(), spec.y(), spec.zoom(), this))
{
    qCDebug(QGeoTiledMapReplyQGCLog) << this;

    (void) connect(_fetch, &QGCTileFetchReply::finished, this, &QGeoTiledMapReplyQGC::_fetchFinished);
}

QGeoTiledMapReplyQGC::~QGeoTiledMapReplyQGC()
{
    qCDebug(QGeoTiledMapReplyQGCLog) << this;
}

bool QGeoTiledMapReplyQGC::init()
{
    return _fetch->init();
}

void QGeoTiledMapReplyQGC::abort()
{
    _fetch->abort();
    QGeoTiledMapReply::abort();
}

void QGeoTiledMapReplyQGC::_fetchFinished()
{
    setMapImageData(_fetch->mapImageData());
    setMapImageFormat(_fetch->mapImageFormat());
    setCached(_fetch->isCached());

    if (_fetch->error() == QGCTileFetchReply::NoError) {
        setFinished(true);
        return;
    }

    switch (_fetch->error()) {
    case QGCTileFetchReply::CommunicationError:
        setError(QGeoTiledMapReply::CommunicationError, _fetch->errorString());
        break;
    case QGCTileFetchReply::ParseError:
        setError(QGeoTiledMapReply::ParseError, _fetch->errorString());
        break;
    default:
        setError(QGeoTiledMapReply::UnknownError, _fetch->errorString());
        break;
    }
}
