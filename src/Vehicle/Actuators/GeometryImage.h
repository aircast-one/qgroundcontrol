#pragma once

#include "Common.h"

#include <QtCore/QObject>
#include <QtCore/QSize>
#include <QtCore/QString>
#ifndef QGC_HEADLESS_CORE
#include "QGCPalette.h"
#include <QtGui/QPainter>
#include <QtQuick/QQuickImageProvider>
#endif

namespace GeometryImage {

/**
 * Renders an image of an airframe geometry (currently only multirotor)
 */
#ifdef QGC_HEADLESS_CORE
class VehicleGeometryImageProvider
#else
class VehicleGeometryImageProvider : public QQuickImageProvider
#endif
{
public:

    struct ImagePosition {
        ActuatorGeometry::Type type;
        int index;
        QPointF position;
        float radius;
    };

#ifndef QGC_HEADLESS_CORE
    void drawAxisIndicator(QPainter& p, const QPointF& origin, float fontSize, const QColor& color);

    QPixmap requestPixmap(const QString& id, QSize* size, const QSize& requestedSize) override;
#endif

    static VehicleGeometryImageProvider* instance();

    int getHighlightedMotorIndexAtPos(const QSizeF& displaySize, const QPointF& position);

    QList<ActuatorGeometry>& actuators() { return _actuators; }

    int numMotors() const;

private:
    VehicleGeometryImageProvider();
    ~VehicleGeometryImageProvider() = default;

    QList<ActuatorGeometry> _actuators{};

    QSize                   _imageSize;                 ///< size of the image requested, used to scale click positions
    QList<ImagePosition>    _actuatorImagePositions{};  ///< highlighted actuators image positions
#ifndef QGC_HEADLESS_CORE
    QGCPalette              _palette;
#endif
};

} // namespace GeometryImage
