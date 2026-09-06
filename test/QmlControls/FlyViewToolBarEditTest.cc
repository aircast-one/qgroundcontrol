/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "FlyViewToolBarEditTest.h"
#include "QuickInteractionTestHelpers.h"

static bool load(QQuickView &view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/FlyViewToolBarEditTest.qml"));
}

static void setHidden(QQuickView &view, const QString &key, bool hidden)
{
    QMetaObject::invokeMethod(view.rootObject(), "setHidden", Q_ARG(QVariant, key), Q_ARG(QVariant, hidden));
}

void FlyViewToolBarEditTest::_hiddenToolComesBackAsAGhostInEditMode()
{
    QQuickView view;
    QVERIFY(load(view));

    QQuickItem *const button = findItemByName(view.rootObject(), QStringLiteral("settingsButton"));
    QVERIFY(button);
    QQuickItem *const slot = button->parentItem();
    QVERIFY(slot);
    QTRY_VERIFY(button->isVisible());

    setHidden(view, QStringLiteral("settingsButton"), true);
    QTRY_VERIFY(!button->isVisible());

    view.rootObject()->setProperty("editMode", true);
    QTRY_VERIFY(button->isVisible());
    QVERIFY(slot->opacity() < 1);

    setHidden(view, QStringLiteral("settingsButton"), false);
    QTRY_COMPARE(slot->opacity(), 1.0);
}

// Takes away every spare pixel between the left cluster and the tool buttons, plus half the
// status pill, so the bar has to give ground no matter what the font metrics are.
static void narrowUntilTheStatusPillGivesGround(QQuickView &view, QQuickItem *pill, QQuickItem *analyze)
{
    const qreal clusterRight = pill->mapToScene(QPointF(pill->width(), 0)).x();
    const qreal slack = analyze->mapToScene(QPointF(0, 0)).x() - clusterRight;
    const qreal narrowWidth = view.rootObject()->width() - slack - pill->width() / 2;
    view.rootObject()->setSize(QSizeF(narrowWidth, view.rootObject()->height()));
    view.resize(qRound(narrowWidth), qRound(view.rootObject()->height()));
}

void FlyViewToolBarEditTest::_narrowBarKeepsEveryToolClearOfTheButtons()
{
    QQuickView view;
    QVERIFY(load(view));

    QQuickItem *const pill = findItemByName(view.rootObject(), QStringLiteral("mainStatusPill"));
    QQuickItem *const analyze = findItemByName(view.rootObject(), QStringLiteral("analyzeButton"));
    QVERIFY(pill && analyze);

    QQuickItem *const label = pill->parentItem();
    QVERIFY(label);
    QTRY_VERIFY(pill->width() >= label->implicitWidth() - 1);
    const qreal naturalWidth = pill->width();

    narrowUntilTheStatusPillGivesGround(view, pill, analyze);

    QTRY_VERIFY(pill->width() < naturalWidth);
    const qreal pillRight = pill->mapToScene(QPointF(pill->width(), 0)).x();
    const qreal analyzeLeft = analyze->mapToScene(QPointF(0, 0)).x();
    QVERIFY(pillRight <= analyzeLeft);
}

void FlyViewToolBarEditTest::_narrowBarDropsTheIndicatorRow()
{
    QQuickView view;
    QVERIFY(load(view));

    QQuickItem *const pill = findItemByName(view.rootObject(), QStringLiteral("mainStatusPill"));
    QQuickItem *const indicators = findItemByName(view.rootObject(), QStringLiteral("flyViewToolIndicators"));
    QQuickItem *const analyze = findItemByName(view.rootObject(), QStringLiteral("analyzeButton"));
    QVERIFY(pill && indicators && analyze);
    QTRY_VERIFY(indicators->width() > 0);

    narrowUntilTheStatusPillGivesGround(view, pill, analyze);

    QTRY_COMPARE(indicators->width(), 0.0);
}

void FlyViewToolBarEditTest::_editModeSwallowsTapsOnTheStatusPill()
{
    QQuickView view;
    QVERIFY(load(view));

    QQuickItem *const pill = findItemByName(view.rootObject(), QStringLiteral("mainStatusPill"));
    QVERIFY(pill);
    QTRY_VERIFY(pill->width() > 0);

    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(pill));
    QTRY_COMPARE(view.rootObject()->property("drawerCount").toInt(), 1);

    view.rootObject()->setProperty("editMode", true);
    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, itemCenter(pill));
    QCOMPARE(view.rootObject()->property("drawerCount").toInt(), 1);
}
