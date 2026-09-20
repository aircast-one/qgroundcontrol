#pragma once

#include <QtCore/QObject>
#include <QtNetwork/QNetworkReply>
#include <QtNetwork/QNetworkRequest>

#include "QGCMapTaskBase.h"

struct QGCCacheTile;
class QNetworkAccessManager;
class QSslError;

class QGCTileFetchReply : public QObject
{
    Q_OBJECT

public:
    enum Error {
        NoError,
        CommunicationError,
        ParseError,
        UnknownError
    };

    QGCTileFetchReply(QNetworkAccessManager *networkManager, const QNetworkRequest &request,
                      int mapId, int x, int y, int zoom, QObject *parent = nullptr);
    ~QGCTileFetchReply();

    bool init();
    void abort();

    int mapId() const { return _mapId; }
    int x() const { return _x; }
    int y() const { return _y; }
    int zoom() const { return _zoom; }

    Error error() const { return _error; }
    QString errorString() const { return _errorString; }
    QByteArray mapImageData() const { return _mapImageData; }
    QString mapImageFormat() const { return _mapImageFormat; }
    bool isCached() const { return _cached; }
    bool isFinished() const { return _finished; }

    static QNetworkRequest networkRequest(int mapId, int x, int y, int zoom);
    /* QNetworkAccessManager queues the requests it receives. The number executed in parallel
     * depends on the protocol; for HTTP on desktop it is 6 per host/port combination. */
    static uint32_t concurrentDownloads(const QString &type) { Q_UNUSED(type); return 6; }

signals:
    void finished();
    void aborted();

private slots:
    void _networkReplyFinished();
    void _networkReplyError(QNetworkReply::NetworkError error);
    void _networkReplySslErrors(const QList<QSslError> &errors);
    void _cacheReply(QGCCacheTile *tile);
    void _cacheError(QGCMapTask::TaskType type, QStringView errorString);

private:
    static void _initDataFromResources();
    void _setError(Error error, const QString &errorString);
    void _setFinished();

    QNetworkAccessManager *_networkManager = nullptr;
    QNetworkRequest _request;
    const int _mapId;
    const int _x;
    const int _y;
    const int _zoom;
    bool _initialized = false;
    bool _finished = false;
    bool _cached = false;
    Error _error = NoError;
    QString _errorString;
    QByteArray _mapImageData;
    QString _mapImageFormat;

    static QByteArray _bingNoTileImage;
    static QByteArray _badTile;
};
