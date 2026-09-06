/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "OverlayRigTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QDateTime>
#include <QPair>
#include <algorithm>
#include <cmath>

static constexpr qreal kSettleTolerance = 1.0;

static void clearTestSettings()
{
    clearDragPositionSettings({"OverlayRigTest"});
}

#define WAIT_SETTLED(rig, ms) \
    do { \
        QObject *const _rig = (rig); \
        const qint64 _deadline = QDateTime::currentMSecsSinceEpoch() + (ms); \
        while (_rig->property("reflowPending").toBool() && QDateTime::currentMSecsSinceEpoch() < _deadline) { \
            QTest::qWait(20); \
        } \
        if (_rig->property("reflowPending").toBool()) { \
            qWarning().noquote() << "RIG DID NOT SETTLE. awake:\n" << _rig->property("awakeReport").toString() \
                                 << "\ntrace (last 60):\n" << _rig->property("trace").toString().split('\n').mid(-60).join('\n'); \
            QFAIL("simulation did not settle"); \
        } \
    } while (false)

static void traceAround(QObject *rig, const QString &item, const QString &restPrefix, const QStringList &alsoShow)
{
    const QStringList lines = rig->property("trace").toString().split('\n');
    int first = -1;
    for (int i = 0; i < lines.size(); ++i) {
        if (lines[i].contains(item + QStringLiteral(" free")) && !lines[i].contains(restPrefix)) {
            first = i;
            break;
        }
    }
    if (first < 0) {
        qWarning().noquote() << "TRACE: " << item << "never left rest";
        return;
    }
    for (int i = qMax(0, first - 3); i < qMin(lines.size(), first + 5); ++i) {
        if (alsoShow.contains(QStringLiteral("*"))) {
            qWarning().noquote() << "TRACE:" << lines[i].left(1200);
            continue;
        }
        QStringList kept;
        for (const QString &part : lines[i].split(QStringLiteral(" | "))) {
            if (part.contains(item) || std::any_of(alsoShow.begin(), alsoShow.end(), [&](const QString &n) { return part.contains(n); })
                    || part.contains(QStringLiteral(" step "))) {
                kept.append(part.left(160));
            }
        }
        qWarning().noquote() << "TRACE:" << kept.join(QStringLiteral("\n       "));
    }
}

static bool loadView(QQuickView &view)
{
    clearTestSettings();
    if (!loadTestView(view, QStringLiteral("qrc:/unittest/OverlayRigTest.qml"))) {
        return false;
    }
    if (QObject *const rig = view.rootObject()->property("rig").value<QObject*>()) {
        rig->setProperty("debugLog", true);
    }
    return true;
}

void OverlayRigTest::cleanupTestCase()
{
    clearTestSettings();
}

static QObject *rigOf(const QQuickView &view)
{
    return view.rootObject()->property("rig").value<QObject*>();
}

static bool hitTest(QObject *rig, qreal x, qreal y, bool *invoked)
{
    QVariant result;
    *invoked = QMetaObject::invokeMethod(rig, "hitTest", Q_RETURN_ARG(QVariant, result),
                                         Q_ARG(QVariant, x), Q_ARG(QVariant, y));
    return result.toBool();
}

void OverlayRigTest::_hitTestFindsRegisteredItems()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QVERIFY(left && staticItem);

    bool invoked = false;
    QVERIFY(hitTest(rigOf(view), left->x() + left->width() / 2, left->y() + left->height() / 2, &invoked));
    QVERIFY(invoked);
    QVERIFY(hitTest(rigOf(view), staticItem->x() + staticItem->width() / 2, staticItem->y() + staticItem->height() / 2, &invoked));
    QVERIFY(invoked);
}

void OverlayRigTest::_hitTestMissesEmptySpace()
{
    QQuickView view;
    QVERIFY(loadView(view));

    bool invoked = false;
    QVERIFY(!hitTest(rigOf(view), 300, 300, &invoked));
    QVERIFY(invoked);
}

static QObject *positionOf(const QQuickView &view, const char *name)
{
    return view.rootObject()->findChild<QObject*>(QString::fromLatin1(name));
}

