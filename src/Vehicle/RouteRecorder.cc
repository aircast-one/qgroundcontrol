#include "RouteRecorder.h"

#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QRegularExpression>
#include <QtCore/QSaveFile>
#include <algorithm>
#include <cmath>

#include "AppSettings.h"
#include "Gimbal.h"
#include "GimbalController.h"
#include "KMLDomDocument.h"
#include "KMLHelper.h"
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
    _samples.clear();
    _pathVariant.clear();
    _distance = 0;
    _lookAt = QGeoCoordinate();
    _vehicle = vehicle;
    _recording = true;
    (void) connect(vehicle, &Vehicle::coordinateChanged, this, &RouteRecorder::_addCoordinate);
    (void) connect(vehicle, &Vehicle::roiCoordChanged, this, &RouteRecorder::_lookAtChanged);
    (void) connect(vehicle, &Vehicle::isROIEnabledChanged, this, [this]() {
        if (_vehicle && !_vehicle->isROIEnabled()) {
            _lookAt = QGeoCoordinate();
        }
    });
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
        (void) disconnect(_vehicle, nullptr, this, nullptr);
    }
    _vehicle.clear();
    _saveTimer.stop();
    _recording = false;
    emit recordingChanged();

    if (_route().count() < 2) {
        (void) QFile::remove(_fileName);
        emit finished(QString(), tr("Route not saved: the vehicle didn't fly far enough while recording."));
        return;
    }
    if (!_save()) {
        emit finished(QString(), tr("Route save error %1").arg(_fileName));
        return;
    }
    emit finished(_fileName, tr("Route saved"));
}

QString RouteRecorder::renameFile(const QString& fileName, const QString& newName)
{
    const QString cleaned =
        QString(newName).replace(QRegularExpression(QStringLiteral("[/\\\\:*?\"<>|]")), QStringLiteral("-")).trimmed();
    const QFileInfo info(fileName);
    if (cleaned.isEmpty() || !info.exists()) {
        return fileName;
    }
    const QString target = info.dir().filePath(QStringLiteral("%1.%2").arg(cleaned, info.suffix()));
    if ((target == fileName) || QFileInfo::exists(target)) {
        return fileName;
    }
    return QFile::rename(fileName, target) ? target : fileName;
}

void RouteRecorder::_lookAtChanged(const QGeoCoordinate& center)
{
    if (!_vehicle || !center.isValid()) {
        return;
    }
    const double homeAltitude = _vehicle->homePosition().altitude();
    const double relativeAltitude = std::isfinite(center.altitude()) ? center.altitude() : 0.0;
    _lookAt = QGeoCoordinate(center.latitude(), center.longitude(),
                             (std::isfinite(homeAltitude) ? homeAltitude : 0.0) + relativeAltitude);
}

void RouteRecorder::_addCoordinate(const QGeoCoordinate& coordinate)
{
    if (!_vehicle || !_vehicle->flying() || !coordinate.isValid() || qIsNaN(coordinate.altitude())) {
        return;
    }
    if (!_samples.isEmpty() && (_samples.last().coordinate.distanceTo(coordinate) < kMinSpacingMeters) &&
        (qAbs(_samples.last().coordinate.altitude() - coordinate.altitude()) < kMinSpacingMeters)) {
        return;
    }

    Sample sample;
    sample.coordinate = coordinate;
    sample.lookAt = _lookAt;
    Gimbal* gimbal = _vehicle->gimbalController() ? _vehicle->gimbalController()->activeGimbal() : nullptr;
    if (gimbal) {
        sample.gimbalPitch = gimbal->absolutePitch()->rawValue().toDouble();
        sample.gimbalYaw = gimbal->bodyYaw()->rawValue().toDouble();
    }

    _distance += _samples.isEmpty() ? 0 : _samples.last().coordinate.distanceTo(coordinate);
    _samples.append(sample);
    _pathVariant.append(QVariant::fromValue(coordinate));
    emit pathChanged();
}

QList<RouteRecorder::Sample> RouteRecorder::_route() const
{
    QList<QGeoCoordinate> path;
    path.reserve(_samples.count());
    for (const Sample& sample : _samples) {
        path.append(sample.coordinate);
    }
    QList<Sample> route;
    for (const qsizetype index : routeIndices(path)) {
        route.append(_samples[index]);
    }
    return route;
}

bool RouteRecorder::_save() const
{
    const QList<Sample> route = _route();
    return (route.count() >= 2) && saveKml(_fileName, _name, route);
}

QList<qsizetype> RouteRecorder::routeIndices(const QList<QGeoCoordinate>& path)
{
    if (path.isEmpty()) {
        return {};
    }

    const QGeoCoordinate start = path.first();
    const QGeoCoordinate end = path.last();
    const auto firstAway = std::find_if(path.cbegin(), path.cend(), [&](const QGeoCoordinate& coordinate) {
        return start.distanceTo(coordinate) >= kTrimRadiusMeters;
    });
    const auto lastAway = std::find_if(path.crbegin(), path.crend(), [&](const QGeoCoordinate& coordinate) {
        return end.distanceTo(coordinate) >= kTrimRadiusMeters;
    });
    if ((firstAway == path.cend()) || (lastAway == path.crend())) {
        return {};
    }

    const qsizetype first = std::distance(path.cbegin(), firstAway) - 1;
    const qsizetype last = path.count() - std::distance(path.crbegin(), lastAway);
    QList<qsizetype> indices;
    for (const qsizetype index :
         QGCGeo::simplifyPathIndices(path.mid(first, last - first + 1), kSimplifyToleranceMeters)) {
        indices.append(first + index);
    }
    return indices;
}

QList<QGeoCoordinate> RouteRecorder::routeFromPath(const QList<QGeoCoordinate>& path)
{
    QList<QGeoCoordinate> route;
    for (const qsizetype index : routeIndices(path)) {
        route.append(path[index]);
    }
    return route;
}

bool RouteRecorder::saveKml(const QString& fileName, const QString& name, const QList<Sample>& route)
{
    QList<QGeoCoordinate> coordinates;
    QList<double> pitch;
    QList<double> yaw;
    QList<QGeoCoordinate> lookAt;
    for (const Sample& sample : route) {
        coordinates.append(sample.coordinate);
        pitch.append(sample.gimbalPitch);
        yaw.append(sample.gimbalYaw);
        lookAt.append(sample.lookAt);
    }

    KMLDomDocument kml(name);
    QDomElement placemark = kml.addPlacemark(name, true);
    QDomElement extendedData = kml.createElement(QStringLiteral("ExtendedData"));
    const auto addData = [&kml, &extendedData](const char* dataName, const QString& value) {
        QDomElement data = kml.createElement(QStringLiteral("Data"));
        data.setAttribute(QStringLiteral("name"), QLatin1String(dataName));
        kml.addTextElement(data, QStringLiteral("value"), value);
        (void) extendedData.appendChild(data);
    };
    addData(KMLHelper::kRouteGimbalDataName, KMLHelper::routeGimbalData(pitch, yaw));
    addData(KMLHelper::kRouteLookAtDataName, KMLHelper::routeLookAtData(lookAt));
    (void) placemark.appendChild(extendedData);
    (void) kml.addLineString(placemark, coordinates, QStringLiteral("absolute"), false);

    QSaveFile file(fileName);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Text)) {
        return false;
    }
    const QByteArray bytes = kml.toByteArray();
    return (file.write(bytes) == bytes.size()) && file.commit();
}
