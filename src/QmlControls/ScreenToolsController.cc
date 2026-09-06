/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/


/// @file
/// @author Gus Grubba <gus@auterion.com>

#include "ScreenToolsController.h"
#include "QGCApplication.h"
#include "QGCLoggingCategory.h"
#include "SettingsManager.h"
#include "AppSettings.h"

#include <QtGui/QCursor>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontMetrics>
#include <QtGui/QInputDevice>

#include <atomic>
#include <optional>

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
QList<ScreenToolsController*>& instances()
{
    static QList<ScreenToolsController*> *const list = new QList<ScreenToolsController*>;
    return *list;
}
}

ScreenToolsController::ScreenToolsController(QObject *parent)
    : QObject(parent)
{
    instances().append(this);
    // qCDebug(ScreenToolsControllerLog) << Q_FUNC_INFO << this;
}

ScreenToolsController::~ScreenToolsController()
{
    instances().removeAll(this);
    // qCDebug(ScreenToolsControllerLog) << Q_FUNC_INFO << this;
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

#if defined(Q_OS_MACOS) || defined(Q_OS_IOS)
    return QFontDatabase::systemFont(QFontDatabase::GeneralFont).family();
#else
    return QStringLiteral("Open Sans");
#endif
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
    for (ScreenToolsController *const instance : std::as_const(instances())) {
        QMetaObject::invokeMethod(instance, [instance] { emit instance->systemFontScaleChanged(); }, Qt::QueuedConnection);
    }
}
