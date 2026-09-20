#pragma once

#include <QtLocation/private/qgeotiledmapreply_p.h>
#include <QtNetwork/QNetworkRequest>

class QGCTileFetchReply;
class QNetworkAccessManager;

class QGeoTiledMapReplyQGC : public QGeoTiledMapReply
{
    Q_OBJECT

public:
    explicit QGeoTiledMapReplyQGC(QNetworkAccessManager *networkManager, const QNetworkRequest &request, const QGeoTileSpec &spec, QObject *parent = nullptr);
    ~QGeoTiledMapReplyQGC();

    bool init();
    void abort() final;

private:
    void _fetchFinished();

    QGCTileFetchReply *_fetch = nullptr;
};
