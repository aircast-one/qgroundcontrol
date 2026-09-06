#include "QGCBridgeCore.h"

#include "Fact.h"
#include "LinkManager.h"
#include "LogDownloadController.h"
#include "MAVLinkConsoleController.h"
#include "MAVLinkInspectorController.h"
#include "MultiVehicleManager.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "Vehicle.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QHash>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaMethod>
#include <QtCore/QMetaProperty>
#include <QtCore/QStringList>
#include <QtCore/QThread>
#include <QtCore/QTimer>

namespace
{

QGCBridgeCore::EventHandler g_eventHandler;

constexpr int kPollIntervalMSecs = 200;
constexpr int kMaxInvokeArgs = 4;

struct Resolved {
    QObject *object = nullptr;
    QString property;
};

QObject *rootObject(const QString &name)
{
    if (name == QLatin1String("settings")) {
        return SettingsManager::instance();
    }
    if (name == QLatin1String("vehicle")) {
        return MultiVehicleManager::instance()->activeVehicle();
    }
    if (name == QLatin1String("vehicles")) {
        return MultiVehicleManager::instance();
    }
    if (name == QLatin1String("links")) {
        return LinkManager::instance();
    }
    if (name == QLatin1String("logDownload")) {
        return LogDownloadController::instance();
    }
    if (name == QLatin1String("mavlinkConsole")) {
        return MAVLinkConsoleController::instance();
    }
    if (name == QLatin1String("mavlinkInspector")) {
        return MAVLinkInspectorController::instance();
    }
    return nullptr;
}

QObject *listElement(QObject *object, const QString &segment)
{
    bool isIndex = false;
    const int index = segment.toInt(&isIndex);
    if (!isIndex) {
        return nullptr;
    }

    QmlObjectListModel *const model = qobject_cast<QmlObjectListModel *>(object);
    if (!model || (index < 0) || (index >= model->count())) {
        return nullptr;
    }
    return model->get(index);
}

QObject *callSegment(QObject *object, const QString &segment)
{
    if (!segment.endsWith(QLatin1Char(')'))) {
        return nullptr;
    }

    const int open = segment.indexOf(QLatin1Char('('));
    if (open <= 0) {
        return nullptr;
    }

    const QByteArray methodName = segment.left(open).toUtf8();
    const QString argText = segment.mid(open + 1, segment.size() - open - 2);
    const QStringList args = argText.isEmpty() ? QStringList() : argText.split(QLatin1Char(','));
    if (args.size() > kMaxInvokeArgs) {
        return nullptr;
    }

    const QMetaObject *const meta = object->metaObject();
    for (int i = 0; i < meta->methodCount(); ++i) {
        const QMetaMethod method = meta->method(i);
        if ((method.name() != methodName) || (method.parameterCount() != args.size())) {
            continue;
        }
        if (!(method.returnMetaType().flags() & QMetaType::PointerToQObject)) {
            continue;
        }

        QVariant values[kMaxInvokeArgs];
        QGenericArgument generic[kMaxInvokeArgs];
        for (int arg = 0; arg < args.size(); ++arg) {
            values[arg] = args.at(arg);
            if (!values[arg].convert(method.parameterMetaType(arg))) {
                return nullptr;
            }
            generic[arg] = QGenericArgument(method.parameterMetaType(arg).name(), values[arg].constData());
        }

        QObject *returned = nullptr;
        const bool ok = method.invoke(object, Qt::DirectConnection,
                                      QGenericReturnArgument(method.returnMetaType().name(), &returned),
                                      generic[0], generic[1], generic[2], generic[3]);
        return ok ? returned : nullptr;
    }

    return nullptr;
}

Resolved resolve(const QString &path)
{
    const QStringList parts = path.split(QLatin1Char('.'), Qt::SkipEmptyParts);
    if (parts.isEmpty()) {
        return Resolved();
    }

    QObject *object = rootObject(parts.first());
    for (int i = 1; object && (i < parts.size()); ++i) {
        if (QObject *const element = listElement(object, parts.at(i))) {
            object = element;
            continue;
        }
        if (QObject *const called = callSegment(object, parts.at(i))) {
            object = called;
            continue;
        }
        const QVariant value = object->property(parts.at(i).toUtf8().constData());
        QObject *const child = value.value<QObject *>();
        if (child) {
            object = child;
            continue;
        }
        return Resolved { object, parts.mid(i).join(QLatin1Char('.')) };
    }

    return Resolved { object, QString() };
}

QJsonObject factJson(Fact *fact)
{
    static const QStringList kFactProperties = {
        QStringLiteral("name"),
        QStringLiteral("shortDescription"),
        QStringLiteral("units"),
        QStringLiteral("value"),
        QStringLiteral("valueString"),
        QStringLiteral("enumOrValueString"),
        QStringLiteral("enumStrings"),
        QStringLiteral("enumValues"),
        QStringLiteral("enumIndex"),
        QStringLiteral("typeIsBool"),
        QStringLiteral("typeIsString"),
        QStringLiteral("min"),
        QStringLiteral("max"),
        QStringLiteral("decimalPlaces"),
        QStringLiteral("readOnly"),
    };

    QJsonObject json;
    json.insert(QStringLiteral("kind"), QStringLiteral("fact"));
    for (const QString &property : kFactProperties) {
        json.insert(property, QJsonValue::fromVariant(fact->property(property.toUtf8().constData())));
    }
    return json;
}

QJsonObject objectJson(QObject *object);

QJsonObject objectJson(QObject *object)
{
    QJsonObject json;
    QJsonArray facts;
    QJsonArray children;

    const QMetaObject *const meta = object->metaObject();
    for (int i = 0; i < meta->propertyCount(); ++i) {
        const QMetaProperty property = meta->property(i);
        if (!property.isReadable()) {
            continue;
        }

        const QVariant value = property.read(object);
        QObject *const child = value.value<QObject *>();
        if (Fact *const fact = qobject_cast<Fact *>(child)) {
            facts.append(factJson(fact));
            continue;
        }
        if (child) {
            children.append(QString::fromLatin1(property.name()));
            continue;
        }
        json.insert(QString::fromLatin1(property.name()), QJsonValue::fromVariant(value));
    }

    if (QmlObjectListModel *const model = qobject_cast<QmlObjectListModel *>(object)) {
        QJsonArray elements;
        for (int i = 0; i < model->count(); ++i) {
            QObject *const element = model->get(i);
            elements.append(element ? objectJson(element) : QJsonObject());
        }
        json.insert(QStringLiteral("elements"), elements);
    }

    json.insert(QStringLiteral("kind"), QStringLiteral("object"));
    json.insert(QStringLiteral("facts"), facts);
    json.insert(QStringLiteral("children"), children);
    return json;
}

QJsonObject readPath(const QString &path)
{
    const Resolved resolved = resolve(path);
    if (!resolved.object) {
        return QJsonObject { { QStringLiteral("kind"), QStringLiteral("null") } };
    }

    if (resolved.property.isEmpty()) {
        if (Fact *const fact = qobject_cast<Fact *>(resolved.object)) {
            return factJson(fact);
        }
        return objectJson(resolved.object);
    }

    const QVariant value = resolved.object->property(resolved.property.toUtf8().constData());
    if (Fact *const fact = qobject_cast<Fact *>(value.value<QObject *>())) {
        return factJson(fact);
    }

    return QJsonObject {
        { QStringLiteral("kind"), QStringLiteral("value") },
        { QStringLiteral("value"), QJsonValue::fromVariant(value) },
    };
}

QJsonObject writePath(const QString &path, const QVariant &value)
{
    const Resolved resolved = resolve(path);
    if (!resolved.object) {
        return QJsonObject { { QStringLiteral("ok"), false } };
    }

    if (resolved.property.isEmpty()) {
        Fact *const fact = qobject_cast<Fact *>(resolved.object);
        if (!fact) {
            return QJsonObject { { QStringLiteral("ok"), false } };
        }
        fact->setCookedValue(value);
        return QJsonObject { { QStringLiteral("ok"), true } };
    }

    const QVariant existing = resolved.object->property(resolved.property.toUtf8().constData());
    if (Fact *const fact = qobject_cast<Fact *>(existing.value<QObject *>())) {
        fact->setCookedValue(value);
        return QJsonObject { { QStringLiteral("ok"), true } };
    }

    const bool ok = resolved.object->setProperty(resolved.property.toUtf8().constData(), value);
    return QJsonObject { { QStringLiteral("ok"), ok } };
}

QJsonObject invokePath(const QString &path, const QJsonArray &args)
{
    const Resolved resolved = resolve(path);
    if (!resolved.object || resolved.property.isEmpty()) {
        return QJsonObject { { QStringLiteral("ok"), false } };
    }

    const QByteArray methodName = resolved.property.toUtf8();
    const QMetaObject *const meta = resolved.object->metaObject();
    for (int i = 0; i < meta->methodCount(); ++i) {
        const QMetaMethod method = meta->method(i);
        if ((method.name() != methodName) || (method.parameterCount() != args.size())) {
            continue;
        }
        if (args.size() > kMaxInvokeArgs) {
            break;
        }

        QVariant values[kMaxInvokeArgs];
        QGenericArgument generic[kMaxInvokeArgs];
        for (int arg = 0; arg < args.size(); ++arg) {
            values[arg] = args.at(arg).toVariant();
            const QString reference = values[arg].toString();
            if (reference.startsWith(QLatin1Char('@'))) {
                const Resolved target = resolve(reference.mid(1));
                if (!target.object || !target.property.isEmpty()) {
                    return QJsonObject { { QStringLiteral("ok"), false } };
                }
                values[arg] = QVariant::fromValue(target.object);
            }
            if (!values[arg].convert(method.parameterMetaType(arg))) {
                return QJsonObject { { QStringLiteral("ok"), false } };
            }
            generic[arg] = QGenericArgument(method.parameterMetaType(arg).name(), values[arg].constData());
        }

        const QMetaType returnType = method.returnMetaType();
        if (returnType.id() == QMetaType::Void) {
            const bool ok = method.invoke(resolved.object, Qt::DirectConnection,
                                          generic[0], generic[1], generic[2], generic[3]);
            return QJsonObject { { QStringLiteral("ok"), ok } };
        }

        QVariant returned(returnType);
        const bool ok = method.invoke(resolved.object, Qt::DirectConnection,
                                      QGenericReturnArgument(returnType.name(), returned.data()),
                                      generic[0], generic[1], generic[2], generic[3]);
        if (!ok) {
            return QJsonObject { { QStringLiteral("ok"), false } };
        }

        QJsonObject result { { QStringLiteral("ok"), true } };
        if (Fact *const fact = qobject_cast<Fact *>(returned.value<QObject *>())) {
            result.insert(QStringLiteral("result"), factJson(fact));
        } else if (QObject *const object = returned.value<QObject *>()) {
            result.insert(QStringLiteral("result"), objectJson(object));
        } else {
            result.insert(QStringLiteral("result"), QJsonValue::fromVariant(returned));
        }
        return result;
    }

    return QJsonObject { { QStringLiteral("ok"), false } };
}

QString jsonToString(const QJsonObject &json)
{
    return QString::fromUtf8(QJsonDocument(json).toJson(QJsonDocument::Compact));
}


// ponytail: watched paths are polled and diffed, not signal-connected. Move to
// generic QMetaMethod notify connections if 200ms latency or the poll cost bites.
class Watcher : public QObject
{
public:
    explicit Watcher(QObject *parent = nullptr)
        : QObject(parent)
    {
        (void) connect(&_timer, &QTimer::timeout, this, &Watcher::_poll);
        _timer.setInterval(kPollIntervalMSecs);
    }

