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
    clearQmlGlobalSettings({"PIPSize", "PIPCustomPosition", "PIPPositionX", "PIPPositionY", "IsPIPVisible", "PipViewTestItem1IsFull"});
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

        const QPointF expected(kMargin + target.x() - start.x(),
                               root->height() - pip->height() - kMargin + target.y() - start.y());
        QTRY_VERIFY(qAbs(pip->x() - expected.x()) < 2);
        QTRY_VERIFY(qAbs(pip->y() - expected.y()) < 2);

        QCOMPARE(itemB->parentItem(), pipParentBefore);

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

    // Aim well past the bottom-right corner; the drag itself is bounded by the
    // MouseArea drag min/max and the committed position must stay fully on screen.
    dragMouse(view, itemCenter(pip), QPoint(root->width() + 200, root->height() + 200));

    QTRY_COMPARE(pip->x(), root->width() - pip->width());
    QTRY_COMPARE(pip->y(), root->height() - pip->height());
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

void PipViewTest::_resizeFromCornerPersists()
{
    clearPipSettings();

    qreal resizedWidth = 0;

    {
        QQuickView view;
        QVERIFY(loadView(view));

        QQuickItem* pip = view.rootObject()->findChild<QQuickItem*>("pip");
        QVERIFY(pip);

        const qreal initialWidth = pip->width();
        const QPoint corner = pip->mapToScene(QPointF(pip->width() - 10, 10)).toPoint();
        dragMouse(view, corner, corner + QPoint(80, 0), false);

        QTRY_VERIFY(qAbs(pip->width() - (initialWidth + 80)) < 2);
        QTRY_VERIFY(qAbs(pip->height() - pip->width() * 9 / 16) < 2);

        // Pulling the handle upward must also grow the pip, with the bottom edge pinned.
        const qreal widthBeforeUpDrag = pip->width();
        const qreal bottomBeforeUpDrag = pip->y() + pip->height();
        const QPoint corner2 = pip->mapToScene(QPointF(pip->width() - 10, 10)).toPoint();
        dragMouse(view, corner2, corner2 + QPoint(0, -60), false);

        QTRY_VERIFY(qAbs(pip->width() - (widthBeforeUpDrag + 60)) < 2);
        QVERIFY(qAbs(pip->y() + pip->height() - bottomBeforeUpDrag) < 1);

        resizedWidth = pip->width();
    }

    QQuickView view2;
    QVERIFY(loadView(view2));
    QQuickItem* pip2 = view2.rootObject()->findChild<QQuickItem*>("pip");
    QVERIFY(pip2);
    QCOMPARE(pip2->width(), resizedWidth);
}
