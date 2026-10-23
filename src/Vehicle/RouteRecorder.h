#pragma once

#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtCore/QTimer>
#include <QtCore/QVariantList>
#include <QtPositioning/QGeoCoordinate>
#include <QtQmlIntegration/QtQmlIntegration>

class Vehicle;

class RouteRecorder : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    Q_MOC_INCLUDE("Vehicle.h")
    Q_PROPERTY(bool recording READ recording NOTIFY recordingChanged)
    Q_PROPERTY(double distance READ distance NOTIFY pathChanged)
    Q_PROPERTY(QVariantList path READ path NOTIFY pathChanged)

public:
    struct Sample
    {
        QGeoCoordinate coordinate;
        double gimbalPitch = qQNaN();
        double gimbalYaw = qQNaN();
        QGeoCoordinate lookAt;
    };

    explicit RouteRecorder(QObject* parent = nullptr);

    bool recording() const { return _recording; }

    double distance() const { return _distance; }

    QVariantList path() const { return _pathVariant; }

    Q_INVOKABLE void start(Vehicle* vehicle);
    Q_INVOKABLE void stop();
    Q_INVOKABLE QString renameFile(const QString& fileName, const QString& newName);

    static QList<qsizetype> routeIndices(const QList<QGeoCoordinate>& path);
    static QList<QGeoCoordinate> routeFromPath(const QList<QGeoCoordinate>& path);
    static bool saveKml(const QString& fileName, const QString& name, const QList<Sample>& route);

    static constexpr double kMinSpacingMeters = 2.0;
    static constexpr double kSimplifyToleranceMeters = 3.0;
    static constexpr double kTrimRadiusMeters = 10.0;
    static constexpr int kSaveIntervalMs = 10000;

signals:
    void recordingChanged();
    void pathChanged();
    void finished(const QString& fileName, const QString& message);

private:
    void _addCoordinate(const QGeoCoordinate& coordinate);
    void _lookAtChanged(const QGeoCoordinate& center);
    QList<Sample> _route() const;
    bool _save() const;

    QPointer<Vehicle> _vehicle;
    bool _recording = false;
    QList<Sample> _samples;
    QVariantList _pathVariant;
    QGeoCoordinate _lookAt;
    double _distance = 0;
    QString _name;
    QString _fileName;
    QTimer _saveTimer;
};
