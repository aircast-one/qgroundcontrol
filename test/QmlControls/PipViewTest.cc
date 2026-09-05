/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "PipViewTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtCore/QSettings>

static const qreal kMargin = 8;

static void clearPipSettings()
{
    clearQmlGlobalSettings({"IsPIPVisible", "PipViewTestItem1IsFull"});
    clearDragPositionSettings({"PIP"});
}

static bool loadView(QQuickView& view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/PipViewTest.qml"));
}

void PipViewTest::_dragRepositionsAndPersists()
{
    clearPipSettings();

    QPointF draggedPos;

    {
        QQuickView view;
        QVERIFY(loadView(view));
        view.rootObject()->setProperty("editMode", true);

        QQuickItem* root = view.rootObject();
        QQuickItem* pip = root->findChild<QQuickItem*>("pip");
        QQuickItem* itemB = root->findChild<QQuickItem*>("itemB");
        QVERIFY(pip);
        QVERIFY(itemB);

        QCOMPARE(pip->x(), kMargin);
        QCOMPARE(pip->y(), root->height() - pip->height() - kMargin);

        QQuickItem* pipParentBefore = itemB->parentItem();

        const QPoint start = itemCenter(pip);
        const QPoint target(500, 250);
        dragMouse(view, start, target);

        const qreal grid = dropGridOf(root);
        const qreal margin = dropMarginOf(root);
        const QPointF expected(snapToDropGrid(kMargin + target.x() - start.x(), pip->width(), root->width(), grid, margin),
                               snapToDropGrid(root->height() - pip->height() - kMargin + target.y() - start.y(), pip->height(), root->height(), grid, margin));
        QTRY_VERIFY(qAbs(pip->x() - expected.x()) < 2);
        QTRY_VERIFY(qAbs(pip->y() - expected.y()) < 2);

        QCOMPARE(itemB->parentItem(), pipParentBefore);

        QTRY_VERIFY(storedPosition(QStringLiteral("PIP800x600-CustomPosition")));
        draggedPos = QPointF(pip->x(), pip->y());
    }

    QQuickView view2;
    QVERIFY(loadView(view2));
    view2.rootObject()->setProperty("editMode", true);
    QQuickItem* pip2 = view2.rootObject()->findChild<QQuickItem*>("pip");
    QVERIFY(pip2);
    QCOMPARE(pip2->x(), draggedPos.x());
    QCOMPARE(pip2->y(), draggedPos.y());
}

void PipViewTest::_dragOffscreenClampsCommittedPosition()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));
        view.rootObject()->setProperty("editMode", true);

    QQuickItem* root = view.rootObject();
    QQuickItem* pip = root->findChild<QQuickItem*>("pip");
    QVERIFY(pip);
    dragMouse(view, itemCenter(pip), QPoint(root->width() - 1, root->height() - 1));
    QObject* const dragPosition = root->findChild<QObject*>("dragPosition");
    QVERIFY(dragPosition);
    const qreal edgeMargin = dragPosition->property("edgeMargin").toReal();
    QVERIFY(edgeMargin > 0);

    QTRY_COMPARE(pip->x(), root->width() - pip->width() - edgeMargin);
    QTRY_COMPARE(pip->y(), root->height() - pip->height() - edgeMargin);
}

void PipViewTest::_dragBackToDefaultSnapsAndResets()
{
    clearPipSettings();

    {
        QQuickView view;
        QVERIFY(loadView(view));
        view.rootObject()->setProperty("editMode", true);

        QQuickItem* root = view.rootObject();
        QQuickItem* pip = root->findChild<QQuickItem*>("pip");
        QVERIFY(pip);

        const QPointF defaultPos(kMargin, root->height() - pip->height() - kMargin);

        dragMouse(view, itemCenter(pip), QPoint(500, 250));
        QTRY_VERIFY(pip->x() != defaultPos.x());
        const QPoint offsetFromDefault(pip->x() - defaultPos.x() - 4, pip->y() - defaultPos.y() - 4);
        dragMouse(view, itemCenter(pip), itemCenter(pip) - offsetFromDefault);

        QTRY_COMPARE(pip->x(), defaultPos.x());
        QTRY_COMPARE(pip->y(), defaultPos.y());
    }
    QQuickView view2;
    QVERIFY(loadView(view2));
    view2.rootObject()->setProperty("editMode", true);
    QQuickItem* pip2 = view2.rootObject()->findChild<QQuickItem*>("pip");
    QVERIFY(pip2);
    QCOMPARE(pip2->x(), kMargin);
    QCOMPARE(pip2->y(), view2.rootObject()->height() - pip2->height() - kMargin);
}

