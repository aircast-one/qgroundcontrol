#include "SafeAreaTest.h"
#include "QuickInteractionTestHelpers.h"
#include "ScreenToolsController.h"

#include <QtTest/QSignalSpy>
#include <QtGui/QGuiApplication>

namespace {

qreal logical(int devicePixels)
{
    return devicePixels / qGuiApp->devicePixelRatio();
}

}

void SafeAreaTest::cleanup()
{
    ScreenToolsController::setSafeAreaInsets(0, 0, 0, 0);
}

void SafeAreaTest::_insetsArriveInLogicalPixels()
{
    ScreenToolsController::setSafeAreaInsets(80, 40, 20, 10);

    QCOMPARE(ScreenToolsController::safeAreaLeft(), logical(80));
    QCOMPARE(ScreenToolsController::safeAreaTop(), logical(40));
    QCOMPARE(ScreenToolsController::safeAreaRight(), logical(20));
    QCOMPARE(ScreenToolsController::safeAreaBottom(), logical(10));
}

void SafeAreaTest::_negativeInsetsAreClamped()
{
    ScreenToolsController::setSafeAreaInsets(-80, -40, -20, -10);

    QCOMPARE(ScreenToolsController::safeAreaLeft(), 0.0);
    QCOMPARE(ScreenToolsController::safeAreaTop(), 0.0);
    QCOMPARE(ScreenToolsController::safeAreaRight(), 0.0);
    QCOMPARE(ScreenToolsController::safeAreaBottom(), 0.0);
}

void SafeAreaTest::_repeatedInsetsDoNotNotify()
{
    ScreenToolsController controller;
    QSignalSpy spy(&controller, &ScreenToolsController::safeAreaChanged);
    QTest::qWait(50);
    spy.clear();

    ScreenToolsController::setSafeAreaInsets(80, 0, 0, 0);
    QVERIFY(spy.wait());
    QCOMPARE(spy.count(), 1);

    ScreenToolsController::setSafeAreaInsets(80, 0, 0, 0);
    QVERIFY(!spy.wait(100));
    QCOMPARE(spy.count(), 1);

    ScreenToolsController::setSafeAreaInsets(80, 33, 0, 0);
    QVERIFY(spy.wait());
    QCOMPARE(spy.count(), 2);
}

void SafeAreaTest::_backdropSpansTheWindowWhileChromeStaysInside()
{
    QQuickView view;
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/SafeAreaTest.qml")));

    QQuickItem *const root = view.rootObject();
    QQuickItem *const backdrop = findItemByName(root, QStringLiteral("backdrop"));
    QQuickItem *const chrome = findItemByName(root, QStringLiteral("chrome"));
    QVERIFY(backdrop);
    QVERIFY(chrome);

    ScreenToolsController::setSafeAreaInsets(80, 40, 20, 10);
    QVERIFY(QTest::qWaitFor([chrome] { return chrome->x() > 0; }));

    const QPointF backdropOrigin = backdrop->mapToItem(root, QPointF(0, 0));
    QCOMPARE(backdropOrigin, QPointF(0, 0));
    QCOMPARE(backdrop->width(), root->width());
    QCOMPARE(backdrop->height(), root->height());

    const QPointF chromeOrigin = chrome->mapToItem(root, QPointF(0, 0));
    QCOMPARE(chromeOrigin.x(), logical(80));
    QCOMPARE(chromeOrigin.y(), logical(40));
    QCOMPARE(chrome->width(), root->width() - logical(80) - logical(20));
    QCOMPARE(chrome->height(), root->height() - logical(40) - logical(10));
}

void SafeAreaTest::_chromeInsetNeverGoesNegative()
{
    QQuickView view;
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/SafeAreaTest.qml")));

    QQuickItem *const root = view.rootObject();
    QQuickItem *const chrome = findItemByName(root, QStringLiteral("chrome"));
    QVERIFY(chrome);

    root->setProperty("barHeight", 0);
    ScreenToolsController::setSafeAreaInsets(0, 120, 0, 0);
    QVERIFY(QTest::qWaitFor([chrome] { return chrome->y() > 0; }));

    QCOMPARE(root->property("topEdgeInset").toReal(), 0.0);
}
