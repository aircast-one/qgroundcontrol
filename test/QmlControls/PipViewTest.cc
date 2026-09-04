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
        const QPointF expected(snapToDropGrid(kMargin + target.x() - start.x(), grid),
                               snapToDropGrid(root->height() - pip->height() - kMargin + target.y() - start.y(), grid));
        QTRY_VERIFY(qAbs(pip->x() - expected.x()) < 2);
        QTRY_VERIFY(qAbs(pip->y() - expected.y()) < 2);

        QCOMPARE(itemB->parentItem(), pipParentBefore);

        QTRY_VERIFY(storedPosition(QStringLiteral("PIP800x600-CustomPosition")));
        draggedPos = QPointF(pip->x(), pip->y());
    }

    QQuickView view2;
    QVERIFY(loadView(view2));
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

    QQuickItem* root = view.rootObject();
    QQuickItem* pip = root->findChild<QQuickItem*>("pip");
    QVERIFY(pip);

    // Aim at the far corner of the window rather than past it: Qt drops mouse events
    // delivered outside the target window, so a drag aimed off-screen simply stops early and
    // never reaches the bound it is meant to prove. drag.maximumX/Y still do the clamping.
    dragMouse(view, itemCenter(pip), QPoint(root->width() - 1, root->height() - 1));

    // Dropped positions are held edgeMargin away from the edge rather than flush against it,
    // so the clamped corner is inset by exactly that much.
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

        QQuickItem* root = view.rootObject();
        QQuickItem* pip = root->findChild<QQuickItem*>("pip");
        QVERIFY(pip);

        const QPointF defaultPos(kMargin, root->height() - pip->height() - kMargin);

        dragMouse(view, itemCenter(pip), QPoint(500, 250));
        QTRY_VERIFY(pip->x() != defaultPos.x());

        // Drop it a few pixels from the default corner: it must snap exactly back.
        const QPoint offsetFromDefault(pip->x() - defaultPos.x() - 4, pip->y() - defaultPos.y() - 4);
        dragMouse(view, itemCenter(pip), itemCenter(pip) - offsetFromDefault);

        QTRY_COMPARE(pip->x(), defaultPos.x());
        QTRY_COMPARE(pip->y(), defaultPos.y());
    }

    // The reset must persist: a fresh view starts at the default position.
    QQuickView view2;
    QVERIFY(loadView(view2));
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