static bool moveTo(QObject *position, qreal x, qreal y)
{
    return QMetaObject::invokeMethod(position, "moveTo", Q_ARG(QVariant, x), Q_ARG(QVariant, y),
                                     Q_ARG(QVariant, false));
}

static QRectF rectOf(QQuickItem *item)
{
    return QRectF(item->x(), item->y(), item->width(), item->height());
}

static QPointF homeOf(QObject *position)
{
    return QPointF(position->property("homeX").toReal(), position->property("homeY").toReal());
}

void OverlayRigTest::_resizeLeavesArrangementAlone()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const root = view.rootObject();
    QQuickItem *const right = root->findChild<QQuickItem*>("rightItem");
    QObject *const rightPosition = positionOf(view, "rightPosition");
    QVERIFY(right && rightPosition);
    QVERIFY(moveTo(rightPosition, 860.0, 600.0));
    QCoreApplication::processEvents();
    QCOMPARE(homeOf(rightPosition).x(), 860.0);

    root->setWidth(root->width() + 200);
    QTRY_COMPARE(homeOf(rightPosition).x(), 860.0);

    root->setWidth(root->width() - 200);
    QTRY_COMPARE(homeOf(rightPosition).x(), 860.0);
}

void OverlayRigTest::_eachWindowSizeKeepsItsOwnArrangement()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const root = view.rootObject();
    QQuickItem *const right = root->findChild<QQuickItem*>("rightItem");
    QObject *const rightPosition = positionOf(view, "rightPosition");
    QVERIFY(right && rightPosition);

    QVERIFY(moveTo(rightPosition, 860.0, 600.0));
    QTRY_COMPARE(homeOf(rightPosition).x(), 860.0);

    root->setWidth(1200);
    QTRY_COMPARE(homeOf(rightPosition).x(), 860.0);
    QVERIFY(moveTo(rightPosition, 400.0, 200.0));
    QTRY_COMPARE(homeOf(rightPosition).x(), 400.0);

    root->setWidth(1000);
    QTRY_COMPARE(homeOf(rightPosition), QPointF(860.0, 600.0));

    root->setWidth(1200);
    QTRY_COMPARE(homeOf(rightPosition).x(), 400.0);
}

void OverlayRigTest::_unseenSizeStartsFromTheNearestArrangement()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const root = view.rootObject();
    QQuickItem *const right = root->findChild<QQuickItem*>("rightItem");
    QVERIFY(right);
    QVERIFY(moveTo(positionOf(view, "rightPosition"), 700.0, 300.0));
    QTRY_COMPARE(right->x(), 700.0);

    root->setWidth(1010);
    QTRY_COMPARE(right->x(), 700.0);
    QCOMPARE(right->y(), 300.0);
}

void OverlayRigTest::_resolveLeavesNothingOverlapping()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const right = view.rootObject()->findChild<QQuickItem*>("rightItem");
    QQuickItem *const heavy = view.rootObject()->findChild<QQuickItem*>("heavyItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QVERIFY(left && right && heavy && staticItem);

    QVERIFY(moveTo(positionOf(view, "leftPosition"), 500.0, 300.0));
    QVERIFY(moveTo(positionOf(view, "rightPosition"), 510.0, 310.0));
    QVERIFY(moveTo(positionOf(view, "heavyPosition"), 490.0, 290.0));
    QCoreApplication::processEvents();

    QVERIFY(QMetaObject::invokeMethod(rigOf(view), "resolve", Q_ARG(QVariant, QVariant())));
    WAIT_SETTLED(rigOf(view), 5000);

    const QList<QQuickItem*> all { left, right, heavy, staticItem };
    for (int i = 0; i < all.size(); ++i) {
        for (int j = i + 1; j < all.size(); ++j) {
            QVERIFY2(!rectOf(all[i]).intersects(rectOf(all[j])),
                     qPrintable(QStringLiteral("%1 overlaps %2").arg(all[i]->objectName(), all[j]->objectName())));
        }
    }
}