void PipViewTest::_clickSwapsWithoutDrag()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem* root = view.rootObject();
    QQuickItem* pip = root->findChild<QQuickItem*>("pip");
    QQuickItem* itemA = root->findChild<QQuickItem*>("itemA");
    QQuickItem* itemB = root->findChild<QQuickItem*>("itemB");
    QVERIFY(pip);
    QVERIFY(itemA);
    QVERIFY(itemB);

    QCOMPARE(itemA->parentItem(), root);
    QVERIFY(itemB->parentItem() != root);

    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(pip));

    QTRY_COMPARE(itemB->parentItem(), root);
    QVERIFY(itemA->parentItem() != root);
}


static QQuickItem *gripOf(QQuickView &view)
{
    return view.rootObject()->findChild<QQuickItem*>(QStringLiteral("pipResizeGrip"));
}

static QQuickItem *pipOf(QQuickView &view)
{
    return view.rootObject()->findChild<QQuickItem*>(QStringLiteral("pip"));
}
void PipViewTest::_gripResizesAndPersists()
{
    clearPipSettings();

    qreal widened = 0;
    {
        QQuickView view;
        QVERIFY(loadView(view));
        view.rootObject()->setProperty("editMode", true);

        QQuickItem *const pip = pipOf(view);
        QQuickItem *const grip = gripOf(view);
        QVERIFY(pip && grip);
        QTRY_VERIFY(grip->isVisible());

        const qreal before = pip->width();
        dragMouse(view, itemCenter(grip), itemCenter(grip) + QPoint(80, 0), false);
        QTRY_VERIFY(pip->width() > before);
        widened = pip->width();
        QCOMPARE(pip->height(), widened * 9 / 16);
    }

    QQuickView view2;
    QVERIFY(loadView(view2));
    QQuickItem *const pip2 = pipOf(view2);
    QVERIFY(pip2);
    QTRY_COMPARE(pip2->width(), widened);
}
void PipViewTest::_resizeClampsToViewportFraction()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));
    view.rootObject()->setProperty("editMode", true);

    QQuickItem *const root = view.rootObject();
    QQuickItem *const pip = pipOf(view);
    QQuickItem *const grip = gripOf(view);
    QVERIFY(pip && grip);
    QTRY_VERIFY(grip->isVisible());

    const qreal maxFraction = pip->property("_maxSize").toReal();
    const qreal minFraction = pip->property("_minSize").toReal();
    QVERIFY(maxFraction > 0 && minFraction > 0);

    dragMouse(view, itemCenter(grip), itemCenter(grip) + QPoint(3000, 0), false);
    QTRY_VERIFY(pip->width() <= root->width() * maxFraction + 1);

    dragMouse(view, itemCenter(grip), itemCenter(grip) - QPoint(3000, 0), false);
    QTRY_VERIFY(pip->width() >= root->width() * minFraction - 1);
}
void PipViewTest::_hoverRevealHoldsSteadyOverTheGrip()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const pip = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("pip"));
    QQuickItem *const grip = view.rootObject()->findChild<QQuickItem*>(QStringLiteral("pipResizeGrip"));
    QVERIFY(pip && grip);
    QVERIFY(!grip->isVisible());

    QTest::mouseMove(&view, itemCenter(pip));
    QTRY_VERIFY(grip->isVisible());
    const QPoint onGrip = itemCenter(grip);
    for (int sample = 0; sample < 20; sample++) {
        QTest::mouseMove(&view, onGrip);
        QVERIFY2(grip->isVisible(), qPrintable(QStringLiteral("grip hid on sample %1 while hovered").arg(sample)));
    }
}
void PipViewTest::_gripResizeOutsideEditModeDoesNotSwap()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem* const root = view.rootObject();
    QQuickItem* const pip = pipOf(view);
    QQuickItem* const grip = gripOf(view);
    QQuickItem* const itemA = root->findChild<QQuickItem*>("itemA");
    QQuickItem* const itemB = root->findChild<QQuickItem*>("itemB");
    QVERIFY(pip && grip && itemA && itemB);

    QCOMPARE(itemA->parentItem(), root);
    QVERIFY(itemB->parentItem() != root);

    QTest::mouseMove(&view, itemCenter(pip));
    QTRY_VERIFY(grip->isVisible());

    const qreal before = pip->width();
    dragMouse(view, itemCenter(grip), itemCenter(grip) + QPoint(80, 0), false);
    QTRY_VERIFY(pip->width() > before);

    QCOMPARE(itemA->parentItem(), root);
    QVERIFY(itemB->parentItem() != root);
}
void PipViewTest::_forcingItem1FullOverridesTheSavedArrangement()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));
    QQuickItem *const root = view.rootObject();
    QQuickItem *const pip = root->findChild<QQuickItem*>("pip");
    QQuickItem *const itemA = root->findChild<QQuickItem*>("itemA");
    QQuickItem *const itemB = root->findChild<QQuickItem*>("itemB");
    QVERIFY(pip && itemA && itemB);
    QMetaObject::invokeMethod(pip, "_swapPip");
    QTRY_COMPARE(itemB->parentItem(), root);

    pip->setProperty("forceItem1Full", true);
    QTRY_COMPARE(itemA->parentItem(), root);

    pip->setProperty("forceItem1Full", false);
    QTRY_COMPARE(itemB->parentItem(), root);
}
void PipViewTest::_forcingItem1FullDoesNotDisturbTheSavedArrangement()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));
    QQuickItem *const pip = view.rootObject()->findChild<QQuickItem*>("pip");
    QVERIFY(pip);

    QMetaObject::invokeMethod(pip, "_swapPip");
    const QVariant saved = QSettings().value(QStringLiteral("PipViewTestItem1IsFull"));
    QCOMPARE(saved.toBool(), false);

    pip->setProperty("forceItem1Full", true);
    QCOMPARE(QSettings().value(QStringLiteral("PipViewTestItem1IsFull")), saved);

    pip->setProperty("forceItem1Full", false);
    QCOMPARE(QSettings().value(QStringLiteral("PipViewTestItem1IsFull")), saved);
}

