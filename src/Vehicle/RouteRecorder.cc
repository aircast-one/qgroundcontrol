#include "RouteRecorder.h"

#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtGui/QVector3D>

#include "AppMessages.h"
#include "AppSettings.h"
#include "KMLDomDocument.h"
#include "QGCGeo.h"
#include "SettingsManager.h"
#include "Vehicle.h"

RouteRecorder::RouteRecorder(QObject* parent) : QObject(parent) {}

void RouteRecorder::start(Vehicle* vehicle)
{
    if (!vehicle || _recording) {
        return;
    }

    _path.clear();
    _vehicle = vehicle;
    _recording = true;
    (void) connect(vehicle, &Vehicle::coordinateChanged, this, &RouteRecorder::_addCoordinate);
    _addCoordinate(vehicle->coordinate());
    emit recordingChanged();
}

QString RouteRecorder::stop()
{
    if (!_recording) {
        return QString();
    }

    if (_vehicle) {
        (void) disconnect(_vehicle, &Vehicle::coordinateChanged, this, &RouteRecorder::_addCoordinate);
    }
    _vehicle.clear();
    _recording = false;
    emit recordingChanged();

    if (_path.count() < 2) {
        QGC::showAppMessage(tr("Route not saved: the vehicle did not move while recording."));
        return QString();
    }

    const QString missionDir = SettingsManager::instance()->appSettings()->missionSavePath();
    if (missionDir.isEmpty()) {
        QGC::showAppMessage(tr("Route not saved: set a save location in Application Settings."));
        return QString();
    }

    const QString name =
        tr("Route %1").arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyy-MM-dd hh-mm-ss")));
    const QString fileName =
        QDir(missionDir).filePath(QStringLiteral("%1.%2").arg(name, AppSettings::kmlFileExtension));
    if (!saveKml(fileName, name, simplify(_path, kSimplifyToleranceMeters))) {
        QGC::showAppMessage(tr("Route save error %1").arg(fileName));
        return QString();
    }

    QGC::showAppMessage(tr("Route saved to %1. Open it in Plan to edit it and upload it as a mission.").arg(fileName));
    return fileName;
}

void RouteRecorder::_addCoordinate(const QGeoCoordinate& coordinate)
{
    if (!coordinate.isValid() || qIsNaN(coordinate.altitude())) {
        return;
    }
    if (!_path.isEmpty() && _path.last().distanceTo(coordinate) < kMinSpacingMeters &&
        qAbs(_path.last().altitude() - coordinate.altitude()) < kMinSpacingMeters) {
        return;
    }
    _path.append(coordinate);
}

QList<QGeoCoordinate> RouteRecorder::simplify(const QList<QGeoCoordinate>& path, double toleranceMeters)
{
    if (path.count() < 3) {
        return path;
    }

    QList<QVector3D> points;
    points.reserve(path.count());
    for (const QGeoCoordinate& coordinate : path) {
        points.append(QGCGeo::convertGpsToEnu(coordinate, path.first()));
    }

    QList<bool> keep(path.count(), false);
    keep.first() = true;
    keep.last() = true;

    QList<std::pair<qsizetype, qsizetype>> spans = {{0, path.count() - 1}};
    while (!spans.isEmpty()) {
        const auto [first, last] = spans.takeLast();
        const QVector3D direction = (points[last] - points[first]).normalized();
        qsizetype farthest = -1;
        float farthestDistance = static_cast<float>(toleranceMeters);
        for (qsizetype i = first + 1; i < last; i++) {
            const float distance = points[i].distanceToLine(points[first], direction);
            if (distance > farthestDistance) {
                farthestDistance = distance;
                farthest = i;
            }
        }
        if (farthest >= 0) {
            keep[farthest] = true;
            spans.append({first, farthest});
            spans.append({farthest, last});
        }
    }

    QList<QGeoCoordinate> simplified;
    for (qsizetype i = 0; i < path.count(); i++) {
        if (keep[i]) {
            simplified.append(path[i]);
        }
    }
    return simplified;
}

bool RouteRecorder::saveKml(const QString& fileName, const QString& name, const QList<QGeoCoordinate>& route)
{
    KMLDomDocument kml(name);
    QDomElement placemark = kml.addPlacemark(name, true);
    (void) kml.addLineString(placemark, route, QStringLiteral("absolute"), false);

    QFile file(fileName);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        return false;
    }
    const QByteArray bytes = kml.toByteArray();
    return file.write(bytes) == bytes.size();
}