void OverlayRigTest::_lightItemYieldsToHeavyOne()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const heavy = view.rootObject()->findChild<QQuickItem*>("heavyItem");
    QVERIFY(left && heavy);

    const QPointF heavyBefore(heavy->x(), heavy->y());
    QVERIFY(moveTo(positionOf(view, "leftPosition"), heavyBefore.x() + 40, heavyBefore.y() + 40));
    QCoreApplication::processEvents();
    const QPointF lightBefore(left->x(), left->y());

    QVERIFY(QMetaObject::invokeMethod(rigOf(view), "resolve", Q_ARG(QVariant, QVariant())));

    const auto lightMoved = [&]() { return QLineF(lightBefore, QPointF(left->x(), left->y())).length(); };
    const auto heavyMoved = [&]() { return QLineF(heavyBefore, QPointF(heavy->x(), heavy->y())).length(); };
    QTRY_VERIFY2_WITH_TIMEOUT(lightMoved() > 30 && lightMoved() > heavyMoved() * 4,
                              qPrintable(QStringLiteral("light moved %1, heavy moved %2").arg(lightMoved()).arg(heavyMoved())),
                              3000);
}

void OverlayRigTest::_registerMovableDoesNotDuplicate()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QObject *const leftPosition = view.rootObject()->findChild<QObject*>("leftPosition");
    QVERIFY(left && leftPosition);

    QObject *const rig = rigOf(view);
    QVERIFY(QMetaObject::invokeMethod(rig, "registerMovable",
                                      Q_ARG(QVariant, QVariant::fromValue(left)),
                                      Q_ARG(QVariant, QVariant::fromValue(leftPosition))));
    QVERIFY(QMetaObject::invokeMethod(leftPosition, "moveTo",
                                      Q_ARG(QVariant, 30.0), Q_ARG(QVariant, 400.0), Q_ARG(QVariant, false)));
    QCoreApplication::processEvents();

    QVERIFY(QMetaObject::invokeMethod(rig, "resolve", Q_ARG(QVariant, QVariant())));
    QCoreApplication::processEvents();
    const QPointF once(left->x(), left->y());

    QVERIFY(QMetaObject::invokeMethod(rig, "registerMovable",
                                      Q_ARG(QVariant, QVariant::fromValue(left)),
                                      Q_ARG(QVariant, QVariant::fromValue(leftPosition))));
    QVERIFY(QMetaObject::invokeMethod(rig, "resolve", Q_ARG(QVariant, QVariant())));
    QCoreApplication::processEvents();

    QCOMPARE(QPointF(left->x(), left->y()), once);
}

void OverlayRigTest::_ownedStaticPushesOthersButNotOwner()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const right = view.rootObject()->findChild<QQuickItem*>("rightItem");
    QQuickItem *const attached = view.rootObject()->findChild<QQuickItem*>("attachedItem");
    QObject *const rightPosition = view.rootObject()->findChild<QObject*>("rightPosition");
    QVERIFY(left && right && attached && rightPosition);

    QVERIFY(QMetaObject::invokeMethod(rightPosition, "moveTo", Q_ARG(QVariant, 150.0), Q_ARG(QVariant, 380.0), Q_ARG(QVariant, false)));
    QCOMPARE(right->x(), 150.0);

    QVERIFY(QMetaObject::invokeMethod(rigOf(view), "resolve", Q_ARG(QVariant, QVariant::fromValue(static_cast<QObject*>(right)))));

    QCOMPARE(QPointF(left->x(), left->y()), homeOf(positionOf(view, "leftPosition")));
    const auto clearOfAttachedNow = [right, attached]() {
        return right->x() >= attached->x() + attached->width() || right->x() + right->width() <= attached->x() ||
               right->y() >= attached->y() + attached->height() || right->y() + right->height() <= attached->y();
    };
    QTRY_VERIFY_WITH_TIMEOUT(clearOfAttachedNow(), 3000);
    const bool clearOfAttached = right->x() >= attached->x() + attached->width() || right->x() + right->width() <= attached->x() ||
                                 right->y() >= attached->y() + attached->height() || right->y() + right->height() <= attached->y();
    QVERIFY(clearOfAttached);
}

