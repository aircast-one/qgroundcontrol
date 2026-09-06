#include "OverlayPhysics.h"

#include <QtCore/QStringList>

#include <algorithm>
#include <cmath>
#include <limits>

namespace {

qreal lengthOf(const QPointF &p)
{
    return std::hypot(p.x(), p.y());
}

bool intersects(const QRectF &a, const QRectF &b)
{
    return a.left() < b.right() && b.left() < a.right() && a.top() < b.bottom() && b.top() < a.bottom();
}

}

OverlayPhysics::OverlayPhysics(QObject *parent)
    : QObject(parent)
{
}

int OverlayPhysics::create(int kind, qreal x, qreal y, qreal w, qreal h, int group)
{
    Body body;
    body.kind = static_cast<Kind>(kind);
    body.pos = QPointF(x, y);
    body.target = body.pos;
    body.size = QSizeF(w, h);
    body.group = group;
    const int id = _nextId++;
    _bodies.insert(id, body);
    _wakeFreeBodies();
    return id;
}

void OverlayPhysics::remove(int id)
{
    if (_bodies.remove(id) > 0) {
        _wakeFreeBodies();
    }
}

void OverlayPhysics::setKind(int id, int kind)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end() || it->kind == static_cast<Kind>(kind)) {
        return;
    }
    it->kind = static_cast<Kind>(kind);
    it->velocity = QPointF();
    _wakeFreeBodies();
}

void OverlayPhysics::setSize(int id, qreal w, qreal h)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end() || (qFuzzyCompare(it->size.width(), w) && qFuzzyCompare(it->size.height(), h))) {
        return;
    }
    const QPointF centre = it->pos + QPointF(it->size.width() / 2, it->size.height() / 2);
    it->size = QSizeF(w, h);
    place(id, centre.x() - (w / 2), centre.y() - (h / 2));
}

void OverlayPhysics::drive(int id, qreal x, qreal y, qreal dt)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end() || dt <= 0) {
        return;
    }
    const QPointF target(x, y);
    const QPointF delta = target - it->pos;
    const qreal jump = lengthOf(delta);
    if (jump > kLayoutJump) {
        place(id, x, y);
        return;
    }
    if (jump < 1.0) {
        it->pos = target;
        it->velocity = QPointF();
        return;
    }
    const qreal limit = kMaxDriveSpeed * dt;
    const QPointF motion = jump > limit ? delta * (limit / jump) : delta;
    it->pos += motion;
    it->velocity = motion / dt;
    _wakeFreeBodies(id);
}

void OverlayPhysics::place(int id, qreal x, qreal y)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end()) {
        return;
    }
    it->pos = QPointF(x, y);
    it->velocity = QPointF();
    _wakeFreeBodies();
}

void OverlayPhysics::setHome(int id, qreal x, qreal y)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end()) {
        return;
    }
    const QPointF home(x, y);
    if (it->hasHome && home == it->home) {
        return;
    }
    it->home = home;
    it->hasHome = true;
    _wakeFreeBodies();
}

void OverlayPhysics::setAttachments(int id, const QVariantList &rects)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end() || it->attachments == rects) {
        return;
    }
    it->attachments = rects;
    it->parts.clear();
    for (const QVariant &value : rects) {
        const QVariantMap rect = value.toMap();
        it->parts.append(QRectF(rect.value(QStringLiteral("x")).toReal(), rect.value(QStringLiteral("y")).toReal(),
                                rect.value(QStringLiteral("w")).toReal(), rect.value(QStringLiteral("h")).toReal()));
    }
    _wakeFreeBodies();
}

void OverlayPhysics::setWalls(qreal left, qreal top, qreal right, qreal bottom)
{
    _bounds = QRectF(QPointF(left, top), QPointF(right, bottom));
    _wakeFreeBodies();
}

QList<QRectF> OverlayPhysics::_rectsOf(const Body &body, const QPointF &at) const
{
    QList<QRectF> rects;
    rects.append(QRectF(at, body.size));
    for (const QRectF &part : body.parts) {
        rects.append(part.translated(at));
    }
    return rects;
}

