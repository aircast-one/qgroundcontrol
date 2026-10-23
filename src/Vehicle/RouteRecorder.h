#pragma once

#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtPositioning/QGeoCoordinate>
#include <QtQmlIntegration/QtQmlIntegration>

class Vehicle;

class RouteRecorder : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    Q_MOC_INCLUDE("Vehicle.h")
    Q_PROPERTY(bool recording READ recording NOTIFY recordingChanged)

public:
    explicit RouteRecorder(QObject* parent = nullptr);

    bool recording() const { return _recording; }

    Q_INVOKABLE void start(Vehicle* vehicle);
    Q_INVOKABLE QString stop();

    static QList<QGeoCoordinate> simplify(const QList<QGeoCoordinate>& path, double toleranceMeters);
    static bool saveKml(const QString& fileName, const QString& name, const QList<QGeoCoordinate>& route);

    static constexpr double kMinSpacingMeters = 2.0;
    static constexpr double kSimplifyToleranceMeters = 3.0;

signals:
    void recordingChanged();

private:
    void _addCoordinate(const QGeoCoordinate& coordinate);

    QPointer<Vehicle> _vehicle;
    bool _recording = false;
    QList<QGeoCoordinate> _path;
};