void OverlayRigTest::_staticThatMovesPushesMovablesOutOfTheWay()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QVERIFY(left && staticItem);

    QObject *const rig = rigOf(view);
    WAIT_SETTLED(rig, 3000);

    const qreal restingX = left->x();
    const qreal restingY = left->y();
    staticItem->setX(restingX);
    staticItem->setY(restingY);

    const auto clearOfStatic = [left, staticItem]() {
        return left->x() >= staticItem->x() + staticItem->width() || left->x() + left->width() <= staticItem->x() ||
               left->y() >= staticItem->y() + staticItem->height() || left->y() + left->height() <= staticItem->y();
    };
    QTRY_VERIFY_WITH_TIMEOUT(clearOfStatic(), 3000);
}

void OverlayRigTest::_pushedItemParksOnTheNeighbourGutter()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QObject *const rig = rigOf(view);
    QVERIFY(left && staticItem && rig);
    WAIT_SETTLED(rig, 3000);

    staticItem->setX(left->x());
    staticItem->setY(left->y());
    WAIT_SETTLED(rig, 5000);

    const QRectF item = rectOf(left);
    const QRectF obstacle = rectOf(staticItem);
    const qreal gap = qMax(qMax(obstacle.left() - item.right(), item.left() - obstacle.right()),
                           qMax(obstacle.top() - item.bottom(), item.top() - obstacle.bottom()));
    QCOMPARE(gap, rig->property("edgeMargin").toReal());
}

void OverlayRigTest::_displacedItemStaysAgainstWhatPushedIt()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QVERIFY(left && staticItem);

    QObject *const rig = rigOf(view);
    WAIT_SETTLED(rig, 3000);

    const qreal homeX = left->x();
    const qreal homeY = left->y();

    staticItem->setWidth(400);
    staticItem->setHeight(200);
    staticItem->setX(homeX - 40);
    staticItem->setY(homeY - 40);

    const auto clearOfStatic = [left, staticItem]() {
        return left->x() >= staticItem->x() + staticItem->width() || left->x() + left->width() <= staticItem->x() ||
               left->y() >= staticItem->y() + staticItem->height() || left->y() + left->height() <= staticItem->y();
    };
    QTRY_VERIFY_WITH_TIMEOUT(clearOfStatic(), 3000);

    const auto hugging = [left, staticItem]() {
        const qreal gapBelow = left->y() - (staticItem->y() + staticItem->height());
        const qreal gapAbove = staticItem->y() - (left->y() + left->height());
        const qreal gapRight = left->x() - (staticItem->x() + staticItem->width());
        const qreal gapLeft  = staticItem->x() - (left->x() + left->width());
        return qMax(qMax(gapBelow, gapAbove), qMax(gapRight, gapLeft));
    };
    if (!QTest::qWaitFor([&]() { return hugging() >= 0 && hugging() < 20; }, 5000)) {
        traceAround(rig, QStringLiteral("leftItem"), QStringLiteral("leftItem free pos 12,392 vel 0,0"), { QStringLiteral("*") });
        qWarning().noquote() << "TRACE: world ->" << rig->property("worldReport").toString();
    }
    QTRY_VERIFY2_WITH_TIMEOUT(hugging() >= 0 && hugging() < 20,
                              qPrintable(QStringLiteral("item settled %1px from the obstacle at %2,%3 (home was %4,%5)")
                                             .arg(hugging()).arg(left->x()).arg(left->y()).arg(homeX).arg(homeY)),
                              5000);
}

void OverlayRigTest::_itemSpringsBackWhenTheObstructionLeaves()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QVERIFY(left && staticItem);

    QObject *const rig = rigOf(view);
    WAIT_SETTLED(rig, 3000);

    const qreal homeX = left->x();
    const qreal homeY = left->y();

    staticItem->setX(homeX);
    staticItem->setY(homeY);
    QTRY_VERIFY_WITH_TIMEOUT(left->x() != homeX || left->y() != homeY, 3000);

    staticItem->setVisible(false);
    QTRY_VERIFY_WITH_TIMEOUT(qFuzzyCompare(left->x() + 1, homeX + 1) &&
                                 qFuzzyCompare(left->y() + 1, homeY + 1), 3000);
}

