#include "QGCPolygon2D.h"

#include <QtCore/QtNumeric>

#include <algorithm>

namespace
{

void windOverEdge(const QPointF &from, const QPointF &to, const QPointF &point, int &winding)
{
    qreal x1 = from.x();
    qreal y1 = from.y();
    qreal x2 = to.x();
    qreal y2 = to.y();
    const qreal y = point.y();
    int direction = 1;

    if (qFuzzyCompare(y1, y2)) {
        return;
    }

    if (y2 < y1) {
        std::swap(x1, x2);
        std::swap(y1, y2);
        direction = -1;
    }

    if ((y >= y1) && (y < y2)) {
        const qreal x = x1 + ((x2 - x1) / (y2 - y1)) * (y - y1);
        if (x <= point.x()) {
            winding += direction;
        }
    }
}

} // namespace

bool QGCPolygon2D::containsPoint(const QList<QPointF> &polygon, const QPointF &point)
{
    if (polygon.isEmpty()) {
        return false;
    }

    int winding = 0;
    QPointF previous = polygon.first();
    for (qsizetype i = 1; i < polygon.count(); ++i) {
        windOverEdge(previous, polygon.at(i), point, winding);
        previous = polygon.at(i);
    }

    if (previous != polygon.first()) {
        windOverEdge(previous, polygon.first(), point, winding);
    }

    return (winding % 2) != 0;
}

QRectF QGCPolygon2D::boundingRect(const QList<QPointF> &polygon)
{
    if (polygon.isEmpty()) {
        return QRectF();
    }

    qreal minX = polygon.first().x();
    qreal maxX = minX;
    qreal minY = polygon.first().y();
    qreal maxY = minY;

    for (const QPointF &point : polygon) {
        minX = qMin(minX, point.x());
        maxX = qMax(maxX, point.x());
        minY = qMin(minY, point.y());
        maxY = qMax(maxY, point.y());
    }

    return QRectF(QPointF(minX, minY), QPointF(maxX, maxY));
}
