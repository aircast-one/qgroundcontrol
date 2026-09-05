#include "OverlayPhysics.h"

#include <box2d/box2d.h>

#include <algorithm>
#include <cmath>

namespace {

constexpr int kSubSteps = 4;

b2Vec2 toMetres(qreal xPixels, qreal yPixels, qreal scale)
{
    return b2Vec2{ static_cast<float>(xPixels / scale), static_cast<float>(yPixels / scale) };
}

} // namespace

OverlayPhysics::OverlayPhysics(QObject *parent)
    : QObject(parent)
{
    b2WorldDef def = b2DefaultWorldDef();
    def.gravity = b2Vec2{ 0.0f, 0.0f };
    def.enableSleep = true;
    def.enableContinous = true;
    def.contactHertz = 120.0f;
    def.contactDampingRatio = 10.0f;
    def.contactPushoutVelocity = 6.0f;
    def.maximumLinearVelocity = 40.0f;
    _world = b2CreateWorld(&def);

    b2BodyDef wallDef = b2DefaultBodyDef();
    wallDef.type = b2_staticBody;
    _walls = b2CreateBody(_world, &wallDef);
}

OverlayPhysics::~OverlayPhysics()
{
    b2DestroyWorld(_world);
}

int OverlayPhysics::create(int kind, qreal x, qreal y, qreal w, qreal h, int group)
{
    Body body;
    body.kind = static_cast<Kind>(kind);
    body.size = QSizeF(w, h);
    body.group = group;

    b2BodyDef def = b2DefaultBodyDef();
    def.type = body.kind == Free ? b2_dynamicBody : b2_kinematicBody;
    def.position = toMetres(x + (w / 2), y + (h / 2), kPixelsPerMetre);
    def.fixedRotation = true;
    def.isBullet = true;
    def.linearDamping = static_cast<float>(_damping);
    body.body = b2CreateBody(_world, &def);

    _rebuildShape(body);

    const int id = _nextId++;
    _bodies.insert(id, body);
    return id;
}

void OverlayPhysics::remove(int id)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end()) {
        return;
    }
    b2DestroyBody(it->body);
    _bodies.erase(it);
}

void OverlayPhysics::setKind(int id, int kind)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end() || it->kind == static_cast<Kind>(kind)) {
        return;
    }
    it->kind = static_cast<Kind>(kind);
    b2Body_SetType(it->body, it->kind == Free ? b2_dynamicBody : b2_kinematicBody);
    b2Body_SetLinearVelocity(it->body, b2Vec2{ 0.0f, 0.0f });
    b2Body_SetAwake(it->body, true);
}

void OverlayPhysics::setSize(int id, qreal w, qreal h)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end() || (qFuzzyCompare(it->size.width(), w) && qFuzzyCompare(it->size.height(), h))) {
        return;
    }
    const QPointF centre = _centre(*it);
    it->size = QSizeF(w, h);
    _rebuildShape(*it);
    place(id, centre.x() - (w / 2), centre.y() - (h / 2));
}

void OverlayPhysics::drive(int id, qreal x, qreal y, qreal dt)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end() || dt <= 0) {
        return;
    }
    const b2Vec2 target = toMetres(x + (it->size.width() / 2), y + (it->size.height() / 2), kPixelsPerMetre);
    const b2Vec2 now = b2Body_GetPosition(it->body);

    const qreal jump = std::hypot((target.x - now.x) * kPixelsPerMetre, (target.y - now.y) * kPixelsPerMetre);
    if (jump > kLayoutJump) {
        place(id, x, y);
        return;
    }

    if (jump < 1.0) {
        const b2Vec2 velocity = b2Body_GetLinearVelocity(it->body);
        if (velocity.x != 0.0f || velocity.y != 0.0f) {
            b2Body_SetTransform(it->body, target, b2Rot_identity);
            b2Body_SetLinearVelocity(it->body, b2Vec2{ 0.0f, 0.0f });
        }
        return;
    }

    b2Vec2 velocity{ static_cast<float>((target.x - now.x) / dt), static_cast<float>((target.y - now.y) / dt) };
    const float speed = std::hypot(velocity.x, velocity.y);
    const float limit = static_cast<float>(kMaxDriveSpeed / kPixelsPerMetre);
    if (speed > limit) {
        velocity.x *= limit / speed;
        velocity.y *= limit / speed;
    }
    b2Body_SetLinearVelocity(it->body, velocity);
    if (velocity.x != 0.0f || velocity.y != 0.0f) {
        b2Body_SetAwake(it->body, true);
        _wakeFreeBodies(*it);
    }
}

