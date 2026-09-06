/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "OverlayViewSwitchTest.h"
#include "QuickInteractionTestHelpers.h"

static bool load(QQuickView &view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/OverlayViewSwitchTest.qml"));
}

static QQuickItem *optionAt(const QQuickView &view, int index)
{
    return findItemByName(view.rootObject(), QStringLiteral("viewSwitchOption%1").arg(index));
}

static void tapOption(QQuickView &view, int index)
{
    QTRY_VERIFY(optionAt(view, index) != nullptr);
    QQuickItem *const option = optionAt(view, index);
    QTRY_VERIFY(option->width() > 0);
    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(option));
}

void OverlayViewSwitchTest::_tapReportsTheOptionTapped()
{
    QQuickView view;
    QVERIFY(load(view));

    tapOption(view, 1);

    QCOMPARE(view.rootObject()->property("activatedCount").toInt(), 1);
    QCOMPARE(view.rootObject()->property("lastActivated").toInt(), 1);
}

void OverlayViewSwitchTest::_tapLeavesTheOwnersBindingIntact()
{
    QQuickView view;
    QVERIFY(load(view));

    tapOption(view, 1);

    view.rootObject()->setProperty("backingIndex", 1);
    QCOMPARE(view.rootObject()->property("currentIndex").toInt(), 1);

    view.rootObject()->setProperty("backingIndex", 0);
    QCOMPARE(view.rootObject()->property("currentIndex").toInt(), 0);
}

void OverlayViewSwitchTest::_dragPastTheMidpointSwitches()
{
    QQuickView view;
    QVERIFY(load(view));

    QQuickItem *const from = optionAt(view, 0);
    QQuickItem *const to = optionAt(view, 1);
    QVERIFY(from && to);

    dragMouse(view, itemCenter(from), itemCenter(to));

    QCOMPARE(view.rootObject()->property("activatedCount").toInt(), 1);
    QCOMPARE(view.rootObject()->property("lastActivated").toInt(), 1);
}

void OverlayViewSwitchTest::_dragShortOfTheMidpointSpringsBack()
{
    QQuickView view;
    QVERIFY(load(view));

    QQuickItem *const from = optionAt(view, 0);
    QVERIFY(from);
    const QPoint start = itemCenter(from);
    const QPoint shortOfMidpoint = start + QPoint(from->width() / 4, 0);

    dragMouse(view, start, shortOfMidpoint);

    QCOMPARE(view.rootObject()->property("activatedCount").toInt(), 0);
    QTRY_COMPARE(view.rootObject()->property("thumbX").toReal(), from->x());
}

void OverlayViewSwitchTest::_tapOnTheCurrentOptionReportsReselectedNotActivated()
{
    QQuickView view;
    QVERIFY(load(view));

    tapOption(view, 0);

    QCOMPARE(view.rootObject()->property("reselectedCount").toInt(), 1);
    QCOMPARE(view.rootObject()->property("lastReselected").toInt(), 0);
    QCOMPARE(view.rootObject()->property("activatedCount").toInt(), 0);
}
