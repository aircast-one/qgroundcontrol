/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "OverlayBackdropTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtQuick/QQuickItem>
#include <QtTest/QTest>

// Which backdrop a glass surface may sample is decided by ancestry, and getting it wrong fails
// silently: sample a backdrop that contains you and the render is a feedback loop, sample the
// wrong layer and the material shows content that is not behind it. Both happened before the
// rule was moved into one place, so the rule is pinned here.
namespace {

QObject *resolve(QQuickView &view, const char *property)
{
    return view.rootObject()->property(property).value<QObject*>();
}

QObject *expected(QQuickView &view, const char *property)
{
    return view.rootObject()->property(property).value<QObject*>();
}

bool load(QQuickView &view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/OverlayBackdropTest.qml"));
}

} // namespace

// A surface living inside the captured content cannot sample it without drawing itself into its
// own input. It gets nothing, and the caller has to supply a local backdrop or go without.
void OverlayBackdropTest::_surfaceInsideTheContentLayerGetsNoBackdrop()
{
    QQuickView view;
    QVERIFY(load(view));
    QCOMPARE(resolve(view, "resolvedInsideContent"), nullptr);
}

// Chrome sitting over the map and video - camera controls, the video rail - may sample the
// content, but never the layer it is itself part of.
void OverlayBackdropTest::_surfaceOverTheContentLayerSamplesContentOnly()
{
    QQuickView view;
    QVERIFY(load(view));
    QCOMPARE(resolve(view, "resolvedInsideFull"), expected(view, "contentBackdropItem"));
}

// Anything outside the view entirely - the toolbar, popups, the telemetry chips - is in no
// captured layer, so it samples content plus the instruments drawn over it.
void OverlayBackdropTest::_surfaceOutsideTheViewSamplesEverything()
{
    QQuickView view;
    QVERIFY(load(view));
    QCOMPARE(resolve(view, "resolvedOutside"), expected(view, "fullBackdropItem"));
}

void OverlayBackdropTest::_nothingResolvesWithoutARegisteredBackdrop()
{
    QQuickView view;
    QVERIFY(load(view));
    QCOMPARE(resolve(view, "resolvedNothing"), nullptr);
}
