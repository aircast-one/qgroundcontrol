#include "QGCBridgeC.h"

#include "QGCBridgeCore.h"

#include <cstdlib>
#include <cstring>

namespace
{

char *duplicate(const QString &text)
{
    const QByteArray utf8 = text.toUtf8();
    return strdup(utf8.constData());
}

} // namespace

char *qgc_bridge_get(const char *path)
{
    return duplicate(QGCBridgeCore::get(QString::fromUtf8(path)));
}

char *qgc_bridge_set(const char *path, const char *value_json)
{
    return duplicate(QGCBridgeCore::set(QString::fromUtf8(path), QString::fromUtf8(value_json)));
}

char *qgc_bridge_invoke(const char *path, const char *args_json)
{
    return duplicate(QGCBridgeCore::invoke(QString::fromUtf8(path), QString::fromUtf8(args_json)));
}

void qgc_bridge_watch(const char *paths_csv)
{
    QGCBridgeCore::watch(QString::fromUtf8(paths_csv).split(QLatin1Char(','), Qt::SkipEmptyParts));
}

void qgc_bridge_set_event_handler(QGCBridgeEventFn handler)
{
    if (!handler) {
        QGCBridgeCore::setEventHandler(nullptr);
        return;
    }
    QGCBridgeCore::setEventHandler([handler](const QString &path, const QString &json) {
        handler(path.toUtf8().constData(), json.toUtf8().constData());
    });
}

void qgc_bridge_free(char *text)
{
    free(text);
}
