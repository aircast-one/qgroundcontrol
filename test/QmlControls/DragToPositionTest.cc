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

static const qreal kMargin = 8;

static void clearPanelSettings()
{
    clearQmlGlobalSettings({"TestPanelCustomPosition", "TestPanelPositionX", "TestPanelPositionY"});
}

static bool loadView(QQuickView& view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/DragToPositionTest.qml"));
}

// DragHandler applies the full translation from the press point once its threshold is
// exceeded, so unlike the MouseArea drag in PipViewTest no anchor compensation is needed.
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

        const QPointF expected(root->width() - panel->width() - kMargin + target.x() - start.x(),
                               root->height() - panel->height() - kMargin + target.y() - start.y());
        QTRY_VERIFY(qAbs(panel->x() - expected.x()) < 2);
        QTRY_VERIFY(qAbs(panel->y() - expected.y()) < 2);

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

    {
        QQuickView view;
        QVERIFY(loadView(view));

        QQuickItem* root = view.rootObject();
        QQuickItem* panel = root->findChild<QQuickItem*>("panel");
        QVERIFY(panel);

        // Drag far past the top-left corner: the DragHandler itself is unbounded, but the
        // committed position must clamp back on screen.
        dragPanel(view, itemCenter(panel), QPoint(-300, -300));

        QTRY_COMPARE(panel->x(), 0.0);
        QTRY_COMPARE(panel->y(), 0.0);
    }

    // The clamped position must also be what a fresh view restores to.
    QQuickView view2;
    QVERIFY(loadView(view2));
    QQuickItem* panel2 = view2.rootObject()->findChild<QQuickItem*>("panel");
    QVERIFY(panel2);
    QCOMPARE(panel2->x(), 0.0);
    QCOMPARE(panel2->y(), 0.0);
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

        // Drop it well off the default corner but overlapping more than half the panel:
        // it must snap exactly back.
        const QPoint offsetFromDefault(panel->x() - defaultPos.x() + qRound(panel->width() * 0.4),
                                       panel->y() - defaultPos.y() + qRound(panel->height() * 0.4));
        dragPanel(view, itemCenter(panel), itemCenter(panel) - offsetFromDefault);

        QTRY_COMPARE(panel->x(), defaultPos.x());
        QTRY_COMPARE(panel->y(), defaultPos.y());
    }

    // The reset must persist: a fresh view starts at the default position.
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
    clearQmlGlobalSettings({"TestSelectableCustomPosition", "TestSelectablePositionX", "TestSelectablePositionY"});

    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem* panel = view.rootObject()->findChild<QQuickItem*>("selectablePanel");
    QQuickItem* combo = view.rootObject()->findChild<QQuickItem*>("combo");
    QVERIFY(panel);
    QVERIFY(combo);

    // Dragging by the content area moves the panel.
    QQuickItem* content = panel->property("contentItem").value<QQuickItem*>();
    QVERIFY(content);
    const QPointF defaultPos(panel->x(), panel->y());
    dragMouse(view, itemCenter(content), itemCenter(content) + QPoint(250, 60), false);
    QTRY_VERIFY(panel->x() != defaultPos.x());

    // Selection UI shown: the combo popup must open and the panel must not move.
    panel->setProperty("showSelectionUI", true);
    QTRY_VERIFY(combo->isVisible() && combo->width() > 0);
    const QPointF posBeforeClick(panel->x(), panel->y());
    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(combo));

    QObject* popup = combo->property("popup").value<QObject*>();
    QVERIFY(popup);
    QTRY_VERIFY(popup->property("visible").toBool());
    QCOMPARE(QPointF(panel->x(), panel->y()), posBeforeClick);
}

void DragToPositionTest::_resizeHandleGrowsUpAndPinsBottom()
{
    clearPanelSettings();

    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem* root = view.rootObject();
    QQuickItem* panel = root->findChild<QQuickItem*>("panel");
    QQuickItem* handle = root->findChild<QQuickItem*>("resizeHandle");
    QVERIFY(panel);
    QVERIFY(handle);

    // Move off the default corner first so the bottom edge is not held by the default binding.
    const qreal defaultX = panel->x();
    dragPanel(view, itemCenter(panel), QPoint(300, 200));
    QTRY_VERIFY(panel->x() != defaultX);

    const qreal initialWidth = panel->width();
    const qreal initialBottom = panel->y() + panel->height();

    // Pull straight up: dominant-axis growth, bottom edge stays pinned.
    const QPoint grip = itemCenter(handle);
    dragMouse(view, grip, grip - QPoint(0, 60), false);

    QTRY_VERIFY(qAbs(panel->width() - (initialWidth + 60)) < 2);
    QVERIFY(qAbs(panel->y() + panel->height() - initialBottom) < 1);

    // Pull down-left (growth is the dominant axis, so shrinking needs both): bottom still pinned.
    const QPoint grip2 = itemCenter(handle);
    dragMouse(view, grip2, grip2 + QPoint(-40, 40), false);

    QTRY_VERIFY(qAbs(panel->width() - (initialWidth + 20)) < 2);
    QVERIFY(qAbs(panel->y() + panel->height() - initialBottom) < 1);
}