void OverlayRigTest::_overlapIsAnsweredOnTheChangeThatCausedIt()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QObject *const leftPosition = positionOf(view, "leftPosition");
    QVERIFY(left && staticItem && leftPosition);

    QObject *const rig = rigOf(view);
    WAIT_SETTLED(rig, 3000);

    staticItem->setY(leftPosition->property("homeY").toReal());
    WAIT_SETTLED(rig, 3000);
    QVERIFY(!leftPosition->property("displaced").toBool());

    staticItem->setX(leftPosition->property("homeX").toReal());

    QTRY_VERIFY2_WITH_TIMEOUT(leftPosition->property("displaced").toBool(),
                              "rig took longer than a few frames to answer the geometry change that caused the overlap",
                              300);
}

void OverlayRigTest::_disturbedLayoutComesToRest()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QObject *const leftPosition = positionOf(view, "leftPosition");
    QObject *const rig = rigOf(view);
    QVERIFY(left && staticItem && leftPosition && rig);

    WAIT_SETTLED(rig, 5000);

    const QPointF home = homeOf(leftPosition);

    staticItem->setX(home.x());
    staticItem->setY(home.y());
    QTRY_VERIFY_WITH_TIMEOUT(leftPosition->property("displaced").toBool(), 3000);

    staticItem->setVisible(false);

    WAIT_SETTLED(rig, 5000);
    QVERIFY2(!leftPosition->property("displaced").toBool(),
             qPrintable(QStringLiteral("settled %1,%2 away from home")
                            .arg(leftPosition->property("nudgeX").toReal())
                            .arg(leftPosition->property("nudgeY").toReal())));
    QCOMPARE(left->x(), home.x());
    QCOMPARE(left->y(), home.y());
}

void OverlayRigTest::_sweepingObstacleDoesNotFlingItemsAcrossTheWindow()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const root = view.rootObject();
    QQuickItem *const left = root->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = root->findChild<QQuickItem*>("staticItem");
    QObject *const leftPosition = positionOf(view, "leftPosition");
    QObject *const rig = rigOf(view);
    QVERIFY(root && left && staticItem && leftPosition && rig);

    WAIT_SETTLED(rig, 5000);

    const QPointF home = homeOf(leftPosition);
    staticItem->setY(home.y() - 20);

    const qreal gap = rig->property("_gap").toReal();
    qreal worstLead = 0;
    for (int step = 0; step < 40; step++) {
        staticItem->setX(home.x() - 300 + (step * 20));
        QTest::qWait(16);
        const qreal edge = staticItem->x() + staticItem->width() + gap;
        worstLead = qMax(worstLead, left->x() - edge);
    }

    QVERIFY2(worstLead < 350,
             qPrintable(QStringLiteral("item got %1px ahead of the edge pushing it").arg(worstLead)));

    staticItem->setVisible(false);
    WAIT_SETTLED(rig, 5000);
    QTRY_COMPARE(QPointF(left->x(), left->y()), home);
}

void OverlayRigTest::_furnitureBoltedToAMovableDoesNotFeedItself()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const carrier = view.rootObject()->findChild<QQuickItem*>("carrierItem");
    QQuickItem *const passenger = view.rootObject()->findChild<QQuickItem*>("passengerItem");
    QObject *const rig = rigOf(view);
    QVERIFY(carrier && passenger && rig);

    WAIT_SETTLED(rig, 8000);

    const QPointF carrierAtRest(carrier->x(), carrier->y());
    const QPointF passengerAtRest(passenger->x(), passenger->y());

    QTest::qWait(400);

    QCOMPARE(QPointF(carrier->x(), carrier->y()), carrierAtRest);
    QCOMPARE(QPointF(passenger->x(), passenger->y()), passengerAtRest);
}