QRectF OverlayPhysics::_footprint(const Body &body, const QPointF &at) const
{
    QRectF footprint(at, body.size);
    for (const QRectF &part : body.parts) {
        footprint |= part.translated(at);
    }
    return footprint;
}

bool OverlayPhysics::_clear(const Body &body, const QPointF &at, const QList<QRectF> &obstacles) const
{
    if (!_bounds.isNull() && !_bounds.contains(_footprint(body, at))) {
        return false;
    }
    const QList<QRectF> mine = _rectsOf(body, at);
    return std::none_of(mine.cbegin(), mine.cend(), [&](const QRectF &a) {
        return std::any_of(obstacles.cbegin(), obstacles.cend(), [&](const QRectF &b) { return intersects(a, b); });
    });
}

QPointF OverlayPhysics::_clampToWalls(const Body &body, const QPointF &at) const
{
    if (_bounds.isNull()) {
        return at;
    }
    const QRectF footprint = _footprint(body, at);
    QPointF shift;
    if (footprint.width() >= _bounds.width() || footprint.left() < _bounds.left()) {
        shift.setX(_bounds.left() - footprint.left());
    } else if (footprint.right() > _bounds.right()) {
        shift.setX(_bounds.right() - footprint.right());
    }
    if (footprint.height() >= _bounds.height() || footprint.top() < _bounds.top()) {
        shift.setY(_bounds.top() - footprint.top());
    } else if (footprint.bottom() > _bounds.bottom()) {
        shift.setY(_bounds.bottom() - footprint.bottom());
    }
    return at + shift;
}

QPointF OverlayPhysics::_carryOn(const Body &body) const
{
    QPointF carry;
    const QList<QRectF> mine = _rectsOf(body);
    for (const Body &other : _bodies) {
        if (other.kind != Driven || lengthOf(other.velocity) < 1.0) {
            continue;
        }
        const QList<QRectF> theirs = _rectsOf(other);
        const bool touching = std::any_of(mine.cbegin(), mine.cend(), [&](const QRectF &a) {
            return std::any_of(theirs.cbegin(), theirs.cend(), [&](const QRectF &b) { return intersects(a, b); });
        });
        if (touching) {
            carry += other.velocity;
        }
    }
    return carry;
}

QPointF OverlayPhysics::_snapped(const Body &body, const QPointF &at) const
{
    if (_grid <= 0 || _bounds.isNull()) {
        return at;
    }
    const QRectF footprint = _footprint(body, at);
    const auto axis = [this](qreal low, qreal high, qreal pos, qreal extent) {
        const qreal maxPos = qMax(low, high - extent);
        const bool nearFarEdge = pos + (extent / 2) > (low + high) / 2;
        const qreal snapped = nearFarEdge ? maxPos - std::round((maxPos - pos) / _grid) * _grid
                                          : low + std::round((pos - low) / _grid) * _grid;
        return qBound(low, snapped, maxPos);
    };
    return QPointF(axis(_bounds.left(), _bounds.right(), footprint.left(), footprint.width()),
                   axis(_bounds.top(), _bounds.bottom(), footprint.top(), footprint.height()))
           + (at - footprint.topLeft());
}

QPointF OverlayPhysics::_targetFor(const Body &body, const QList<QRectF> &obstacles, const QPointF &carry) const
{
    const QPointF home = _clampToWalls(body, body.home);
    if (_clear(body, home, obstacles)) {
        return home;
    }
    const QRectF footprint = _footprint(body, home);
    const QPointF offset = home - footprint.topLeft();
    const qreal w = footprint.width();
    const qreal h = footprint.height();

    QList<qreal> xs { footprint.left() };
    QList<qreal> ys { footprint.top() };
    for (const QRectF &r : obstacles) {
        xs.append(r.right());
        xs.append(r.left() - w);
        ys.append(r.bottom());
        ys.append(r.top() - h);
    }

    QPointF best = home;
    qreal bestDistance = std::numeric_limits<qreal>::max();
    const auto consider = [&](const QPointF &candidate) {
        const QPointF at = _clampToWalls(body, candidate + offset);
        const bool behind = QPointF::dotProduct(at - body.pos, carry) < 0;
        const qreal distance = lengthOf(at - home) + lengthOf(at - body.pos) + (behind ? kBehindPenalty : 0.0);
        if (distance < bestDistance && _clear(body, at, obstacles)) {
            best = at;
            bestDistance = distance;
        }
    };
    for (const qreal cx : xs) {
        for (const qreal cy : ys) {
            consider(QPointF(cx, cy));
        }
    }
    if (bestDistance == std::numeric_limits<qreal>::max() && !_bounds.isNull()) {
        for (qreal cy = _bounds.top(); cy + h <= _bounds.bottom(); cy += kGridStep) {
            for (qreal cx = _bounds.left(); cx + w <= _bounds.right(); cx += kGridStep) {
                consider(QPointF(cx, cy));
            }
        }
    }
    const QPointF aligned = _snapped(body, best);
    return _clear(body, aligned, obstacles) ? aligned : best;
}

