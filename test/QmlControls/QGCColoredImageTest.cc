/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "QGCColoredImageTest.h"
#include "QuickInteractionTestHelpers.h"

#include <QtQuick/QQuickItem>
#include <QtTest/QTest>

#include <algorithm>

// The alpha of `color` selects between two renderings that look identical in code and nothing
// alike on screen: zero alpha means "leave the source art alone" (QGCToolBarButton's logo mode),
// any other alpha tints and dims. Getting it wrong renders an invisible icon with no error, so
// which child is showing, and at what opacity, is pinned here.
namespace {

QQuickItem *coloredImage(QQuickView &view, const char *property)
{
    return view.rootObject()->property(property).value<QQuickItem*>();
}

bool isImage(const QQuickItem *item)
{
    return QString::fromLatin1(item->metaObject()->className()).startsWith(QStringLiteral("QQuickImage"));
}

QQuickItem *sourceImage(QQuickItem *root)
{
    const QList<QQuickItem*> children = root->childItems();
    const auto found = std::find_if(children.cbegin(), children.cend(), [](const QQuickItem *item) { return isImage(item); });
    return found == children.cend() ? nullptr : *found;
}

QQuickItem *tintOverlay(QQuickItem *root)
{
    const QList<QQuickItem*> children = root->childItems();
    const auto found = std::find_if(children.cbegin(), children.cend(), [](const QQuickItem *item) { return !isImage(item); });
    return found == children.cend() ? nullptr : *found;
}

constexpr qreal kFadedAlpha    = 0.35;
constexpr qreal kAlphaTolerance = 0.01;

bool load(QQuickView &view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/QGCColoredImageTest.qml"));
}

} // namespace

void QGCColoredImageTest::_aFullyTransparentColorLeavesTheSourceArtUntinted()
{
    QQuickView view;
    QVERIFY(load(view));

    QQuickItem *const root = coloredImage(view, "untinted");
    QVERIFY(root);
    QCOMPARE(root->childItems().size(), 2);
    QQuickItem *const image = sourceImage(root);
    QVERIFY(image);
    QVERIFY2(image->isVisible(), "a transparent tint must show the source image, not nothing");
    QVERIFY2(!tintOverlay(root)->isVisible(), "an untinted image must not also draw the tint layer");
}

void QGCColoredImageTest::_anOpaqueColorTintsAtFullStrength()
{
    QQuickView view;
    QVERIFY(load(view));

    QQuickItem *const root = coloredImage(view, "opaque");
    QVERIFY(root);
    QCOMPARE(root->childItems().size(), 2);
    QQuickItem *const image = sourceImage(root);
    QVERIFY(image);
    QVERIFY2(!image->isVisible(), "a tinted image must not draw the untinted source underneath");
    QVERIFY2(tintOverlay(root)->isVisible(), "a tinted image must draw the tint layer");
    QCOMPARE(tintOverlay(root)->opacity(), 1.0);
}

void QGCColoredImageTest::_aPartlyTransparentColorTintsAtThatStrength()
{
    QQuickView view;
    QVERIFY(load(view));

    QQuickItem *const root = coloredImage(view, "faded");
    QVERIFY(root);
    QCOMPARE(root->childItems().size(), 2);

    const QColor color = root->property("color").value<QColor>();
    QVERIFY2(qAbs(color.alphaF() - kFadedAlpha) < kAlphaTolerance, "the tint alpha did not survive the round trip");

    QQuickItem *const overlay = tintOverlay(root);
    QVERIFY2(overlay, "expected a ColorOverlay child to carry the tint");
    QVERIFY2(qAbs(overlay->opacity() - kFadedAlpha) < kAlphaTolerance,
             "the tint must be drawn at the alpha of the requested color");
}