void OverlayRigTest::_anEdgeCarriesWhatItHitsInItsOwnDirection()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QObject *const leftPosition = positionOf(view, "leftPosition");
    QObject *const rig = rigOf(view);
    QVERIFY(left && staticItem && leftPosition && rig);

    WAIT_SETTLED(rig, 5000);
    const QPointF home = homeOf(leftPosition);

    const qreal gap = rig->property("_gap").toReal();
    staticItem->setY(home.y() + left->height() + gap + 24);
    staticItem->setX(home.x() - 250);
    WAIT_SETTLED(rig, 5000);
    staticItem->setWidth(600);
    staticItem->setHeight(80);
    WAIT_SETTLED(rig, 5000);
    QVERIFY2(QPointF(left->x(), left->y()) == home,
             qPrintable(QStringLiteral("item moved to %1,%2 during setup (home %3,%4, gap %5)").arg(left->x()).arg(left->y()).arg(home.x()).arg(home.y()).arg(gap)));

    const qreal before = staticItem->y();
    staticItem->setY(before - 120);

    const bool carriedUp = QTest::qWaitFor([&]() { return left->y() + left->height() <= staticItem->y() + 12.0; }, 1500);
    if (!carriedUp) {
        traceAround(rig, QStringLiteral("leftItem"), QStringLiteral("leftItem free pos 12,392 vel 0,0"), { QStringLiteral("*") });
    }
    QTRY_VERIFY2_WITH_TIMEOUT(left->y() + left->height() <= staticItem->y() + 12.0,
                              qPrintable(QStringLiteral("item bottom %1 is still inside the obstacle top %2")
                                             .arg(left->y() + left->height()).arg(staticItem->y())),
                              1000);
    QVERIFY2(qAbs(left->x() - home.x()) < 2.0,
             qPrintable(QStringLiteral("item was ejected sideways to x=%1 (home %2)").arg(left->x()).arg(home.x())));
}

void OverlayRigTest::_releasedItemStaysWhereItWasDropped()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QObject *const leftPosition = positionOf(view, "leftPosition");
    QVERIFY(left && leftPosition);

    QObject *const rig = rigOf(view);
    WAIT_SETTLED(rig, 3000);

    const qreal startX = left->x();
    for (int frame = 1; frame <= 6; ++frame) {
        left->setX(startX + frame * 70);
        QVERIFY(QMetaObject::invokeMethod(rig, "resolve", Q_ARG(QVariant, QVariant())));
        QTest::qWait(16);
    }
    const QPointF dropped(left->x(), left->y());
    QVERIFY(QMetaObject::invokeMethod(leftPosition, "commit"));
    QVERIFY(QMetaObject::invokeMethod(rig, "resolve", Q_ARG(QVariant, QVariant())));

    qreal worst = 0;
    for (int i = 0; i < 60; ++i) {
        QTest::qWait(16);
        worst = std::max(worst, QLineF(dropped, QPointF(left->x(), left->y())).length());
    }
    const qreal slot = rig->property("slotSize").toReal();
    const qreal snapReach = std::hypot(slot, slot) / 2 + kSettleTolerance;
    QVERIFY2(worst < snapReach, qPrintable(QStringLiteral("strayed %1 px from the drop point (snap reach %2)").arg(worst).arg(snapReach)));
    WAIT_SETTLED(rig, 3000);
}

void OverlayRigTest::_itemFollowsAMovedHomeWhenTheObstructionLeaves()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QObject *const leftPosition = positionOf(view, "leftPosition");
    QVERIFY(left && staticItem && leftPosition);

    QObject *const rig = rigOf(view);
    WAIT_SETTLED(rig, 3000);

    const qreal homeX = left->x();
    const qreal homeY = left->y();

    staticItem->setX(homeX);
    staticItem->setY(homeY);
    QTRY_VERIFY_WITH_TIMEOUT(left->x() != homeX || left->y() != homeY, 3000);
    WAIT_SETTLED(rig, 3000);

    leftPosition->setProperty("defaultX", homeX + 300);
    const QPointF newHome = homeOf(leftPosition);
    QVERIFY(newHome.x() > homeX + 200);
    staticItem->setVisible(false);
    QTRY_VERIFY2_WITH_TIMEOUT(qFuzzyCompare(left->x() + 1, newHome.x() + 1) && qFuzzyCompare(left->y() + 1, newHome.y() + 1),
                              qPrintable(QStringLiteral("left rests at %1,%2 instead of its new home %3,%4")
                                             .arg(left->x()).arg(left->y()).arg(newHome.x()).arg(newHome.y())),
                              3000);
}

