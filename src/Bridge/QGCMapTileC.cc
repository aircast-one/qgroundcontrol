#include "QGCMapTileC.h"

#include "QGCCacheTile.h"
#include "QGCMapEngine.h"
#include "QGCMapTasks.h"
#include "QGCMapUrlEngine.h"
#include "SettingsManager.h"
#include "FlightMapSettings.h"
#include "Fact.h"

#include <QtCore/QString>

#include <cstring>

void qgc_map_tile_fetch(const char *mapType, int x, int y, int zoom,
                        QGCTileHandler handler, void *context)
{
    if (!handler) {
        return;
    }

    const QString type = QString::fromUtf8(mapType);
    const QString hash = UrlFactory::getTileHash(type, x, y, zoom);

    QGCFetchTileTask *const task = new QGCFetchTileTask(hash);

    // Qt::DirectConnection: the worker signals from its own thread and the handler
    // only copies bytes out, so there is nothing to marshal and nothing to outlive
    // the tile, which the task owns.
    (void) QObject::connect(task, &QGCFetchTileTask::tileFetched, task,
                            [handler, context](QGCCacheTile *tile) {
        if (tile && !tile->img().isEmpty()) {
            const QByteArray &bytes = tile->img();
            handler(reinterpret_cast<const unsigned char *>(bytes.constData()), bytes.size(), context);
        } else {
            handler(nullptr, 0, context);
        }
    }, Qt::DirectConnection);

    (void) QObject::connect(task, &QGCMapTask::error, task,
                            [handler, context](QGCMapTask::TaskType, QStringView) {
        handler(nullptr, 0, context);
    }, Qt::DirectConnection);

    if (!QGCMapEngine::instance()->addTask(task)) {
        handler(nullptr, 0, context);
        delete task;
    }
}

char *qgc_map_current_type(void)
{
    // QGC names imagery as "<provider> <type>", e.g. "Bing Satellite", which is what
    // the tile hash is keyed on.
    FlightMapSettings *const settings = SettingsManager::instance()->flightMapSettings();
    if (!settings) {
        return strdup("");
    }
    const QString type = QStringLiteral("%1 %2")
                             .arg(settings->mapProvider()->rawValueString(),
                                  settings->mapType()->rawValueString());
    return strdup(type.toUtf8().constData());
}
