#include "QGCBridgeCore.h"

#include "Fact.h"
#include "LinkManager.h"
#include "LogDownloadController.h"
#include "MAVLinkConsoleController.h"
#include "APMSensorsComponentController.h"
#include "GeoTagController.h"
#include "VideoManager.h"
#include "MAVLinkInspectorController.h"
#include "RadioComponentController.h"
#include "MultiVehicleManager.h"
#include "PlanMasterController.h"
#include "MissionCommandTree.h"
#include "PositionManager.h"
#include "QmlUnitsConversion.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "Vehicle.h"

#include <QtCore/QSequentialIterable>
#include <QtCore/QSet>
#include <QtCore/QCoreApplication>
#include <QtPositioning/QGeoCoordinate>
#include <QtCore/QHash>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaMethod>
#include <QtCore/QMetaProperty>
#include <QtCore/QStringList>
#include <QtCore/QThread>

#include <optional>
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
    if (name == QLatin1String("plan")) {
        // Native frontends have no QML view to own a plan controller, so the bridge
        // keeps one. Created on first use because start() begins syncing with the
        // vehicle, which is not wanted until something actually asks for the mission.
        static PlanMasterController *plan = nullptr;
        if (!plan) {
            plan = new PlanMasterController(QCoreApplication::instance());
            plan->setFlyView(false);
            plan->start();
        }
        return plan;
    }
    if (name == QLatin1String("logDownload")) {
        return LogDownloadController::instance();
    }
    if (name == QLatin1String("video")) {
        return VideoManager::instance();
    }
    if (name == QLatin1String("geoTag")) {
        static GeoTagController *geoTag = nullptr;
        if (!geoTag) {
            geoTag = new GeoTagController(QCoreApplication::instance());
        }
        return geoTag;
    }
    if (name == QLatin1String("positionManager")) {
        return QGCPositionManager::instance();
    }
    if (name == QLatin1String("units")) {
        // QmlUnitsConversion only forwards to FactMetaData statics, so a bridge-owned one
        // behaves exactly like QGroundControlQmlGlobal's. Facts cross the bridge cooked
        // while invokables and plain doubles stay metric, so a native head needs these to
        // put the two beside each other without mixing units.
        static QmlUnitsConversion *units = nullptr;
        if (!units) {
            units = new QmlUnitsConversion(QCoreApplication::instance());
        }
        return units;
    }
    if (name == QLatin1String("missionCommandTree")) {
        return MissionCommandTree::instance();
    }
    if (name == QLatin1String("mavlinkConsole")) {
        return MAVLinkConsoleController::instance();
    }
    if (name == QLatin1String("mavlinkInspector")) {
        return MAVLinkInspectorController::instance();
    }
    if (name == QLatin1String("sensorsCal")) {
        return APMSensorsComponentController::forActiveVehicle();
    }
    if (name == QLatin1String("radioCal")) {
        return RadioComponentController::forActiveVehicle();
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

QObject *variantListElement(const QVariant &value, const QString &segment)
{
    if (value.typeId() != QMetaType::QVariantList) {
        return nullptr;
    }

    bool isIndex = false;
    const int index = segment.toInt(&isIndex);
    if (!isIndex) {
        return nullptr;
    }

    const QVariantList list = value.toList();
    if ((index < 0) || (index >= list.size())) {
        return nullptr;
    }
    return list.at(index).value<QObject *>();
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
                                      QGenericReturnArgument(method.typeName(), &returned),
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
        if ((i + 1) < parts.size()) {
            if (QObject *const element = variantListElement(value, parts.at(i + 1))) {
                object = element;
                ++i;
                continue;
            }
        }
        return Resolved { object, parts.mid(i).join(QLatin1Char('.')) };
    }

    return Resolved { object, QString() };
}

QJsonObject objectJson(QObject *object, const QSet<QString> &fields = {}, bool compactFacts = false,
                       QSet<QString> *seen = nullptr);

