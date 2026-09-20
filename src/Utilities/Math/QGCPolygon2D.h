#pragma once

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtCore/QRectF>

namespace QGCPolygon2D
{

/// Even-odd point-in-polygon test over an implicitly closed polygon.
bool containsPoint(const QList<QPointF> &polygon, const QPointF &point);

/// Axis-aligned bounds of @p polygon; a null rect when it is empty.
QRectF boundingRect(const QList<QPointF> &polygon);

} // namespace QGCPolygon2D
