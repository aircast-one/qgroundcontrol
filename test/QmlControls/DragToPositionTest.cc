/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "DragToPositionTest.h"
#include "QuickInteractionTestHelpers.h"

#include <cmath>

static const qreal kMargin = 8;

static void clearPanelSettings()
{
    clearDragPositionSettings({"TestPanel"});
}

static bool loadView(QQuickView& view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/DragToPositionTest.qml"));
}
static void dragPanel(QQuickView& view, const QPoint& from, const QPoint& to)
{
    dragMouse(view, from, to, false);
}

void DragToPositionTest::_dragRepositionsAndPersists()
{
    clearPanelSettings();

    QPointF draggedPos;

    {
        QQuickView view;
        QVERIFY(loadView(view));

        QQuickItem* root = view.rootObject();
        QQuickItem* panel = root->findChild<QQuickItem*>("panel");
        QVERIFY(panel);

        QCOMPARE(panel->x(), root->width() - panel->width() - kMargin);
        QCOMPARE(panel->y(), root->height() - panel->height() - kMargin);

        const QPoint start = itemCenter(panel);
        const QPoint target(300, 200);
        dragPanel(view, start, target);

        const qreal grid = dropGridOf(root);
        const qreal margin = dropMarginOf(root);
        const QPointF expected(snapToDropGrid(root->width() - panel->width() - kMargin + target.x() - start.x(), panel->width(), root->width(), grid, margin),
                               snapToDropGrid(root->height() - panel->height() - kMargin + target.y() - start.y(), panel->height(), root->height(), grid, margin));
        QTRY_VERIFY(qAbs(panel->x() - expected.x()) < 2);
        QTRY_VERIFY(qAbs(panel->y() - expected.y()) < 2);

        QTRY_VERIFY(storedPosition(QStringLiteral("TestPanel800x600-CustomPosition")));
        draggedPos = QPointF(panel->x(), panel->y());
    }

    QQuickView view2;
    QVERIFY(loadView(view2));
    QQuickItem* panel2 = view2.rootObject()->findChild<QQuickItem*>("panel");
    QVERIFY(panel2);
    QCOMPARE(panel2->x(), draggedPos.x());
    QCOMPARE(panel2->y(), draggedPos.y());
}

void DragToPositionTest::_dragOffscreenClampsCommittedPosition()
{
    clearPanelSettings();

    qreal edgeMargin = 0;

    {
        QQuickView view;
        QVERIFY(loadView(view));

        QQuickItem* root = view.rootObject();
        QQuickItem* panel = root->findChild<QQuickItem*>("panel");
        QVERIFY(panel);

        QObject* dragPosition = root->findChild<QObject*>("dragPosition");
        QVERIFY(dragPosition);
        edgeMargin = dragPosition->property("edgeMargin").toReal();
        QVERIFY(edgeMargin > 0);
        dragPanel(view, itemCenter(panel), QPoint(-300, -300));

        QTRY_COMPARE(panel->x(), edgeMargin);
        QTRY_COMPARE(panel->y(), edgeMargin);
        QTRY_VERIFY(storedPosition(QStringLiteral("TestPanel800x600-CustomPosition")));
    }
    QQuickView view2;
    QVERIFY(loadView(view2));
    QQuickItem* panel2 = view2.rootObject()->findChild<QQuickItem*>("panel");
    QVERIFY(panel2);
    QCOMPARE(panel2->x(), edgeMargin);
    QCOMPARE(panel2->y(), edgeMargin);
}

void DragToPositionTest::_dragBackToDefaultSnapsAndResets()
{
    clearPanelSettings();

    {
        QQuickView view;
        QVERIFY(loadView(view));

        QQuickItem* root = view.rootObject();
        QQuickItem* panel = root->findChild<QQuickItem*>("panel");
        QVERIFY(panel);

        const QPointF defaultPos(root->width() - panel->width() - kMargin,
                                 root->height() - panel->height() - kMargin);

        dragPanel(view, itemCenter(panel), QPoint(300, 200));
        QTRY_VERIFY(panel->x() != defaultPos.x());
        const QPoint offsetFromDefault(panel->x() - defaultPos.x() + qRound(panel->width() * 0.4),
                                       panel->y() - defaultPos.y() + qRound(panel->height() * 0.4));
        dragPanel(view, itemCenter(panel), itemCenter(panel) - offsetFromDefault);

        QTRY_COMPARE(panel->x(), defaultPos.x());
        QTRY_COMPARE(panel->y(), defaultPos.y());
    }
    QQuickView view2;
    QVERIFY(loadView(view2));
    QQuickItem* root2 = view2.rootObject();
    QQuickItem* panel2 = root2->findChild<QQuickItem*>("panel");
    QVERIFY(panel2);
    QCOMPARE(panel2->x(), root2->width() - panel2->width() - kMargin);
    QCOMPARE(panel2->y(), root2->height() - panel2->height() - kMargin);
}

