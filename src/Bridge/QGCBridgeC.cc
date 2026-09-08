#include "QGCBridgeC.h"

#include "QGCBridgeCore.h"
#include "QGCCoreC.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QThread>

#include <cstdlib>
#include <cstring>

namespace
{

char *duplicate(const QString &text)
{
    const QByteArray utf8 = text.toUtf8();
    return strdup(utf8.constData());
}

template <typename Fn>
QString onQtThread(Fn body)
{
    QCoreApplication *const app = QCoreApplication::instance();
    if (!app || QThread::currentThread() == app->thread()) {
        return body();
    }

    QString result;
    QMetaObject::invokeMethod(app, [&] { result = body(); }, Qt::BlockingQueuedConnection);
    return result;
}

} // namespace

char *qgc_qt_get(const char *path)
{
    const QString copied = QString::fromUtf8(path);
    return duplicate(onQtThread([&] { return QGCBridgeCore::get(copied); }));
}

char *qgc_qt_get_fields(const char *path, const char *fields_csv)
{
    const QString copiedPath = QString::fromUtf8(path);
    const QString copiedFields = QString::fromUtf8(fields_csv);
    return duplicate(onQtThread([&] { return QGCBridgeCore::getFields(copiedPath, copiedFields); }));
}

char *qgc_qt_set(const char *path, const char *value_json)
{
    const QString copiedPath = QString::fromUtf8(path);
    const QString copiedValue = QString::fromUtf8(value_json);
    return duplicate(onQtThread([&] { return QGCBridgeCore::set(copiedPath, copiedValue); }));
}

char *qgc_qt_invoke(const char *path, const char *args_json)
{
    const QString copiedPath = QString::fromUtf8(path);
    const QString copiedArgs = QString::fromUtf8(args_json);
    return duplicate(onQtThread([&] { return QGCBridgeCore::invoke(copiedPath, copiedArgs); }));
}

void qgc_qt_watch(const char *paths_csv)
{
    const QString copied = QString::fromUtf8(paths_csv);
    (void) onQtThread([&] {
        QGCBridgeCore::watch(copied.split(QLatin1Char(','), Qt::SkipEmptyParts));
        return QString();
    });
}

void qgc_qt_set_event_handler(QGCCoreEventFn handler)
{
    if (!handler) {
        QGCBridgeCore::setEventHandler(nullptr);
        return;
    }
    QGCBridgeCore::setEventHandler([handler](const QString &path, const QString &json) {
        handler(path.toUtf8().constData(), json.toUtf8().constData());
    });
}

void qgc_qt_free(char *text)
{
    free(text);
}

#ifdef QGC_RUST_CORE

char *qgc_bridge_get(const char *path) { return qgc_core_get(path); }
char *qgc_bridge_get_fields(const char *path, const char *fields_csv) { return qgc_core_get_fields(path, fields_csv); }
char *qgc_bridge_set(const char *path, const char *value_json) { return qgc_core_set(path, value_json); }
char *qgc_bridge_invoke(const char *path, const char *args_json) { return qgc_core_invoke(path, args_json); }
void qgc_bridge_watch(const char *paths_csv) { qgc_core_watch(paths_csv); }
void qgc_bridge_watch_client(const char *client, const char *paths_csv) { qgc_core_watch_client(client, paths_csv); }
void qgc_bridge_set_event_handler(QGCBridgeEventFn handler) { qgc_core_set_event_handler(handler); }
void qgc_bridge_free(char *text) { qgc_core_free(text); }

#else

char *qgc_bridge_get(const char *path) { return qgc_qt_get(path); }
char *qgc_bridge_get_fields(const char *path, const char *fields_csv) { return qgc_qt_get_fields(path, fields_csv); }
char *qgc_bridge_set(const char *path, const char *value_json) { return qgc_qt_set(path, value_json); }
char *qgc_bridge_invoke(const char *path, const char *args_json) { return qgc_qt_invoke(path, args_json); }
void qgc_bridge_watch(const char *paths_csv) { qgc_qt_watch(paths_csv); }
void qgc_bridge_watch_client(const char *, const char *paths_csv) { qgc_qt_watch(paths_csv); }
void qgc_bridge_set_event_handler(QGCBridgeEventFn handler) { qgc_qt_set_event_handler(handler); }
void qgc_bridge_free(char *text) { qgc_qt_free(text); }

#endif