QJsonValue variantJson(const QVariant &value)
{
    if (value.canConvert<QGeoCoordinate>()) {
        const QGeoCoordinate coordinate = value.value<QGeoCoordinate>();
        return coordinate.isValid()
            ? QJsonValue(QJsonObject {
                  // A direct read of a coordinate property carries "valid"; nested in an
                  // object read it did not, so a caller checking that key got nothing back
                  // for a perfectly good coordinate and could read it as invalid.
                  { QStringLiteral("valid"), true },
                  { QStringLiteral("latitude"), coordinate.latitude() },
                  { QStringLiteral("longitude"), coordinate.longitude() },
                  { QStringLiteral("altitude"), coordinate.altitude() },
              })
            : QJsonValue();
    }

    switch (value.metaType().id()) {
    case QMetaType::UChar:
    case QMetaType::SChar:
    case QMetaType::Char:
        return QJsonValue(value.toInt());
    default:
        break;
    }

    // Any registered sequential container, not just QVariantList: QList<int> and
    // QList<qreal> reach QJsonValue::fromVariant as themselves and come back null, so a
    // property like objectAvoidance.distances read as empty with nothing reporting why.
    if (value.canConvert<QSequentialIterable>() && value.metaType().id() != QMetaType::QString) {
        QJsonArray array;
        const QSequentialIterable iterable = value.value<QSequentialIterable>();
        for (const QVariant &element : iterable) {
            if (QObject *const child = element.value<QObject *>()) {
                array.append(objectJson(child));
                continue;
            }
            array.append(variantJson(element));
        }
        return array;
    }

    // A Q_ENUM has no QJsonValue conversion, so fromVariant yields null and a native head
    // silently loses the state instead of erroring. The number is what a caller wants; the
    // printed name changes with translations and refactors.
    if (value.metaType().flags().testFlag(QMetaType::IsEnumeration)) {
        bool numeric = false;
        const int enumerator = value.toInt(&numeric);
        if (numeric) {
            return QJsonValue(enumerator);
        }
    }

    return QJsonValue::fromVariant(value);
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
        QStringLiteral("minString"),
        QStringLiteral("maxString"),
        QStringLiteral("minIsDefaultForType"),
        QStringLiteral("maxIsDefaultForType"),
        QStringLiteral("defaultValueString"),
        QStringLiteral("defaultValueAvailable"),
        QStringLiteral("vehicleRebootRequired"),
        QStringLiteral("qgcRebootRequired"),
        QStringLiteral("decimalPlaces"),
        QStringLiteral("readOnly"),
    };

    QJsonObject json;
    json.insert(QStringLiteral("kind"), QStringLiteral("fact"));
    const bool defaultAvailable = fact->defaultValueAvailable();
    for (const QString &property : kFactProperties) {
        const bool skipped = (property == QLatin1String("defaultValueString")) && !defaultAvailable;
        json.insert(property, skipped ? QJsonValue() : QJsonValue::fromVariant(fact->property(property.toUtf8().constData())));
    }
    return json;
}

QJsonObject compactFactJson(Fact *fact)
{
    return QJsonObject {
        { QStringLiteral("kind"), QStringLiteral("fact") },
        { QStringLiteral("name"), fact->name() },
        { QStringLiteral("value"), QJsonValue::fromVariant(fact->cookedValue()) },
        { QStringLiteral("valueString"), fact->cookedValueString() },
    };
}

QObject *pointerWithoutDereference(const QVariant &value)
{
    if (!(value.metaType().flags() & QMetaType::PointerToQObject)) {
        return nullptr;
    }
    return *static_cast<QObject *const *>(value.constData());
}

Fact *factByStaticType(const QVariant &value, QObject *child)
{
    if (!child) {
        return nullptr;
    }
    const QMetaObject *const declared = value.metaType().metaObject();
    if (declared == &QObject::staticMetaObject) {
        return qobject_cast<Fact *>(child);
    }
    return declared && declared->inherits(&Fact::staticMetaObject) ? static_cast<Fact *>(child) : nullptr;
}

