/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "ScreenToolsController.h"
#include "PlatformTheme.h"
#if defined(Q_OS_IOS)
#include "iOSSafeArea.h"
#include <QtGui/QGuiApplication>
#include <QtGui/QScreen>
#include <QtGui/QWindow>
#endif
#include "QGCApplication.h"
#include "QGCLoggingCategory.h"
#include "SettingsManager.h"
#ifdef Q_OS_ANDROID
#include "AndroidInterface.h"
#endif
#include "AppSettings.h"

#include <QtGui/QCursor>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontMetrics>
#include <QtGui/QGuiApplication>
#include <QtGui/QInputDevice>

#include <atomic>
#include <optional>
#include <QtCore/QMetaObject>
#include <atomic>
#include <algorithm>
#include <QtCore/QList>

#if defined(Q_OS_ANDROID)
#include "AndroidInterface.h"
#endif

#if defined(Q_OS_IOS)
#include <sys/utsname.h>
#endif

QGC_LOGGING_CATEGORY(ScreenToolsControllerLog, "qgc.qmlcontrols.screentoolscontroller")

namespace {
    std::atomic<int> s_systemFontScaleMilli(0);
    std::optional<bool> s_fakeMobileOverride;
    QList<ScreenToolsController*> s_instances;
    std::atomic<int> s_safeAreaLeft(0);
    std::atomic<int> s_safeAreaTop(0);
    std::atomic<int> s_safeAreaRight(0);
    std::atomic<int> s_safeAreaBottom(0);

    qreal toLogical(int devicePixels)
    {
        const qreal ratio = qGuiApp ? qGuiApp->devicePixelRatio() : 1.0;
        return (ratio > 0.0) ? (devicePixels / ratio) : devicePixels;
    }
}

qreal ScreenToolsController::safeAreaLeft() { return toLogical(s_safeAreaLeft); }
qreal ScreenToolsController::safeAreaTop() { return toLogical(s_safeAreaTop); }
qreal ScreenToolsController::safeAreaRight() { return toLogical(s_safeAreaRight); }
qreal ScreenToolsController::safeAreaBottom() { return toLogical(s_safeAreaBottom); }

void ScreenToolsController::setSafeAreaInsets(int left, int top, int right, int bottom)
{
    const int clampedLeft   = std::max(0, left);
    const int clampedTop    = std::max(0, top);
    const int clampedRight  = std::max(0, right);
    const int clampedBottom = std::max(0, bottom);

    const bool leftChanged   = s_safeAreaLeft.exchange(clampedLeft) != clampedLeft;
    const bool topChanged    = s_safeAreaTop.exchange(clampedTop) != clampedTop;
    const bool rightChanged  = s_safeAreaRight.exchange(clampedRight) != clampedRight;
    const bool bottomChanged = s_safeAreaBottom.exchange(clampedBottom) != clampedBottom;

    if (!(leftChanged || topChanged || rightChanged || bottomChanged) || !qGuiApp) {
        return;
    }

    QMetaObject::invokeMethod(qGuiApp, []() {
        for (ScreenToolsController *const controller : s_instances) {
            emit controller->safeAreaChanged();
        }
    }, Qt::QueuedConnection);
}

ScreenToolsController::ScreenToolsController(QObject *parent)
    : QObject(parent)
{
#if defined(Q_OS_IOS)
    iOSSafeArea::report();
    connect(qGuiApp, &QGuiApplication::focusWindowChanged, this, [](QWindow *) { iOSSafeArea::report(); });
    connect(qGuiApp->primaryScreen(), &QScreen::orientationChanged, this, [](Qt::ScreenOrientation) { iOSSafeArea::report(); });
#endif
    s_instances.append(this);
#ifdef Q_OS_ANDROID
    if (s_instances.count() == 1) {
        AndroidInterface::setSafeAreaHandler(&ScreenToolsController::setSafeAreaInsets);
    }
#endif
}

ScreenToolsController::~ScreenToolsController()
{
    s_instances.removeAll(this);
#ifdef Q_OS_ANDROID
    if (s_instances.isEmpty()) {
        AndroidInterface::setSafeAreaHandler(nullptr);
    }
#endif
}

int ScreenToolsController::mouseX()
{
    return QCursor::pos().x();
}

int ScreenToolsController::mouseY()
{
    return QCursor::pos().y();
}

bool ScreenToolsController::hasTouch()
{
    for (const auto &inputDevice: QInputDevice::devices()) {
        if (inputDevice->type() == QInputDevice::DeviceType::TouchScreen) {
            return true;
        }
    }
    return false;
}

QString ScreenToolsController::iOSDevice()
{
#if defined(Q_OS_IOS)
    struct utsname systemInfo;
    uname(&systemInfo);
    return QString(systemInfo.machine);
#else
    return QString();
#endif
}

qreal ScreenToolsController::controlRadiusRatio()
{
    return PlatformTheme::instance()->shape().controlRadiusRatio;
}

bool ScreenToolsController::capsuleControls()
{
    return PlatformTheme::instance()->shape().capsuleControls;
}

QString ScreenToolsController::fixedFontFamily()
{
    return QFontDatabase::systemFont(QFontDatabase::FixedFont).family();
}

QString ScreenToolsController::normalFontFamily()
{
    const int langID = SettingsManager::instance()->appSettings()->qLocaleLanguage()->rawValue().toInt();
    if (langID == QLocale::Korean) {
        return QStringLiteral("NanumGothic");
    }

    return PlatformTheme::instance()->fontFamily();
}

double ScreenToolsController::defaultFontDescent(int pointSize)
{
    return QFontMetrics(QFont(normalFontFamily(), pointSize)).descent();
}

#if !defined(Q_OS_ANDROID) && !defined(Q_OS_IOS)
bool ScreenToolsController::fakeMobile()
{
    return s_fakeMobileOverride.value_or(qgcApp()->fakeMobile());
}
#endif

void ScreenToolsController::setFakeMobile(bool fake)
{
    s_fakeMobileOverride = fake;
}

qreal ScreenToolsController::systemFontScale()
{
    if (s_systemFontScaleMilli == 0) {
#if defined(Q_OS_ANDROID)
        s_systemFontScaleMilli = qRound(AndroidInterface::systemFontScale() * 1000);
#else
        s_systemFontScaleMilli = 1000;
#endif
    }
    return s_systemFontScaleMilli / 1000.0;
}

void ScreenToolsController::setSystemFontScale(qreal scale)
{
    const int milli = scale > 0 ? qRound(scale * 1000) : 1000;
    if (s_systemFontScaleMilli.exchange(milli) == milli) {
        return;
    }
    for (ScreenToolsController *const instance : std::as_const(s_instances)) {
        QMetaObject::invokeMethod(instance, [instance] { emit instance->systemFontScaleChanged(); }, Qt::QueuedConnection);
    }
}