void OverlayPhysics::place(int id, qreal x, qreal y)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end()) {
        return;
    }
    b2Body_SetTransform(it->body, toMetres(x + (it->size.width() / 2), y + (it->size.height() / 2), kPixelsPerMetre), b2Rot_identity);
    b2Body_SetLinearVelocity(it->body, b2Vec2{ 0.0f, 0.0f });
    _wakeFreeBodies(*it);
}

void OverlayPhysics::setHome(int id, qreal x, qreal y)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end()) {
        return;
    }
    const QPointF home(x + (it->size.width() / 2), y + (it->size.height() / 2));
    if (it->hasHome && home == it->home) {
        return;
    }
    it->home = home;
    it->hasHome = true;
    b2Body_SetAwake(it->body, true);
}

void OverlayPhysics::setAttachments(int id, const QVariantList &rects)
{
    auto it = _bodies.find(id);
    if (it == _bodies.end() || it->attachments == rects) {
        return;
    }
    it->attachments = rects;
    for (const b2ShapeId shape : it->extraShapes) {
        b2DestroyShape(shape);
    }
    it->extraShapes.clear();

    b2ShapeDef shapeDef = b2DefaultShapeDef();
    shapeDef.density = 1.0f;
    shapeDef.friction = static_cast<float>(_friction);
    shapeDef.restitution = static_cast<float>(_restitution);
    shapeDef.filter.groupIndex = it->group;

    for (const QVariant &value : rects) {
        const QVariantMap rect = value.toMap();
        const qreal w = rect.value(QStringLiteral("w")).toReal();
        const qreal h = rect.value(QStringLiteral("h")).toReal();
        const qreal cx = rect.value(QStringLiteral("x")).toReal() + (w / 2) - (it->size.width() / 2);
        const qreal cy = rect.value(QStringLiteral("y")).toReal() + (h / 2) - (it->size.height() / 2);
        const b2Polygon box = b2MakeOffsetBox(static_cast<float>(w / 2 / kPixelsPerMetre),
                                              static_cast<float>(h / 2 / kPixelsPerMetre),
                                              toMetres(cx, cy, kPixelsPerMetre), 0.0f);
        it->extraShapes.append(b2CreatePolygonShape(it->body, &shapeDef, &box));
    }
    b2Body_SetAwake(it->body, true);
}

void OverlayPhysics::setWalls(qreal left, qreal top, qreal right, qreal bottom)
{
    _bounds = QRectF(QPointF(left, top), QPointF(right, bottom));
    b2DestroyBody(_walls);
    b2BodyDef wallDef = b2DefaultBodyDef();
    wallDef.type = b2_staticBody;
    _walls = b2CreateBody(_world, &wallDef);

    const float thickness = 5.0f;
    const b2Vec2 tl = toMetres(left, top, kPixelsPerMetre);
    const b2Vec2 br = toMetres(right, bottom, kPixelsPerMetre);
    const float midX = (tl.x + br.x) / 2;
    const float midY = (tl.y + br.y) / 2;
    const float halfW = (br.x - tl.x) / 2;
    const float halfH = (br.y - tl.y) / 2;

    b2ShapeDef shapeDef = b2DefaultShapeDef();
    shapeDef.friction = static_cast<float>(_friction);

    const b2Polygon topWall    = b2MakeOffsetBox(halfW + thickness, thickness, b2Vec2{ midX, tl.y - thickness }, 0.0f);
    const b2Polygon bottomWall = b2MakeOffsetBox(halfW + thickness, thickness, b2Vec2{ midX, br.y + thickness }, 0.0f);
    const b2Polygon leftWall   = b2MakeOffsetBox(thickness, halfH + thickness, b2Vec2{ tl.x - thickness, midY }, 0.0f);
    const b2Polygon rightWall  = b2MakeOffsetBox(thickness, halfH + thickness, b2Vec2{ br.x + thickness, midY }, 0.0f);
    b2CreatePolygonShape(_walls, &shapeDef, &topWall);
    b2CreatePolygonShape(_walls, &shapeDef, &bottomWall);
    b2CreatePolygonShape(_walls, &shapeDef, &leftWall);
    b2CreatePolygonShape(_walls, &shapeDef, &rightWall);
}

