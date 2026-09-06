#include "ScreenToolsTest.h"
#include "QuickInteractionTestHelpers.h"
#include "ScreenToolsController.h"

namespace {

constexpr qreal kBasePointSize     = 14;
constexpr qreal kDesktopSmallRatio = 0.75;
constexpr qreal kMobileSmallFloor  = 12;
constexpr qreal kMobileTouchFloor  = 48;

class FakeMobileScope
{
public:
    explicit FakeMobileScope(bool mobile) { ScreenToolsController::setFakeMobile(mobile); }
    ~FakeMobileScope() { ScreenToolsController::setFakeMobile(false); }
};

class SystemFontScaleScope
{
public:
    ~SystemFontScaleScope() { ScreenToolsController::setSystemFontScale(1.0); }
};

bool load(QQuickView &view)
{
    return loadTestView(view, QStringLiteral("qrc:/unittest/ScreenToolsTest.qml"));
}

qreal readReal(const QQuickView &view, const char *name)
{
    return view.rootObject()->property(name).toReal();
}

bool setBase(QQuickView &view, qreal pointSize)
{
    return QMetaObject::invokeMethod(view.rootObject(), "setBasePointSize", Q_ARG(QVariant, pointSize));
}

}

void ScreenToolsTest::_desktopKeepsTheDesktopTiers()
{
    FakeMobileScope desktop(false);
    QQuickView view;
    QVERIFY(load(view));

    QVERIFY(setBase(view, kBasePointSize));
    QVERIFY(qFuzzyCompare(readReal(view, "smallPointSize"), kBasePointSize * kDesktopSmallRatio));
    QVERIFY(readReal(view, "minTouchPixels") < kMobileTouchFloor);
}

void ScreenToolsTest::_mobileFloorsTheSmallTierAndTheTouchSize()
{
    FakeMobileScope mobile(true);
    QQuickView view;
    QVERIFY(load(view));

    QVERIFY(setBase(view, kBasePointSize));
    QVERIFY(readReal(view, "smallPointSize") >= kMobileSmallFloor);
    QVERIFY(readReal(view, "minTouchPixels") >= kMobileTouchFloor);

    QVERIFY(setBase(view, 11));
    QVERIFY(readReal(view, "smallPointSize") >= kMobileSmallFloor);
}

void ScreenToolsTest::_systemFontScaleMultipliesTheBase()
{
    FakeMobileScope desktop(false);
    SystemFontScaleScope scale;
    QQuickView view;
    QVERIFY(load(view));

    QVERIFY(setBase(view, kBasePointSize));
    QVERIFY(qFuzzyCompare(readReal(view, "basePointSize"), kBasePointSize));

    ScreenToolsController::setSystemFontScale(1.5);
    QTRY_VERIFY(qFuzzyCompare(readReal(view, "systemFontScale"), 1.5));
    QTRY_VERIFY(qFuzzyCompare(readReal(view, "basePointSize"), kBasePointSize * 1.5));

    ScreenToolsController::setSystemFontScale(1.0);
    QTRY_VERIFY(qFuzzyCompare(readReal(view, "basePointSize"), kBasePointSize));
}