void OverlayPhysics::step(qreal dt)
{
    QList<int> order;
    QList<QRectF> obstacles;
    for (auto it = _bodies.constBegin(); it != _bodies.constEnd(); ++it) {
        if (it->kind == Driven) {
            obstacles.append(_rectsOf(*it));
        } else {
            order.append(it.key());
        }
    }
    std::stable_sort(order.begin(), order.end(), [this](int a, int b) {
        const Body &first = _bodies[a];
        const Body &second = _bodies[b];
        return first.priority != second.priority ? first.priority > second.priority : _mass(first) > _mass(second);
    });

    const qreal omega = qMax(1.0, _damping) / (2.0 * kDampingRatio);
    QMap<int, QPointF> before;
    for (const int id : order) {
        Body &body = _bodies[id];
        before.insert(id, body.pos);
        const QPointF carry = _carryOn(body);
        body.target = body.hasHome ? _targetFor(body, obstacles, carry) : body.pos;
        obstacles.append(_rectsOf(body, body.target));
        if (!body.awake) {
            continue;
        }
        const QPointF delta = body.target - body.pos;
        if (!carry.isNull() || (lengthOf(delta) < kMinStep && lengthOf(body.velocity) < kRestSpeed)) {
            body.pos = body.target;
            body.velocity = QPointF();
            continue;
        }
        body.velocity += (delta * (omega * omega) - body.velocity * (2.0 * kDampingRatio * omega)) * dt;
        const qreal speed = lengthOf(body.velocity);
        if (speed > kMaxSpeed) {
            body.velocity *= kMaxSpeed / speed;
        }
        body.pos += body.velocity * dt;
    }

    for (const int id : order) {
        Body &body = _bodies[id];
        const QPointF moved = body.pos - before.value(id);
        const bool arrived = lengthOf(body.target - body.pos) <= kRestMotion && lengthOf(body.velocity) < kRestSpeed;
        if (arrived && lengthOf(moved) <= kRestMotion) {
            body.restSteps += 1;
        } else {
            body.restSteps = 0;
            body.awake = true;
        }
        if (body.restSteps >= kRestSteps) {
            body.pos = body.target;
            body.velocity = QPointF();
            body.awake = false;
        }
    }
}

void OverlayPhysics::_wakeFreeBodies(int except)
{
    for (auto it = _bodies.begin(); it != _bodies.end(); ++it) {
        if (it->kind == Free && it.key() != except) {
            it->awake = true;
            it->restSteps = 0;
        }
    }
}

qreal OverlayPhysics::x(int id) const
{
    const auto it = _bodies.constFind(id);
    return it == _bodies.constEnd() ? 0 : it->pos.x();
}

qreal OverlayPhysics::y(int id) const
{
    const auto it = _bodies.constFind(id);
    return it == _bodies.constEnd() ? 0 : it->pos.y();
}

bool OverlayPhysics::asleep(int id) const
{
    const auto it = _bodies.constFind(id);
    return it == _bodies.constEnd() ? true : !it->awake;
}

bool OverlayPhysics::allAsleep() const
{
    return std::all_of(_bodies.cbegin(), _bodies.cend(), [](const Body &body) {
        return body.kind == Free ? !body.awake : lengthOf(body.velocity) <= 1.0;
    });
}