void DragToPositionTest::_clickStillReachesChild()
{
    clearPanelSettings();

    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem* panel = view.rootObject()->findChild<QQuickItem*>("panel");
    QVERIFY(panel);

    const QPointF before(panel->x(), panel->y());
    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(panel));

    QTRY_COMPARE(panel->property("childClicks").toInt(), 1);
    QCOMPARE(QPointF(panel->x(), panel->y()), before);

    QQuickItem* button = panel->findChild<QQuickItem*>("panelButton");
    QVERIFY(button);
    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(button));
    QTRY_COMPARE(panel->property("buttonClicks").toInt(), 1);
}

void DragToPositionTest::_comboBoxInPanelOpensPopup()
{
    clearPanelSettings();
    clearDragPositionSettings({"TestSelectable"});

    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem* panel = view.rootObject()->findChild<QQuickItem*>("selectablePanel");
    QQuickItem* combo = view.rootObject()->findChild<QQuickItem*>("combo");
    QVERIFY(panel);
    QVERIFY(combo);
    QQuickItem* content = panel->property("contentItem").value<QQuickItem*>();
    QVERIFY(content);
    const QPointF defaultPos(panel->x(), panel->y());
    dragMouse(view, itemCenter(content), itemCenter(content) + QPoint(250, 60), false);
    QTRY_VERIFY(panel->x() != defaultPos.x());
    panel->setProperty("showSelectionUI", true);
    QTRY_VERIFY(combo->isVisible() && combo->width() > 0);
    const QPointF posBeforeClick(panel->x(), panel->y());
    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(combo));

    QObject* popup = combo->property("popup").value<QObject*>();
    QVERIFY(popup);
    QTRY_VERIFY(popup->property("visible").toBool());
    QCOMPARE(QPointF(panel->x(), panel->y()), posBeforeClick);
}


void DragToPositionTest::_dropsSnapToTheGridFromTheNearestEdge()
{
    clearPanelSettings();

    QQuickView view;
    QVERIFY(loadView(view));
    QQuickItem* root = view.rootObject();
    QQuickItem* panel = root->findChild<QQuickItem*>("panel");
    QVERIFY(panel);
    const qreal grid = dropGridOf(root);
    const qreal margin = dropMarginOf(root);
    QVERIFY(grid > 0);

    dragPanel(view, itemCenter(panel), QPoint(230, 170));
    QTRY_VERIFY(panel->x() < root->width() / 2);
    QCOMPARE(std::fmod(panel->x() - margin, grid), 0.0);
    QCOMPARE(std::fmod(panel->y() - margin, grid), 0.0);

    dragPanel(view, itemCenter(panel), QPoint(root->width() - 130, root->height() - 90));
    QTRY_VERIFY(panel->x() > root->width() / 2);
    QCOMPARE(std::fmod(root->width() - margin - (panel->x() + panel->width()), grid), 0.0);
    QCOMPARE(std::fmod(root->height() - margin - (panel->y() + panel->height()), grid), 0.0);
}

void DragToPositionTest::_defaultHomeKeepsItsOwnLayoutOffTheGrid()
{
    clearDragPositionSettings({"TestSelectable"});

    QQuickView view;
    QVERIFY(loadView(view));
    QQuickItem* root = view.rootObject();
    QQuickItem* panel = root->findChild<QQuickItem*>("selectablePanel");
    QVERIFY(panel);
    const qreal grid = dropGridOf(root);
    const qreal margin = dropMarginOf(root);
    QVERIFY(grid > 0);
    QVERIFY(std::fmod(10.0 - margin, grid) != 0.0);
    QVERIFY(std::fmod(root->height() - margin - (380.0 + panel->height()), grid) != 0.0);

    QCOMPARE(panel->x(), 10.0);
    QCOMPARE(panel->y(), 380.0);
}
