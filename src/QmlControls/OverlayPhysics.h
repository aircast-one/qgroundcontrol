#pragma once

#include <QtCore/QHash>
#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QPointF>
#include <QtCore/QRectF>
#include <QtCore/QSizeF>
#include <QtCore/QVariantList>

#include <box2d/id.h>

class OverlayPhysics : public QObject
{
    Q_OBJECT

    Q_PROPERTY(qreal pull         READ pull         WRITE setPull         NOTIFY tuningChanged)
    Q_PROPERTY(qreal springRadius READ springRadius WRITE setSpringRadius NOTIFY tuningChanged)
    Q_PROPERTY(qreal damping      READ damping      WRITE setDamping      NOTIFY tuningChanged)
    Q_PROPERTY(qreal friction     READ friction     WRITE setFriction     NOTIFY tuningChanged)
    Q_PROPERTY(qreal restitution  READ restitution  WRITE setRestitution  NOTIFY tuningChanged)

public:
    enum Kind { Free, Driven };
    Q_ENUM(Kind)

    explicit OverlayPhysics(QObject *parent = nullptr);
    ~OverlayPhysics() override;

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

    qreal pull() const { return _pull; }
    qreal springRadius() const { return _springRadius; }
    qreal damping() const { return _damping; }
    qreal friction() const { return _friction; }
    qreal restitution() const { return _restitution; }

    void setPull(qreal value);
    void setSpringRadius(qreal value);
    void setDamping(qreal value);
    void setFriction(qreal value);
    void setRestitution(qreal value);

signals:
    void tuningChanged();

private:
    struct Body {
        b2BodyId         body{};
        b2ShapeId        shape{};
        Kind             kind = Free;
        QSizeF           size;
        QPointF          home;
        bool             hasHome = false;
        int              group = 0;
        bool             escaping = false;
        QVariantList     attachments;
        QList<b2ShapeId> extraShapes;
    };

    static constexpr qreal kPixelsPerMetre = 100.0;
    static constexpr qreal kMaxDriveSpeed  = 3000.0;
    static constexpr qreal kLayoutJump     = 160.0;
    static constexpr qreal kEscapeSpeed    = 900.0;

    void    _rebuildShape(Body &body);
    void    _applyPull(Body &body);
    void    _settleAtHome(Body &body);
    bool    _touching(const Body &body) const;
    bool    _applyEscape(Body &body);
    void    _wakeFreeBodies(const Body &mover);
    qreal   _vx(int id) const;
    qreal   _vy(int id) const;
    QPointF _centre(const Body &body) const;

    b2WorldId        _world{};
    b2BodyId         _walls{};
    QRectF           _bounds;
    QHash<int, Body> _bodies;
    int              _nextId = 1;

    qreal _pull         = 9000;
    qreal _springRadius = 40;
    qreal _damping      = 6;
    qreal _friction     = 0.6;
    qreal _restitution  = 0.15;
};