void OverlayRigTest::_crowdedItemsAllFindSeparatePlaces()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const root = view.rootObject();
    QQuickItem *const staticItem = root->findChild<QQuickItem*>("staticItem");
    QObject *const rig = rigOf(view);
    QVERIFY(root && staticItem && rig);
    WAIT_SETTLED(rig, 5000);

    staticItem->setX(0);
    staticItem->setY(250);
    staticItem->setWidth(root->width());
    staticItem->setHeight(root->height() - 250);
    WAIT_SETTLED(rig, 8000);

    const QStringList names = { "leftItem", "rightItem", "heavyItem", "carrierItem", "carrierRail", "passengerItem", "staticItem" };
    QList<QPair<QString, QRectF>> rects;
    for (const QString &name : names) {
        QQuickItem *const item = root->findChild<QQuickItem*>(name);
        QVERIFY2(item, qPrintable(name));
        const QRectF rect(item->x(), item->y(), item->width(), item->height());
        QVERIFY2(QRectF(0, 0, root->width(), root->height()).contains(rect),
                 qPrintable(QStringLiteral("%1 left the window at %2,%3").arg(name).arg(rect.x()).arg(rect.y())));
        rects.append({ name, rect });
    }
    for (int i = 0; i < rects.size(); ++i) {
        for (int j = i + 1; j < rects.size(); ++j) {
            const QRectF overlap = rects[i].second.intersected(rects[j].second);
            QVERIFY2(overlap.isEmpty(),
                     qPrintable(QStringLiteral("%1 overlaps %2 by %3x%4").arg(rects[i].first, rects[j].first)
                                    .arg(overlap.width()).arg(overlap.height())));
        }
    }
}

void OverlayRigTest::_droppedItemNeverSnapsBackToWhereItCameFrom()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QObject *const leftPosition = positionOf(view, "leftPosition");
    QObject *const rig = rigOf(view);
    QVERIFY(left && leftPosition && rig);
    WAIT_SETTLED(rig, 3000);

    const qreal fromX = left->x();
    left->setX(fromX + 400);
    left->setY(left->y() + 100);
    QVERIFY(QMetaObject::invokeMethod(leftPosition, "commit"));
    QVERIFY(QMetaObject::invokeMethod(rig, "requestReflow"));

    const qreal droppedX = left->x();
    const qint64 deadline = QDateTime::currentMSecsSinceEpoch() + 600;
    while (QDateTime::currentMSecsSinceEpoch() < deadline) {
        QVERIFY2(left->x() > fromX + 200,
                 qPrintable(QStringLiteral("item flew back to x=%1 after being dropped at %2").arg(left->x()).arg(droppedX)));
        QTest::qWait(5);
    }
    QCOMPARE(left->x(), homeOf(leftPosition).x());
}

void OverlayRigTest::_dropAlignsWithANeighboursEdge()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const right = view.rootObject()->findChild<QQuickItem*>("rightItem");
    QObject *const rightPosition = positionOf(view, "rightPosition");
    QObject *const rig = rigOf(view);
    QVERIFY(left && right && rightPosition && rig);
    QVERIFY(moveTo(rightPosition, 503.0, 300.0));
    WAIT_SETTLED(rig, 3000);

    QVariant landing;
    QVERIFY(QMetaObject::invokeMethod(rig, "alignDrop", Q_RETURN_ARG(QVariant, landing),
                                      Q_ARG(QVariant, QVariant::fromValue(static_cast<QObject*>(left))),
                                      Q_ARG(QVariant, 507.0), Q_ARG(QVariant, 380.0)));
    QCOMPARE(landing.toPointF().x(), 503.0);
    QCOMPARE(landing.toPointF().y(), right->y() + right->height() + rig->property("edgeMargin").toReal());

    QVERIFY(QMetaObject::invokeMethod(rig, "alignDrop", Q_RETURN_ARG(QVariant, landing),
                                      Q_ARG(QVariant, QVariant::fromValue(static_cast<QObject*>(left))),
                                      Q_ARG(QVariant, 750.0), Q_ARG(QVariant, 450.0)));
    const qreal grid = rig->property("slotSize").toReal();
    const qreal margin = rig->property("edgeMargin").toReal();
    QVERIFY(grid > 0);
    const auto onLattice = [&](qreal position, qreal extent, qreal size) {
        return qFuzzyIsNull(std::fmod(position - margin, grid)) || qFuzzyIsNull(std::fmod(size - margin - (position + extent), grid));
    };
    QVERIFY2(onLattice(landing.toPointF().x(), left->width(), view.rootObject()->width()),
             qPrintable(QStringLiteral("x=%1 is off the %2px lattice").arg(landing.toPointF().x()).arg(grid)));
    QVERIFY2(onLattice(landing.toPointF().y(), left->height(), view.rootObject()->height()),
             qPrintable(QStringLiteral("y=%1 is off the %2px lattice").arg(landing.toPointF().y()).arg(grid)));
}

