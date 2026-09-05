/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include <QtCore/QLoggingCategory>
#include <QtCore/QPointer>
#include <QtLocation/private/qgeotiledmapreply_p.h>
#include <QtNetwork/QNetworkReply>
#include <QtNetwork/QNetworkRequest>

#include "QGCMapTasks.h"

Q_DECLARE_LOGGING_CATEGORY(QGeoTiledMapReplyQGCLog)

class QNetworkAccessManager;
class QSslError;

class QGeoTiledMapReplyQGC : public QGeoTiledMapReply
{
    Q_OBJECT

public:
    QGeoTiledMapReplyQGC(QNetworkAccessManager *networkManager, const QNetworkRequest &request, const QGeoTileSpec &spec, QObject *parent = nullptr);
    ~QGeoTiledMapReplyQGC();


private slots:
    void _networkReplyFinished();
    void _networkReplyError(QNetworkReply::NetworkError error);
    void _networkReplySslErrors(const QList<QSslError> &errors);
    void _cacheReply(QGCCacheTile *tile);
    void _cacheError(QGCMapTask::TaskType type, QStringView errorString);

private:
    static void _initDataFromResources();

    QPointer<QNetworkAccessManager> _networkManager;
    QPointer<QNetworkReply> _reply;
    QNetworkRequest _request;

    static QByteArray _bingNoTileImage;
    static QByteArray _badTile;

    enum HTTP_Response {
        SUCCESS_OK = 200,
        REDIRECTION_MULTIPLE_CHOICES = 300
    };
};
