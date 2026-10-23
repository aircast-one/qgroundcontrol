#include "RouteRecorder.h"

#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QSaveFile>
#include <algorithm>

#include "AppSettings.h"
#include "KMLDomDocument.h"
#include "QGCGeo.h"
#include "SettingsManager.h"
#include "Vehicle.h"

RouteRecorder::RouteRecorder(QObject* parent) : QObject(parent)
{
    _saveTimer.setInterval(kSaveIntervalMs);
    (void) connect(&_saveTimer, &QTimer::timeout, this, [this]() { (void) _save(); });
}

void RouteRecorder::start(Vehicle* vehicle)
{
    if (!vehicle || _recording) {
        return;
    }

    const QString missionDir = SettingsManager::instance()->appSettings()->missionSavePath();
    if (missionDir.isEmpty()) {
        emit finished(QString(), tr("Route not recorded: set a save location in Application Settings."));
        return;
    }

    _name = tr("Route %1").arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyy-MM-dd hh-mm-ss")));
    _fileName = QDir(missionDir).filePath(QStringLiteral("%1.%2").arg(_name, AppSettings::kmlFileExtension));
    _path.clear();
    _pathVariant.clear();
    _distance = 0;
    _vehicle = vehicle;
    _recording = true;
    (void) connect(vehicle, &Vehicle::coordinateChanged, this, &RouteRecorder::_addCoordinate);
    _addCoordinate(vehicle->coordinate());
    _saveTimer.start();
    emit recordingChanged();
    emit pathChanged();
}

void RouteRecorder::stop()
{
    if (!_recording) {
        return;
    }

    if (_vehicle) {
        (void) disconnect(_vehicle, &Vehicle::coordinateChanged, this, &RouteRecorder::_addCoordinate);
    }
    _vehicle.clear();
    _saveTimer.stop();
    _recording = false;
    emit recordingChanged();

    if (routeFromPath(_path).count() < 2) {
        (void) QFile::remove(_fileName);
        emit finished(QString(), tr("Route not saved: the vehicle didn't fly far enough while recording."));
        return;
    }
    if (!_save()) {
        emit finished(QString(), tr("Route save error %1").arg(_fileName));
        return;
    }
    emit finished(_fileName, tr("Route saved: %1").arg(QFileInfo(_fileName).fileName()));
}

void RouteRecorder::_addCoordinate(const QGeoCoordinate& coordinate)
{
    if (!_vehicle || !_vehicle->flying() || !coordinate.isValid() || qIsNaN(coordinate.altitude())) {
        return;
    }
    if (!_path.isEmpty() && (_path.last().distanceTo(coordinate) < kMinSpacingMeters) &&
        (qAbs(_path.last().altitude() - coordinate.altitude()) < kMinSpacingMeters)) {
        return;
    }
    _distance += _path.isEmpty() ? 0 : _path.last().distanceTo(coordinate);
    _path.append(coordinate);
    _pathVariant.append(QVariant::fromValue(coordinate));
    emit pathChanged();
}

bool RouteRecorder::_save() const
{
    const QList<QGeoCoordinate> route = routeFromPath(_path);
    return (route.count() >= 2) && saveKml(_fileName, _name, route);
}

QList<QGeoCoordinate> RouteRecorder::trimVerticalEnds(const QList<QGeoCoordinate>& path, double radiusMeters)
{
    if (path.isEmpty()) {
        return {};
    }

    const QGeoCoordinate start = path.first();
    const QGeoCoordinate end = path.last();
    const auto firstAway = std::find_if(path.cbegin(), path.cend(), [&](const QGeoCoordinate& coordinate) {
        return start.distanceTo(coordinate) >= radiusMeters;
    });
    if (firstAway == path.cend()) {
        return {};
    }
    const auto lastAway = std::find_if(path.crbegin(), path.crend(), [&](const QGeoCoordinate& coordinate) {
        return end.distanceTo(coordinate) >= radiusMeters;
    });
    if (lastAway == path.crend()) {
        return {};
    }

    const qsizetype first = std::distance(path.cbegin(), firstAway) - 1;
    const qsizetype last = path.count() - std::distance(path.crbegin(), lastAway);
    return path.mid(first, last - first + 1);
}

QList<QGeoCoordinate> RouteRecorder::routeFromPath(const QList<QGeoCoordinate>& path)
{
    return QGCGeo::simplifyPath(trimVerticalEnds(path, kTrimRadiusMeters), kSimplifyToleranceMeters);
}

bool RouteRecorder::saveKml(const QString& fileName, const QString& name, const QList<QGeoCoordinate>& route)
{
    KMLDomDocument kml(name);
    QDomElement placemark = kml.addPlacemark(name, true);
    (void) kml.addLineString(placemark, route, QStringLiteral("absolute"), false);

    QSaveFile file(fileName);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        return false;
    }
    const QByteArray bytes = kml.toByteArray();
    return (file.write(bytes) == bytes.size()) && file.commit();
}
