/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "OverlaySegmentedControlTest.h"
#include "QuickInteractionTestHelpers.h"

static bool load(QQuickView &view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/OverlaySegmentedControlTest.qml"));
}

static QQuickItem *segmentAt(const QQuickView &view, int index)
{
    return findItemByName(view.rootObject(), QStringLiteral("segment%1").arg(index));
}

// Changing which segments are enabled rebuilds the delegates, and a tap aimed at where a segment
// used to be lands on whatever has not been laid out yet. Wait for the segment itself.
static void tapSegment(QQuickView &view, int index)
{
    QTRY_VERIFY(segmentAt(view, index) != nullptr);
    QQuickItem *const segment = segmentAt(view, index);
    QTRY_VERIFY(segment->width() > 0);
    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(segment));
}

void OverlaySegmentedControlTest::_tapReportsTheSegmentTapped()
{
    QQuickView view;
    QVERIFY(load(view));

    tapSegment(view, 1);

    QCOMPARE(view.rootObject()->property("activatedCount").toInt(), 1);
    QCOMPARE(view.rootObject()->property("lastActivated").toInt(), 1);
}

// The control reports; the owner decides. It used to assign its own currentIndex on every tap,
// which overwrote whatever binding the owner had set it from - so the plan view's layer selector
// stopped following the layer as soon as anyone touched it, and only looked right because the
// value it wrote happened to match.
void OverlaySegmentedControlTest::_tapLeavesTheOwnersBindingIntact()
{
    QQuickView view;
    QVERIFY(load(view));

    tapSegment(view, 1);

    view.rootObject()->setProperty("backingIndex", 2);
    QCOMPARE(view.rootObject()->property("currentIndex").toInt(), 2);
}

void OverlaySegmentedControlTest::_disabledSegmentReportsNothing()
{
    QQuickView view;
    QVERIFY(load(view));

    tapSegment(view, 2);
    QCOMPARE(view.rootObject()->property("activatedCount").toInt(), 0);

    view.rootObject()->setProperty("thirdEnabled", true);
    QTRY_VERIFY(segmentAt(view, 2) && segmentAt(view, 2)->property("_isEnabled").toBool());

    tapSegment(view, 2);
    QCOMPARE(view.rootObject()->property("activatedCount").toInt(), 1);
    QCOMPARE(view.rootObject()->property("lastActivated").toInt(), 2);
}
