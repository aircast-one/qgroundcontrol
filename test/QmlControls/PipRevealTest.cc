/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "PipRevealTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtTest/QSignalSpy>
#include <QtTest/QTest>

static void clearPipSettings()
{
    clearQmlGlobalSettings({"IsPIPVisible", "PipViewTestItem1IsFull"});
    clearDragPositionSettings({"PIP"});
}

static bool loadView(QQuickView& view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/PipViewTest.qml"));
}

static bool setExpanded(QQuickItem* pip, bool expanded)
{
    return QMetaObject::invokeMethod(pip, "_setPipIsExpanded", Q_ARG(QVariant, expanded));
}

// A property that animates reports many intermediate values on its way; one that is simply
// switched reports a single change. That difference is the whole of "there is a transition",
// and it is what a reader of the screen actually sees.
void PipRevealTest::_collapsingAnimatesInsteadOfVanishing()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem* const pip = view.rootObject()->findChild<QQuickItem*>("pip");
    QVERIFY(pip);
    QQuickItem* const content = view.rootObject()->findChild<QQuickItem*>("pipContent");
    QVERIFY(content);
    QCOMPARE(content->opacity(), 1.0);
    QCOMPARE(content->scale(), 1.0);

    QSignalSpy opacitySteps(content, SIGNAL(opacityChanged()));
    QSignalSpy scaleSteps(content, SIGNAL(scaleChanged()));

    QVERIFY(setExpanded(pip, false));
    QTRY_VERIFY_WITH_TIMEOUT(qFuzzyIsNull(content->opacity()), 3000);

    QVERIFY2(opacitySteps.count() > 3,
             qPrintable(QStringLiteral("opacity reached 0 in %1 step(s): it was switched, not animated").arg(opacitySteps.count())));
    QVERIFY2(scaleSteps.count() > 3,
             qPrintable(QStringLiteral("scale reached its collapsed value in %1 step(s)").arg(scaleSteps.count())));

    // It shrinks toward the corner the toggle occupies rather than shrinking to nothing.
    QVERIFY(content->scale() < 1.0);
    QVERIFY(content->scale() > 0.5);
}

// The collapsed and expanded states used to be two different buttons in two different places.
// Whichever state the pip is in there must be exactly one of them, and it must turn to face
// the other way rather than be swapped for a differently drawn one.
void PipRevealTest::_oneToggleTurnsToServeBothStates()
{
    clearPipSettings();

    QQuickView view;
    QVERIFY(loadView(view));

    QQuickItem* const pip = view.rootObject()->findChild<QQuickItem*>("pip");
    QVERIFY(pip);

    QCOMPARE(view.rootObject()->findChildren<QQuickItem*>("pipToggle").count(), 1);

    QQuickItem* const chevron = view.rootObject()->findChild<QQuickItem*>("pipToggleChevron");
    QVERIFY(chevron);
    QCOMPARE(chevron->rotation(), 0.0);

    QSignalSpy rotationSteps(chevron, SIGNAL(rotationChanged()));

    QVERIFY(setExpanded(pip, false));
    QTRY_VERIFY_WITH_TIMEOUT(qFuzzyCompare(chevron->rotation(), 180.0), 3000);

    QVERIFY2(rotationSteps.count() > 3,
             qPrintable(QStringLiteral("the chevron snapped to 180 in %1 step(s) instead of turning").arg(rotationSteps.count())));
    QCOMPARE(view.rootObject()->findChildren<QQuickItem*>("pipToggle").count(), 1);

    QVERIFY(setExpanded(pip, true));
    QTRY_VERIFY_WITH_TIMEOUT(qFuzzyIsNull(chevron->rotation()), 3000);
}