QJsonObject objectJson(QObject *object, const QSet<QString> &fields, bool compactFacts,
                       QSet<QString> *seen)
{
    const bool everything = fields.isEmpty();
    QJsonObject json;
    QJsonArray facts;
    QJsonArray children;

    const QMetaObject *const meta = object->metaObject();
    for (int i = 0; i < meta->propertyCount(); ++i) {
        const QMetaProperty property = meta->property(i);
        if (!property.isReadable()) {
            continue;
        }

        const QString name = QString::fromLatin1(property.name());
        if (seen) {
            seen->insert(name);
        }
        const bool wanted = everything || fields.contains(name);

        const QVariant value = property.read(object);

        QObject *const child = pointerWithoutDereference(value);
        if (Fact *const fact = factByStaticType(value, child)) {
            if (!wanted) {
                continue;
            }
            QJsonObject described = compactFacts ? compactFactJson(fact) : factJson(fact);
            described.insert(QStringLiteral("property"), name);
            facts.append(described);
            continue;
        }
        if (child) {
            children.append(name);
            continue;
        }
        if (!wanted) {
            continue;
        }
        json.insert(name, variantJson(value));
    }

    if (QmlObjectListModel *const model = qobject_cast<QmlObjectListModel *>(object)) {
        QJsonArray elements;
        for (int i = 0; i < model->count(); ++i) {
            QObject *const element = model->get(i);
            elements.append(element ? objectJson(element, fields, compactFacts, seen) : QJsonObject());
        }
        json.insert(QStringLiteral("elements"), elements);
    }

    json.insert(QStringLiteral("kind"), QStringLiteral("object"));
    json.insert(QStringLiteral("class"), QString::fromUtf8(object->metaObject()->className()));
    json.insert(QStringLiteral("facts"), facts);
    json.insert(QStringLiteral("children"), children);
    return json;
}

QJsonObject readPath(const QString &path, const QSet<QString> &fields = {}, bool compactFacts = false,
                     QSet<QString> *seen = nullptr)
{
    const Resolved resolved = resolve(path);
    if (!resolved.object) {
        return QJsonObject { { QStringLiteral("kind"), QStringLiteral("null") } };
    }

    if (resolved.property.isEmpty()) {
        if (Fact *const fact = qobject_cast<Fact *>(resolved.object)) {
            return factJson(fact);
        }
        return objectJson(resolved.object, fields, compactFacts, seen);
    }

    const QVariant value = resolved.object->property(resolved.property.toUtf8().constData());
    if (Fact *const fact = qobject_cast<Fact *>(value.value<QObject *>())) {
        return factJson(fact);
    }

    if (value.canConvert<QGeoCoordinate>()) {
        const QGeoCoordinate coordinate = value.value<QGeoCoordinate>();
        return QJsonObject {
            { QStringLiteral("kind"), QStringLiteral("coordinate") },
            { QStringLiteral("valid"), coordinate.isValid() },
            { QStringLiteral("latitude"), coordinate.latitude() },
            { QStringLiteral("longitude"), coordinate.longitude() },
            { QStringLiteral("altitude"), coordinate.altitude() },
        };
    }

    return QJsonObject {
        { QStringLiteral("kind"), QStringLiteral("value") },
        { QStringLiteral("value"), variantJson(value) },
    };
}

// resolve() follows the last segment into whatever object the property holds, which is
// what a read wants and the opposite of what a write wants. A property that currently
// reads null keeps its name and is writable; the same path with a live object under it
// arrives here with no property name at all. This re-resolves against the parent so a
// PointerToQObject property can be written whether or not it is already pointing at
// something.
Resolved resolveForWrite(const QString &path)
{
    const QStringList parts = path.split(QLatin1Char('.'), Qt::SkipEmptyParts);
    if (parts.size() < 2) {
        return Resolved();
    }

    const Resolved parent = resolve(parts.mid(0, parts.size() - 1).join(QLatin1Char('.')));
    if (!parent.object || !parent.property.isEmpty()) {
        return Resolved();
    }

    const QByteArray name = parts.last().toUtf8();
    if (parent.object->metaObject()->indexOfProperty(name.constData()) < 0) {
        return Resolved();
    }

    return Resolved { parent.object, parts.last() };
}

