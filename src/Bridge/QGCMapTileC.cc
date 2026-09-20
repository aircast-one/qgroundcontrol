#include "QGCMapTileC.h"

#include "Fact.h"
#include "FlightMapSettings.h"
#include "QGCMapEngine.h"
#include "QGCMapUrlEngine.h"
#include "QGCQtThread.h"
#include "QGCTileCache.h"
#include "QGCTileFetchReply.h"
#include "SettingsManager.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QString>
#include <QtNetwork/QNetworkAccessManager>

#include <cstring>

static QNetworkAccessManager *tileNetworkManager()
{
    static QNetworkAccessManager *const manager = new QNetworkAccessManager(QCoreApplication::instance());
    return manager;
}

void qgc_map_tile_fetch(const char *mapType, int x, int y, int zoom,
                        QGCTileHandler handler, void *context)
{
    if (!handler) {
        return;
    }

    const QString type = QString::fromUtf8(mapType);

    qgcOnQtThreadVoid([&] {
        QGCMapEngine::instance()->init(QGCTileCache::getDatabaseFilePath());

        const int mapId = UrlFactory::getQtMapIdFromProviderType(type);
        const QNetworkRequest request = QGCTileFetchReply::networkRequest(mapId, x, y, zoom);
        if (request.url().isEmpty()) {
            handler(nullptr, 0, context);
            return;
        }

        QGCTileFetchReply *const reply = new QGCTileFetchReply(tileNetworkManager(), request, mapId, x, y, zoom);
        (void) QObject::connect(reply, &QGCTileFetchReply::finished, reply, [reply, handler, context] {
            reply->deleteLater();
            if (reply->error() != QGCTileFetchReply::NoError || reply->mapImageData().isEmpty()) {
                handler(nullptr, 0, context);
                return;
            }
            const QByteArray bytes = reply->mapImageData();
            handler(reinterpret_cast<const unsigned char *>(bytes.constData()), bytes.size(), context);
        });

        if (!reply->init()) {
            reply->deleteLater();
            handler(nullptr, 0, context);
        }
    });
}

char *qgc_map_current_type(void)
{
    return qgcOnQtThread([]() -> char * {
        FlightMapSettings *const settings = SettingsManager::instance()->flightMapSettings();
        if (!settings) {
            return strdup("");
        }
        const QString type = QStringLiteral("%1 %2")
                                 .arg(settings->mapProvider()->rawValueString(),
                                      settings->mapType()->rawValueString());
        return strdup(type.toUtf8().constData());
    });
}
