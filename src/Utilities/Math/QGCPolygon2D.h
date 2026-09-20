#pragma once

#include <QtCore/QList>
#include <QtCore/QPointF>
#include <QtCore/QRectF>

namespace QGCPolygon2D
{

bool containsPoint(const QList<QPointF> &polygon, const QPointF &point);

QRectF boundingRect(const QList<QPointF> &polygon);

} // namespace QGCPolygon2D