QJsonObject writePath(const QString &path, const QVariant &value)
{
    Resolved resolved = resolve(path);
    if (!resolved.object) {
        return QJsonObject {
            { QStringLiteral("ok"), false },
            { QStringLiteral("reason"), QStringLiteral("%1 does not resolve").arg(path) },
        };
    }

    if (resolved.property.isEmpty() && !qobject_cast<Fact *>(resolved.object)) {
        const Resolved viaParent = resolveForWrite(path);
        if (viaParent.object) {
            resolved = viaParent;
        }
    }

    if (resolved.property.isEmpty()) {
        Fact *const fact = qobject_cast<Fact *>(resolved.object);
        if (!fact) {
            return QJsonObject {
                { QStringLiteral("ok"), false },
                { QStringLiteral("reason"), QStringLiteral("%1 names an object, not a writable "
                                                           "property").arg(path) },
            };
        }
        fact->setCookedValue(value);
        return QJsonObject { { QStringLiteral("ok"), true } };
    }

    const QVariant existing = resolved.object->property(resolved.property.toUtf8().constData());
    if (Fact *const fact = qobject_cast<Fact *>(existing.value<QObject *>())) {
        fact->setCookedValue(value);
        return QJsonObject { { QStringLiteral("ok"), true } };
    }

    const QByteArray name = resolved.property.toUtf8();
    const QMetaObject *const meta = resolved.object->metaObject();
    const int index = meta->indexOfProperty(name.constData());
    if (index < 0) {
        return QJsonObject {
            { QStringLiteral("ok"), false },
            { QStringLiteral("reason"), QStringLiteral("no property %1 on %2")
                  .arg(resolved.property, QString::fromUtf8(meta->className())) },
        };
    }
    if (!meta->property(index).isWritable()) {
        return QJsonObject {
            { QStringLiteral("ok"), false },
            { QStringLiteral("reason"), QStringLiteral("%1 on %2 has no WRITE accessor")
                  .arg(resolved.property, QString::fromUtf8(meta->className())) },
        };
    }

    // A coordinate arrives as a JSON object, and QVariantMap converts to nothing the
    // property will take, so dragging a waypoint had no way to write where it landed.
    const QMetaType target = meta->property(index).metaType();
    if (target.flags().testFlag(QMetaType::PointerToQObject)) {
        const QString reference = value.toString();
        if (!reference.startsWith(QLatin1Char('@'))) {
            return QJsonObject {
                { QStringLiteral("ok"), false },
                { QStringLiteral("reason"), QStringLiteral("%1 takes an object, so the value must be "
                                                           "an @path").arg(resolved.property) },
            };
        }
        const Resolved referent = resolve(reference.mid(1));
        if (!referent.object || !referent.property.isEmpty()) {
            return QJsonObject {
                { QStringLiteral("ok"), false },
                { QStringLiteral("reason"), QStringLiteral("%1 does not resolve").arg(reference.mid(1)) },
            };
        }
        const QByteArray expected = QByteArray(target.name()).chopped(1);
        if (!referent.object->inherits(expected.constData())) {
            return QJsonObject {
                { QStringLiteral("ok"), false },
                { QStringLiteral("reason"), QStringLiteral("%1 is a %2, not a %3")
                      .arg(reference.mid(1), QString::fromUtf8(referent.object->metaObject()->className()),
                           QString::fromUtf8(expected)) },
            };
        }
        QObject *pointer = referent.object;
        const bool linked = resolved.object->setProperty(name.constData(), QVariant(target, &pointer));
        return QJsonObject {
            { QStringLiteral("ok"), linked },
            { QStringLiteral("reason"), linked ? QString()
                  : QStringLiteral("%1 rejected the object").arg(resolved.property) },
        };
    }

    if (target.id() == qMetaTypeId<QGeoCoordinate>() && value.canConvert<QVariantMap>()) {
        const QVariantMap point = value.toMap();
        if (!point.contains(QStringLiteral("latitude")) || !point.contains(QStringLiteral("longitude"))) {
            return QJsonObject {
                { QStringLiteral("ok"), false },
                { QStringLiteral("reason"), QStringLiteral("%1 needs latitude and longitude").arg(resolved.property) },
            };
        }
        const QGeoCoordinate coordinate(point.value(QStringLiteral("latitude")).toDouble(),
                                        point.value(QStringLiteral("longitude")).toDouble(),
                                        point.value(QStringLiteral("altitude")).toDouble());
        const bool placed = resolved.object->setProperty(name.constData(), QVariant::fromValue(coordinate));
        return QJsonObject {
            { QStringLiteral("ok"), placed },
            { QStringLiteral("reason"), placed ? QString() : QStringLiteral("%1 rejected the coordinate").arg(resolved.property) },
        };
    }

    const bool ok = resolved.object->setProperty(name.constData(), value);
    if (ok) {
        return QJsonObject { { QStringLiteral("ok"), true } };
    }
    return QJsonObject {
        { QStringLiteral("ok"), false },
        { QStringLiteral("reason"), QStringLiteral("%1 rejected the value").arg(resolved.property) },
    };
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
            if (method.parameterMetaType(arg).id() == qMetaTypeId<QGeoCoordinate>()) {
                const QJsonObject point = args.at(arg).toObject();
                if (!point.contains(QStringLiteral("latitude"))
                    || !point.contains(QStringLiteral("longitude"))) {
                    return QJsonObject { { QStringLiteral("ok"), false } };
                }
                values[arg] = QVariant::fromValue(QGeoCoordinate(
                    point.value(QStringLiteral("latitude")).toDouble(),
                    point.value(QStringLiteral("longitude")).toDouble(),
                    point.value(QStringLiteral("altitude")).toDouble()));
                generic[arg] = QGenericArgument(method.parameterMetaType(arg).name(), values[arg].constData());
                continue;
            }

            // A QVariant parameter wants the QVariant itself, not a value converted into
            // one — converting a double to the QVariant metatype fails and the call is
            // refused with no reason. QmlUnitsConversion takes all its arguments this way.
            if (method.parameterMetaType(arg).id() == QMetaType::QVariant) {
                generic[arg] = QGenericArgument("QVariant", &values[arg]);
                continue;
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

        // The metaobject records the return type as it was written, so a Q_ENUM declared
        // inside a class is "State" there and "Class::State" in the metatype. invoke compares
        // the two names and refuses, so the declared spelling is the one to hand it.
        QVariant returned(returnType);
        const bool ok = method.invoke(resolved.object, Qt::DirectConnection,
                                      QGenericReturnArgument(method.typeName(), returned.data()),
                                      generic[0], generic[1], generic[2], generic[3]);
        if (!ok) {
            return QJsonObject { { QStringLiteral("ok"), false } };
        }

        // A method declared to return QVariant hands back a QVariant holding one, and
        // the outer wrapper renders as nothing.
        if (returned.metaType().id() == QMetaType::QVariant) {
            returned = returned.value<QVariant>();
        }

        QJsonObject result { { QStringLiteral("ok"), true } };
        if (Fact *const fact = qobject_cast<Fact *>(returned.value<QObject *>())) {
            result.insert(QStringLiteral("result"), factJson(fact));
        } else if (QObject *const object = returned.value<QObject *>()) {
            result.insert(QStringLiteral("result"), objectJson(object));
        } else {
            result.insert(QStringLiteral("result"), variantJson(returned));
        }
        return result;
    }

    return QJsonObject { { QStringLiteral("ok"), false } };
}

