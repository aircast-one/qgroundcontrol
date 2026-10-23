#pragma once

#include <QtCore/QList>
#include <QtPositioning/QGeoCoordinate>

#include "ShapeFileHelper.h"

namespace KMLHelper
{
    ShapeFileHelper::ShapeType determineShapeType(const QString &file, QString &errorString);

    /// Get the number of geometry entities in the KML file
    int getEntityCount(const QString &kmlFile, QString &errorString);

    /// Load all polygon entities
    /// @param filterMeters Filter vertices closer than this distance (0 to disable)
    bool loadPolygonsFromFile(const QString &kmlFile, QList<QList<QGeoCoordinate>> &polygons, QString &errorString,
                              double filterMeters = ShapeFileHelper::kDefaultVertexFilterMeters);

    /// Load all polyline entities
    /// @param filterMeters Filter vertices closer than this distance (0 to disable)
    bool loadPolylinesFromFile(const QString &kmlFile, QList<QList<QGeoCoordinate>> &polylines, QString &errorString,
                               double filterMeters = ShapeFileHelper::kDefaultVertexFilterMeters);

    /// Load all point entities
    bool loadPointsFromFile(const QString &kmlFile, QList<QGeoCoordinate> &points, QString &errorString);

    struct Route
    {
        enum class Altitude
        {
            Ground,
            AboveGround,
            Absolute
        };

        QList<QGeoCoordinate> path;
        Altitude altitude = Altitude::Ground;
        int pathCount = 0;
        int ignoredPointCount = 0;
        qsizetype sourceVertexCount = 0;
    };

    bool loadRouteFromFile(const QString& kmlFile, Route& route, QString& errorString);

    constexpr qsizetype kDenseTrackVertices = 200;
    constexpr double kDenseTrackToleranceMeters = 3.0;
}
