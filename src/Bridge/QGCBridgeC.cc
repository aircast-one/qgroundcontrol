#include "QGCBridgeC.h"

#include "QGCBridgeCore.h"

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

// QObject properties, QStrings and Facts are not thread safe, and Swift calls this ABI
// from whatever queue it likes -- the parameter load reads ~1400 facts off the main
// thread. Reading them beside the running Qt thread corrupted refcounts and crashed in
// Swift long afterwards, so every call is marshalled onto the thread Qt owns.
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

char *qgc_bridge_get(const char *path)
{
    const QString copied = QString::fromUtf8(path);
    return duplicate(onQtThread([&] { return QGCBridgeCore::get(copied); }));
}

char *qgc_bridge_get_fields(const char *path, const char *fields_csv)
{
    const QString copiedPath = QString::fromUtf8(path);
    const QString copiedFields = QString::fromUtf8(fields_csv);
    return duplicate(onQtThread([&] { return QGCBridgeCore::getFields(copiedPath, copiedFields); }));
}

char *qgc_bridge_set(const char *path, const char *value_json)
{
    const QString copiedPath = QString::fromUtf8(path);
    const QString copiedValue = QString::fromUtf8(value_json);
    return duplicate(onQtThread([&] { return QGCBridgeCore::set(copiedPath, copiedValue); }));
}

char *qgc_bridge_invoke(const char *path, const char *args_json)
{
    const QString copiedPath = QString::fromUtf8(path);
    const QString copiedArgs = QString::fromUtf8(args_json);
    return duplicate(onQtThread([&] { return QGCBridgeCore::invoke(copiedPath, copiedArgs); }));
}

void qgc_bridge_watch(const char *paths_csv)
{
    const QString copied = QString::fromUtf8(paths_csv);
    (void) onQtThread([&] {
        QGCBridgeCore::watch(copied.split(QLatin1Char(','), Qt::SkipEmptyParts));
        return QString();
    });
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