QString jsonToString(const QJsonObject &json)
{
    return QString::fromUtf8(QJsonDocument(json).toJson(QJsonDocument::Compact));
}


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
        _unbindAll();
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
            if (_bound.contains(path)) {
                continue;
            }
            (void) _bind(path);
            _emit(path);
        }
    }

    bool _bind(const QString &path)
    {
        const Resolved resolved = resolve(path);
        Fact *const fact = qobject_cast<Fact *>(resolved.object);
        if (!fact || resolved.property.contains(QLatin1Char('.'))) {
            return false;
        }
        _bound.insert(path, {
            connect(fact, &Fact::rawValueChanged, this, [this, path]() { _emit(path); }),
            connect(fact, &QObject::destroyed, this, [this, path]() { _bound.remove(path); }),
        });
        return true;
    }

    void _emit(const QString &path)
    {
        if (!g_eventHandler) {
            return;
        }
        const QString json = jsonToString(readPath(path));
        if (_last.value(path) == json) {
            return;
        }
        _last.insert(path, json);
        g_eventHandler(path, json);
    }

    void _unbindAll()
    {
        for (const auto &connections : std::as_const(_bound)) {
            (void) disconnect(connections.first);
            (void) disconnect(connections.second);
        }
        _bound.clear();
    }

    QStringList _paths;
    QHash<QString, QString> _last;
    QHash<QString, std::pair<QMetaObject::Connection, QMetaObject::Connection>> _bound;
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