    void setPaths(const QStringList &paths)
    {
        _paths = paths;
        _last.clear();
        if (_paths.isEmpty()) {
            _timer.stop();
            return;
        }
        _timer.start();
        _poll();
    }

private:
    void _poll()
    {
        if (!g_eventHandler) {
            return;
        }
        for (const QString &path : std::as_const(_paths)) {
            const QString json = jsonToString(readPath(path));
            if (_last.value(path) == json) {
                continue;
            }
            _last.insert(path, json);
            g_eventHandler(path, json);
        }
    }

    QStringList _paths;
    QHash<QString, QString> _last;
    QTimer _timer;
};

Watcher *watcher()
{
    static Watcher *instance = nullptr;
    if (!instance) {
        instance = new Watcher(QCoreApplication::instance());
    }
    return instance;
}

template<typename Fn>
void runOnQtThread(Fn &&fn)
{
    QCoreApplication *const app = QCoreApplication::instance();
    if (!app || (QThread::currentThread() == app->thread())) {
        fn();
        return;
    }
    QMetaObject::invokeMethod(app, std::forward<Fn>(fn), Qt::BlockingQueuedConnection);
}

QVariant variantFromJsonText(const QString &text)
{
    const QJsonDocument document = QJsonDocument::fromJson(text.toUtf8());
    return document.object().value(QStringLiteral("value")).toVariant();
}

} // namespace

namespace QGCBridgeCore
{

QString get(const QString &path)
{
    QString result;
    runOnQtThread([&result, &path]() { result = jsonToString(readPath(path)); });
    return result;
}

QString set(const QString &path, const QString &valueJson)
{
    const QVariant value = variantFromJsonText(valueJson);
    QString result;
    runOnQtThread([&result, &path, &value]() { result = jsonToString(writePath(path, value)); });
    return result;
}

QString invoke(const QString &path, const QString &argsJson)
{
    const QJsonArray args = QJsonDocument::fromJson(argsJson.toUtf8()).array();
    QString result;
    runOnQtThread([&result, &path, &args]() { result = jsonToString(invokePath(path, args)); });
    return result;
}

void watch(const QStringList &paths)
{
    runOnQtThread([paths]() { watcher()->setPaths(paths); });
}

void setEventHandler(EventHandler handler)
{
    runOnQtThread([&handler]() { g_eventHandler = std::move(handler); });
}

} // namespace QGCBridgeCore
