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

static void clearTestSettings()
{
    clearDragPositionSettings({"OverlayRigTest"});
}

static bool loadView(QQuickView &view)
{
    clearTestSettings();
    return loadTestView(view, QStringLiteral("qrc:/unittest/OverlayRigTest.qml"));
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

// A tap on a registered widget must not read as empty space: the fly view uses hitTest to
// decide whether a click in edit mode is "arrange this" or "leave edit mode".
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

// A resize used to remap and rewrite every stored position, so shrinking and growing back lost
// the arrangement a little more each time. Now each size keeps its own, and a size that has
// never been arranged simply inherits.
void OverlayRigTest::_resizeLeavesArrangementAlone()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const root = view.rootObject();
    QQuickItem *const right = root->findChild<QQuickItem*>("rightItem");
    QVERIFY(right);
    QVERIFY(moveTo(positionOf(view, "rightPosition"), 860.0, 600.0));
    QCoreApplication::processEvents();
    QCOMPARE(right->x(), 860.0);

    root->setWidth(root->width() + 200);
    QTRY_COMPARE(right->x(), 860.0);

    root->setWidth(root->width() - 200);
    QTRY_COMPARE(right->x(), 860.0);
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
    QTRY_VERIFY(right->x() == 860.0);

    root->setWidth(1200);
    QTRY_COMPARE(right->x(), 860.0);
    QVERIFY(moveTo(rightPosition, 400.0, 200.0));
    QTRY_COMPARE(right->x(), 400.0);

    // Back to the first size: its own arrangement returns, not the one just made at 1200.
    root->setWidth(1000);
    QTRY_COMPARE(right->x(), 860.0);
    QCOMPARE(right->y(), 600.0);

    root->setWidth(1200);
    QTRY_COMPARE(right->x(), 400.0);
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

    // 1010x800 has never been arranged; 1000x800 is the closest thing to what the user meant.
    root->setWidth(1010);
    QTRY_COMPARE(right->x(), 700.0);
    QCOMPARE(right->y(), 300.0);
}

// The contract the fly view actually needs: after a reflow nothing overlaps anything. Stacking
// every movable on one spot is the case relaxation alone cannot solve - each item has three
// obstacles and no single-axis escape - so this covers the eviction pass as well.
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
    QCoreApplication::processEvents();

    const QList<QQuickItem*> all { left, right, heavy, staticItem };
    for (int i = 0; i < all.size(); ++i) {
        for (int j = i + 1; j < all.size(); ++j) {
            QVERIFY2(!rectOf(all[i]).intersects(rectOf(all[j])),
                     qPrintable(QStringLiteral("%1 overlaps %2").arg(all[i]->objectName(), all[j]->objectName())));
        }
    }
}

// Mass is area. A small chip must not shove a big panel across the screen.
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
    QCoreApplication::processEvents();

    const qreal lightMoved = QLineF(lightBefore, QPointF(left->x(), left->y())).length();
    const qreal heavyMoved = QLineF(heavyBefore, QPointF(heavy->x(), heavy->y())).length();
    QVERIFY2(lightMoved > heavyMoved * 4,
             qPrintable(QStringLiteral("light moved %1, heavy moved %2").arg(lightMoved).arg(heavyMoved)));
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

    // A duplicate entry would push the same item twice per pass, so it lands somewhere other
    // than where one registration puts it.
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

    QCOMPARE(left->x(), 20.0);
    QCOMPARE(left->y(), 400.0);
    const bool clearOfAttached = right->x() >= attached->x() + attached->width() || right->x() + right->width() <= attached->x() ||
                                 right->y() >= attached->y() + attached->height() || right->y() + right->height() <= attached->y();
    QVERIFY(clearOfAttached);
}

// The video rail is glued to the pip, so it travels whenever the pip is dragged or nudged. A
// rig that only watches size never hears about that and leaves the telemetry chips sitting
// under the rail's buttons.
//
// The settle wait is what makes this a test of the move and not of the load: the rig reflows
// once on startup and again on every viewport resize, and the window being shown supplies
// both. Without waiting those out, the static gets separated by a reflow that was already
// queued and the test passes on a rig that ignores position entirely.
void OverlayRigTest::_staticThatMovesPushesMovablesOutOfTheWay()
{
    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const left = view.rootObject()->findChild<QQuickItem*>("leftItem");
    QQuickItem *const staticItem = view.rootObject()->findChild<QQuickItem*>("staticItem");
    QVERIFY(left && staticItem);

    QObject *const rig = rigOf(view);
    QTRY_VERIFY_WITH_TIMEOUT(!rig->property("reflowPending").toBool(), 3000);

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