template<typename Fn>
void postToQtThread(Fn &&fn)
{
    QCoreApplication *const app = QCoreApplication::instance();
    if (!app) {
        return;
    }
    if (QThread::currentThread() == app->thread()) {
        fn();
        return;
    }
    QMetaObject::invokeMethod(app, std::forward<Fn>(fn), Qt::QueuedConnection);
}

std::optional<QVariant> variantFromJsonText(const QString &text)
{
    const QJsonDocument document = QJsonDocument::fromJson(text.toUtf8());
    const QJsonValue value = document.object().value(QStringLiteral("value"));
    if (value.isUndefined()) {
        return std::nullopt;
    }
    return value.toVariant();
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

QString getFields(const QString &path, const QString &fieldsCsv)
{
    const QStringList requested = fieldsCsv.split(QLatin1Char(','), Qt::SkipEmptyParts);
    QSet<QString> fields;
    for (const QString &field : requested) {
        const QString trimmed = field.trimmed();
        if (!trimmed.isEmpty()) {
            fields.insert(trimmed);
        }
    }

    // "*" keeps every property and still drops the fact metadata, because compacting the
    // facts is most of the saving and enumerating fields to get it is a list to maintain.
    const bool allProperties = fields.remove(QStringLiteral("*"));
    if (allProperties) {
        fields.clear();
    }

    QString result;
    runOnQtThread([&result, &path, &fields]() {
        QSet<QString> seen;
        QJsonObject json = readPath(path, fields, true, &seen);

        // A name that matched no property anywhere is a caller error - a typo, or a
        // binary older than the field it is asking for - and silently returning items
        // without it looks like a vehicle with nothing on it.
        QStringList unknown;
        for (const QString &field : fields) {
            if (!seen.contains(field)) {
                unknown.append(field);
            }
        }
        if (!unknown.isEmpty()) {
            unknown.sort();
            json.insert(QStringLiteral("unknownFields"), QJsonArray::fromStringList(unknown));
        }
        result = jsonToString(json);
    });
    return result;
}

QString set(const QString &path, const QString &valueJson)
{
    const std::optional<QVariant> value = variantFromJsonText(valueJson);
    if (!value) {
        return jsonToString(QJsonObject { { QStringLiteral("ok"), false } });
    }

    QString result;
    runOnQtThread([&result, &path, &value]() { result = jsonToString(writePath(path, *value)); });
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
    postToQtThread([paths]() { watcher()->setPaths(paths); });
}

void setEventHandler(EventHandler handler)
{
    runOnQtThread([&handler]() { g_eventHandler = std::move(handler); });
}

} // namespace QGCBridgeCore