QString OverlayPhysics::describe(int id) const
{
    const auto it = _bodies.constFind(id);
    if (it == _bodies.constEnd()) {
        return QStringLiteral("?");
    }
    return QStringLiteral("%1[%9] pos %2,%3 vel %4,%5 home %6,%7 %8")
        .arg(it->kind == Free ? QStringLiteral("free") : QStringLiteral("driven"))
        .arg(it->pos.x(), 0, 'f', 0).arg(it->pos.y(), 0, 'f', 0)
        .arg(it->velocity.x(), 0, 'f', 0).arg(it->velocity.y(), 0, 'f', 0)
        .arg(it->home.x(), 0, 'f', 0).arg(it->home.y(), 0, 'f', 0)
        .arg(it->awake ? QStringLiteral("awake") : QStringLiteral("asleep"))
        .arg(it->kind == Free ? QStringLiteral("dyn") : QStringLiteral("kin"));
}

QPointF OverlayPhysics::landing(int id, qreal homeX, qreal homeY, qreal w, qreal h) const
{
    const auto it = _bodies.constFind(id);
    if (it == _bodies.constEnd()) {
        return QPointF(homeX, homeY);
    }
    Body probe = *it;
    probe.home = QPointF(homeX, homeY);
    probe.hasHome = true;
    probe.size = QSizeF(w, h);
    QList<QRectF> obstacles;
    for (auto other = _bodies.constBegin(); other != _bodies.constEnd(); ++other) {
        if (other.key() != id && other->kind == Driven && (probe.group == 0 || probe.group != other->group)) {
            obstacles.append(_rectsOf(*other));
        }
    }
    return _targetFor(probe, obstacles, QPointF());
}

void OverlayPhysics::touch(int id)
{
    auto it = _bodies.find(id);
    if (it != _bodies.end()) {
        it->priority = ++_touches;
        _wakeFreeBodies();
    }
}

QString OverlayPhysics::report() const
{
    QStringList lines;
    for (auto it = _bodies.constBegin(); it != _bodies.constEnd(); ++it) {
        const Body &body = it.value();
        QString line = QStringLiteral("#%1 %2 rect %3,%4 %5x%6 %7 target %8,%9 home %10,%11 priority %12")
            .arg(it.key())
            .arg(body.kind == Free ? QStringLiteral("free") : QStringLiteral("driven"))
            .arg(body.pos.x(), 0, 'f', 0).arg(body.pos.y(), 0, 'f', 0)
            .arg(body.size.width(), 0, 'f', 0).arg(body.size.height(), 0, 'f', 0)
            .arg(body.awake ? QStringLiteral("awake") : QStringLiteral("asleep"))
            .arg(body.target.x(), 0, 'f', 0).arg(body.target.y(), 0, 'f', 0)
            .arg(body.home.x(), 0, 'f', 0).arg(body.home.y(), 0, 'f', 0)
            .arg(body.priority);
        for (const QRectF &part : body.parts) {
            const QRectF r = part.translated(body.pos);
            line += QStringLiteral(" +attached %1,%2 %3x%4").arg(r.x(), 0, 'f', 0).arg(r.y(), 0, 'f', 0).arg(r.width(), 0, 'f', 0).arg(r.height(), 0, 'f', 0);
        }
        lines.append(line);
    }
    return lines.join(QLatin1Char('\n'));
}

void OverlayPhysics::setPull(qreal value)         { if (!qFuzzyCompare(_pull, value))         { _pull = value;         emit tuningChanged(); } }
void OverlayPhysics::setSpringRadius(qreal value) { if (!qFuzzyCompare(_springRadius, value)) { _springRadius = value; emit tuningChanged(); } }
void OverlayPhysics::setDamping(qreal value)      { if (!qFuzzyCompare(_damping, value))      { _damping = value;      emit tuningChanged(); } }
void OverlayPhysics::setFriction(qreal value)     { if (!qFuzzyCompare(_friction, value))     { _friction = value;     emit tuningChanged(); } }
void OverlayPhysics::setRestitution(qreal value)  { if (!qFuzzyCompare(_restitution, value))  { _restitution = value;  emit tuningChanged(); } }
void OverlayPhysics::setGrid(qreal value)         { if (!qFuzzyCompare(_grid, value))         { _grid = value;         _wakeFreeBodies(); emit tuningChanged(); } }