void OverlayPhysics::step(qreal dt)
{
    for (auto &body : _bodies) {
        if (body.kind != Free || !body.hasHome) {
            continue;
        }
        if (!_applyEscape(body)) {
            _applyPull(body);
        }
    }
    b2World_Step(_world, static_cast<float>(dt), kSubSteps);
}

qreal OverlayPhysics::x(int id) const
{
    const auto it = _bodies.constFind(id);
    return it == _bodies.constEnd() ? 0 : _centre(*it).x() - (it->size.width() / 2);
}

qreal OverlayPhysics::y(int id) const
{
    const auto it = _bodies.constFind(id);
    return it == _bodies.constEnd() ? 0 : _centre(*it).y() - (it->size.height() / 2);
}

qreal OverlayPhysics::_vx(int id) const
{
    const auto it = _bodies.constFind(id);
    return it == _bodies.constEnd() ? 0 : b2Body_GetLinearVelocity(it->body).x * kPixelsPerMetre;
}

qreal OverlayPhysics::_vy(int id) const
{
    const auto it = _bodies.constFind(id);
    return it == _bodies.constEnd() ? 0 : b2Body_GetLinearVelocity(it->body).y * kPixelsPerMetre;
}

bool OverlayPhysics::asleep(int id) const
{
    const auto it = _bodies.constFind(id);
    return it == _bodies.constEnd() ? true : !b2Body_IsAwake(it->body);
}

bool OverlayPhysics::allAsleep() const
{
    for (const auto &body : _bodies) {
        if (body.kind == Free && b2Body_IsAwake(body.body)) {
            return false;
        }
        if (body.kind == Driven) {
            const b2Vec2 velocity = b2Body_GetLinearVelocity(body.body);
            if (std::hypot(velocity.x, velocity.y) * kPixelsPerMetre > 1.0) {
                return false;
            }
        }
    }
    return true;
}

QString OverlayPhysics::describe(int id) const
{
    const auto it = _bodies.constFind(id);
    if (it == _bodies.constEnd()) {
        return QStringLiteral("?");
    }
    const b2BodyType type = b2Body_GetType(it->body);
    const QString engineType = type == b2_dynamicBody ? QStringLiteral("dyn") : type == b2_kinematicBody ? QStringLiteral("kin") : QStringLiteral("static");
    return QStringLiteral("%1[%9] pos %2,%3 vel %4,%5 home %6,%7 %8")
        .arg(it->escaping ? QStringLiteral("escaping") : it->kind == Free ? QStringLiteral("free") : QStringLiteral("driven"))
        .arg(x(id), 0, 'f', 0).arg(y(id), 0, 'f', 0)
        .arg(_vx(id), 0, 'f', 0).arg(_vy(id), 0, 'f', 0)
        .arg(it->home.x() - (it->size.width() / 2), 0, 'f', 0).arg(it->home.y() - (it->size.height() / 2), 0, 'f', 0)
        .arg(b2Body_IsAwake(it->body) ? QStringLiteral("awake") : QStringLiteral("asleep"))
        .arg(engineType);
}