void OverlayRigTest::_droppedItemTakesItsSlotAndTheNeighbourYields()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const passenger = view.rootObject()->findChild<QQuickItem*>("passengerItem");
    QQuickItem *const right = view.rootObject()->findChild<QQuickItem*>("rightItem");
    QObject *const passengerPosition = positionOf(view, "passengerPosition");
    QObject *const rig = rigOf(view);
    QVERIFY(passenger && right && passengerPosition && rig);
    WAIT_SETTLED(rig, 3000);

    const QPointF rightHome(right->x(), right->y());
    passenger->setX(rightHome.x() + 3);
    passenger->setY(rightHome.y() + 2);
    QVERIFY(QMetaObject::invokeMethod(passengerPosition, "commit"));
    QVERIFY(QMetaObject::invokeMethod(rig, "requestReflow"));
    WAIT_SETTLED(rig, 5000);

    QVERIFY2(QPointF(passenger->x(), passenger->y()) == homeOf(passengerPosition),
             qPrintable(QStringLiteral("passenger rests at %1,%2, home %3,%4\n%5").arg(passenger->x()).arg(passenger->y())
                            .arg(homeOf(passengerPosition).x()).arg(homeOf(passengerPosition).y()).arg(rig->property("worldReport").toString())));
    QVERIFY2(!rectOf(passenger).intersects(rectOf(right)),
             qPrintable(QStringLiteral("passenger %1,%2 still overlaps right %3,%4").arg(passenger->x()).arg(passenger->y()).arg(right->x()).arg(right->y())));
    QVERIFY2(QPointF(right->x(), right->y()) != rightHome, "the neighbour did not make way for the dropped item");
}

void OverlayRigTest::_gapReadoutMarksTheGutterBesideANeighbour()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const right = view.rootObject()->findChild<QQuickItem*>("rightItem");
    QObject *const rightPosition = positionOf(view, "rightPosition");
    QObject *const rig = rigOf(view);
    QVERIFY(left && right && rightPosition && rig);
    QVERIFY(moveTo(rightPosition, 503.0, 300.0));
    WAIT_SETTLED(rig, 3000);

    const qreal margin = rig->property("edgeMargin").toReal();
    QVariant readouts;
    QVERIFY(QMetaObject::invokeMethod(rig, "spacingReadoutsFor", Q_RETURN_ARG(QVariant, readouts),
                                      Q_ARG(QVariant, QVariant::fromValue(static_cast<QObject*>(left))),
                                      Q_ARG(QVariant, right->x() + right->width() + margin), Q_ARG(QVariant, right->y())));
    const QVariantList list = readouts.toList();
    QCOMPARE(list.size(), 1);
    QCOMPARE(list.first().toMap().value("text").toInt(), qRound(margin));
    QCOMPARE(list.first().toMap().value("x").toReal(), right->x() + right->width() + margin / 2);
    QVERIFY(list.first().toMap().value("y").toReal() < right->y());

    QVERIFY(QMetaObject::invokeMethod(rig, "spacingReadoutsFor", Q_RETURN_ARG(QVariant, readouts),
                                      Q_ARG(QVariant, QVariant::fromValue(static_cast<QObject*>(left))),
                                      Q_ARG(QVariant, right->x() + right->width() + margin + 30), Q_ARG(QVariant, right->y())));
    QVERIFY(readouts.toList().isEmpty());
}
