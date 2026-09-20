#include "QGCQGeoCoordinate.h"

#include "QGCCppOwnership.h"

QGCQGeoCoordinate::QGCQGeoCoordinate(const QGeoCoordinate& coord, QObject* parent)
    : QObject       (parent)
    , _coordinate   (coord)
    , _dirty        (false)
{
    qgcCppOwnership(this);
}

void QGCQGeoCoordinate::setCoordinate(const QGeoCoordinate& coordinate)
{
    if (_coordinate != coordinate) {
        _coordinate = coordinate;
        emit coordinateChanged(coordinate);
        setDirty(true);
    }
}

void QGCQGeoCoordinate::setDirty(bool dirty)
{
    if (_dirty != dirty) {
        _dirty = dirty;
        emit dirtyChanged(dirty);
    }
}