void PipViewTest::_swapIsRefusedWhileItem1IsForcedFull()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));
    QQuickItem *const root = view.rootObject();
    QQuickItem *const pip = root->findChild<QQuickItem*>("pip");
    QQuickItem *const itemA = root->findChild<QQuickItem*>("itemA");
    QVERIFY(pip && itemA);

    pip->setProperty("forceItem1Full", true);
    QTRY_COMPARE(itemA->parentItem(), root);

    QMetaObject::invokeMethod(pip, "_swapPip");
    QCOMPARE(itemA->parentItem(), root);
    QVERIFY(!QSettings().contains(QStringLiteral("PipViewTestItem1IsFull")));
}

void PipViewTest::_gripResizeDoesNotMoveThePanel()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));
    view.rootObject()->setProperty("editMode", true);

    QQuickItem *const pip = pipOf(view);
    QQuickItem *const grip = gripOf(view);
    QVERIFY(pip && grip);
    QTRY_VERIFY(grip->isVisible());

    const qreal left = pip->x();
    const qreal bottom = pip->y() + pip->height();
    const qreal before = pip->width();
    dragMouse(view, itemCenter(grip), itemCenter(grip) + QPoint(80, 0), false);
    QTRY_VERIFY(pip->width() > before);

    QCOMPARE(pip->x(), left);
    QCOMPARE(pip->y() + pip->height(), bottom);
    QVERIFY(!pip->property("hasCustomPosition").toBool());
}

void PipViewTest::_gripResizeNeverReachesTheItemBeneath()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem *const pip = pipOf(view);
    QQuickItem *const grip = gripOf(view);
    QVERIFY(pip && grip);
    QTest::mouseMove(&view, itemCenter(pip));
    QTest::mouseMove(&view, itemCenter(grip));
    QTRY_VERIFY(grip->isVisible());

    const qreal before = pip->width();
    dragMouse(view, itemCenter(grip), itemCenter(grip) + QPoint(120, 0), false);
    QTRY_VERIFY(pip->width() > before);

    QCOMPARE(view.rootObject()->property("fullItemDrags").toInt(), 0);
}