QString OverlayPhysics::report() const
{
    QStringList lines;
    for (auto it = _bodies.constBegin(); it != _bodies.constEnd(); ++it) {
        const Body &body = it.value();
        QString line = QStringLiteral("#%1 %2 rect %3,%4 %5x%6 %7")
            .arg(it.key())
            .arg(body.kind == Free ? QStringLiteral("free") : QStringLiteral("driven"))
            .arg(x(it.key()), 0, 'f', 0).arg(y(it.key()), 0, 'f', 0)
            .arg(body.size.width(), 0, 'f', 0).arg(body.size.height(), 0, 'f', 0)
            .arg(b2Body_IsAwake(body.body) ? QStringLiteral("awake") : QStringLiteral("asleep"));
        const QPointF centre = _centre(body);
        for (const QVariant &value : body.attachments) {
            const QVariantMap rect = value.toMap();
            const qreal w = rect.value(QStringLiteral("w")).toReal();
            const qreal h = rect.value(QStringLiteral("h")).toReal();
            const qreal ax = centre.x() - (body.size.width() / 2) + rect.value(QStringLiteral("x")).toReal();
            const qreal ay = centre.y() - (body.size.height() / 2) + rect.value(QStringLiteral("y")).toReal();
            line += QStringLiteral(" +attached %1,%2 %3x%4").arg(ax, 0, 'f', 0).arg(ay, 0, 'f', 0).arg(w, 0, 'f', 0).arg(h, 0, 'f', 0);
        }
        lines.append(line);
    }
    return lines.join(QLatin1Char('\n'));
}

void OverlayPhysics::setPull(qreal value)         { if (!qFuzzyCompare(_pull, value))         { _pull = value;         emit tuningChanged(); } }
void OverlayPhysics::setSpringRadius(qreal value) { if (!qFuzzyCompare(_springRadius, value)) { _springRadius = value; emit tuningChanged(); } }
void OverlayPhysics::setFriction(qreal value)     { if (!qFuzzyCompare(_friction, value))     { _friction = value;     emit tuningChanged(); } }
void OverlayPhysics::setRestitution(qreal value)  { if (!qFuzzyCompare(_restitution, value))  { _restitution = value;  emit tuningChanged(); } }

void OverlayPhysics::setDamping(qreal value)
{
    if (qFuzzyCompare(_damping, value)) {
        return;
    }
    _damping = value;
    for (auto &body : _bodies) {
        b2Body_SetLinearDamping(body.body, static_cast<float>(_damping));
    }
    emit tuningChanged();
}

void OverlayPhysics::_rebuildShape(Body &body)
{
    if (B2_IS_NON_NULL(body.shape)) {
        b2DestroyShape(body.shape);
    }
    b2ShapeDef shapeDef = b2DefaultShapeDef();
    shapeDef.density = 1.0f;
    shapeDef.friction = static_cast<float>(_friction);
    shapeDef.restitution = static_cast<float>(_restitution);
    shapeDef.filter.groupIndex = body.group;
    const b2Polygon box = b2MakeBox(static_cast<float>(body.size.width() / 2 / kPixelsPerMetre),
                                    static_cast<float>(body.size.height() / 2 / kPixelsPerMetre));
    body.shape = b2CreatePolygonShape(body.body, &shapeDef, &box);
}

void OverlayPhysics::_applyPull(Body &body)
{
    if (!body.hasHome || !b2Body_IsAwake(body.body)) {
        return;
    }
    const QPointF centre = _centre(body);
    const QPointF delta = body.home - centre;
    const qreal distance = std::hypot(delta.x(), delta.y());
    const b2Vec2 velocity = b2Body_GetLinearVelocity(body.body);
    const qreal speed = std::hypot(velocity.x, velocity.y) * kPixelsPerMetre;

    if (distance < 1.0 && speed < 20.0) {
        b2Body_SetTransform(body.body, toMetres(body.home.x(), body.home.y(), kPixelsPerMetre), b2Rot_identity);
        b2Body_SetLinearVelocity(body.body, b2Vec2{ 0.0f, 0.0f });
        b2Body_SetAwake(body.body, false);
        return;
    }
    if (distance < 0.5) {
        return;
    }
    const qreal accelerationPixels = distance < _springRadius ? _pull * (distance / _springRadius) : _pull;
    const qreal accelerationMetres = accelerationPixels / kPixelsPerMetre;
    const float mass = b2Body_GetMass(body.body);
    const b2Vec2 force{ static_cast<float>(mass * accelerationMetres * (delta.x() / distance)),
                        static_cast<float>(mass * accelerationMetres * (delta.y() / distance)) };
    b2Body_ApplyForceToCenter(body.body, force, true);
}

