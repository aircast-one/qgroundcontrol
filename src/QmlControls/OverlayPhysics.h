#pragma once

#include <QtCore/QList>
#include <QtCore/QMap>
#include <QtCore/QObject>
#include <QtCore/QPointF>
#include <QtCore/QRectF>
#include <QtCore/QSizeF>
#include <QtCore/QVariantList>

class OverlayPhysics : public QObject
{
    Q_OBJECT

    Q_PROPERTY(qreal pull         READ pull         WRITE setPull         NOTIFY tuningChanged)
    Q_PROPERTY(qreal springRadius READ springRadius WRITE setSpringRadius NOTIFY tuningChanged)
    Q_PROPERTY(qreal damping      READ damping      WRITE setDamping      NOTIFY tuningChanged)
    Q_PROPERTY(qreal friction     READ friction     WRITE setFriction     NOTIFY tuningChanged)
    Q_PROPERTY(qreal restitution  READ restitution  WRITE setRestitution  NOTIFY tuningChanged)
    Q_PROPERTY(qreal grid         READ grid         WRITE setGrid         NOTIFY tuningChanged)

public:
    enum Kind { Free, Driven };
    Q_ENUM(Kind)

    explicit OverlayPhysics(QObject *parent = nullptr);

    Q_INVOKABLE int  create(int kind, qreal x, qreal y, qreal w, qreal h, int group = 0);
    Q_INVOKABLE void remove(int id);
    Q_INVOKABLE void setKind(int id, int kind);
    Q_INVOKABLE void setSize(int id, qreal w, qreal h);
    Q_INVOKABLE void drive(int id, qreal x, qreal y, qreal dt);
    Q_INVOKABLE void place(int id, qreal x, qreal y);
    Q_INVOKABLE void setHome(int id, qreal x, qreal y);
    Q_INVOKABLE void setAttachments(int id, const QVariantList &rects);
    Q_INVOKABLE void setWalls(qreal left, qreal top, qreal right, qreal bottom);
    Q_INVOKABLE void step(qreal dt);

    Q_INVOKABLE qreal   x(int id) const;
    Q_INVOKABLE qreal   y(int id) const;
    Q_INVOKABLE bool    asleep(int id) const;
    Q_INVOKABLE bool    allAsleep() const;
    Q_INVOKABLE QString describe(int id) const;
    Q_INVOKABLE QString report() const;
    Q_INVOKABLE QPointF landing(int id, qreal homeX, qreal homeY) const;

    qreal pull() const { return _pull; }
    qreal springRadius() const { return _springRadius; }
    qreal damping() const { return _damping; }
    qreal friction() const { return _friction; }
    qreal restitution() const { return _restitution; }
    qreal grid() const { return _grid; }

    void setPull(qreal value);
    void setSpringRadius(qreal value);
    void setDamping(qreal value);
    void setFriction(qreal value);
    void setRestitution(qreal value);
    void setGrid(qreal value);

signals:
    void tuningChanged();

private:
    struct Body {
        Kind          kind = Free;
        QPointF       pos;
        QPointF       velocity;
        QSizeF        size;
        QPointF       home;
        QPointF       target;
        bool          hasHome = false;
        int           group = 0;
        bool          awake = true;
        int           restSteps = 0;
        QVariantList  attachments;
        QList<QRectF> parts;
    };

    static constexpr qreal kMaxDriveSpeed = 3000.0;
    static constexpr qreal kLayoutJump    = 160.0;
    static constexpr qreal kMinStep       = 1.0;
    static constexpr qreal kRestMotion    = 0.5;
    static constexpr int   kRestSteps     = 2;
    static constexpr qreal kGridStep      = 8.0;
    static constexpr qreal kBehindPenalty = 1.0e6;

    QList<QRectF> _rectsOf(const Body &body, const QPointF &at) const;
    QList<QRectF> _rectsOf(const Body &body) const { return _rectsOf(body, body.pos); }
    QRectF        _footprint(const Body &body, const QPointF &at) const;
    bool          _clear(const Body &body, const QPointF &at, const QList<QRectF> &obstacles) const;
    QPointF       _clampToWalls(const Body &body, const QPointF &at) const;
    QPointF       _targetFor(const Body &body, const QList<QRectF> &obstacles, const QPointF &carry) const;
    QPointF       _snapped(const Body &body, const QPointF &at) const;
    QPointF       _carryOn(const Body &body) const;
    void          _wakeFreeBodies(int except = -1);
    qreal         _mass(const Body &body) const { return qMax(1.0, body.size.width() * body.size.height()); }

    QRectF           _bounds;
    QMap<int, Body>  _bodies;
    int              _nextId = 1;

    qreal _pull         = 9000;
    qreal _springRadius = 40;
    qreal _damping      = 6;
    qreal _friction     = 0.6;
    qreal _restitution  = 0.15;
    qreal _grid         = 0;
};