bool OverlayPhysics::_applyEscape(Body &body)
{
    const QPointF centre = _centre(body);
    for (const auto &other : _bodies) {
        if (other.kind != Driven) {
            continue;
        }
        const QPointF oc = _centre(other);
        const qreal left = oc.x() - (other.size.width() / 2), right = oc.x() + (other.size.width() / 2);
        const qreal top = oc.y() - (other.size.height() / 2), bottom = oc.y() + (other.size.height() / 2);
        if (centre.x() <= left || centre.x() >= right || centre.y() <= top || centre.y() >= bottom) {
            continue;
        }

        const b2Vec2 motion = b2Body_GetLinearVelocity(other.body);
        const float moving = std::hypot(motion.x, motion.y);
        b2Vec2 direction{ 0.0f, 0.0f };
        if (moving * kPixelsPerMetre > 50.0f) {
            direction = b2Vec2{ motion.x / moving, motion.y / moving };
        } else {
            const qreal hw = body.size.width() / 2, hh = body.size.height() / 2;
            struct Exit { qreal distance; b2Vec2 direction; bool open; };
            const Exit exits[4] = {
                { centre.x() - left,   b2Vec2{ -1.0f, 0.0f }, _bounds.isNull() || left - hw >= _bounds.left() },
                { right - centre.x(),  b2Vec2{ 1.0f, 0.0f },  _bounds.isNull() || right + hw <= _bounds.right() },
                { centre.y() - top,    b2Vec2{ 0.0f, -1.0f }, _bounds.isNull() || top - hh >= _bounds.top() },
                { bottom - centre.y(), b2Vec2{ 0.0f, 1.0f },  _bounds.isNull() || bottom + hh <= _bounds.bottom() },
            };
            const Exit *best = nullptr;
            for (const Exit &exit : exits) {
                if (exit.open && (!best || exit.distance < best->distance)) {
                    best = &exit;
                }
            }
            if (!best) {
                best = std::min_element(std::begin(exits), std::end(exits),
                                        [](const Exit &a, const Exit &b) { return a.distance < b.distance; });
            }
            direction = best->direction;
        }

        const float speed = static_cast<float>(kEscapeSpeed / kPixelsPerMetre);
        if (!body.escaping) {
            body.escaping = true;
            b2Body_SetType(body.body, b2_kinematicBody);
        }
        b2Body_SetAwake(body.body, true);
        b2Body_SetLinearVelocity(body.body, b2Vec2{ direction.x * speed, direction.y * speed });
        return true;
    }
    if (body.escaping) {
        body.escaping = false;
        b2Body_SetType(body.body, b2_dynamicBody);
        b2Body_SetLinearVelocity(body.body, b2Vec2{ 0.0f, 0.0f });
        b2Body_SetAwake(body.body, true);
    }
    return false;
}

void OverlayPhysics::_wakeFreeBodies(const Body &mover)
{
    for (const auto &other : _bodies) {
        if (other.kind == Free && !B2_ID_EQUALS(other.body, mover.body)) {
            b2Body_SetAwake(other.body, true);
        }
    }
}

QPointF OverlayPhysics::_centre(const Body &body) const
{
    const b2Vec2 position = b2Body_GetPosition(body.body);
    return QPointF(position.x * kPixelsPerMetre, position.y * kPixelsPerMetre);
}
