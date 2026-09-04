/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "GCSBattery.h"

#if defined(Q_OS_MACOS)
#include <CoreFoundation/CoreFoundation.h>
#include <IOKit/ps/IOPSKeys.h>
#include <IOKit/ps/IOPowerSources.h>
#elif defined(Q_OS_WIN)
#include <windows.h>
#elif defined(Q_OS_ANDROID)
#include <QtCore/QJniObject>
#include <QtCore/qcoreapplication_platform.h>
#elif defined(Q_OS_LINUX)
#include <QtCore/QDir>
#include <QtCore/QFile>
#endif

namespace {

#if defined(Q_OS_MACOS)

int cfInt(CFDictionaryRef dict, CFStringRef key)
{
    const CFNumberRef number = static_cast<CFNumberRef>(CFDictionaryGetValue(dict, key));
    int value = 0;
    if (!number || !CFNumberGetValue(number, kCFNumberIntType, &value)) {
        return -1;
    }
    return value;
}

void readBattery(int &level, bool &charging)
{
    const CFTypeRef blob = IOPSCopyPowerSourcesInfo();
    if (!blob) {
        return;
    }

    const CFArrayRef sources = IOPSCopyPowerSourcesList(blob);
    if (sources) {
        for (CFIndex i = 0; i < CFArrayGetCount(sources); ++i) {
            const CFDictionaryRef desc = IOPSGetPowerSourceDescription(blob, CFArrayGetValueAtIndex(sources, i));
            if (!desc) {
                continue;
            }
            const int current = cfInt(desc, CFSTR(kIOPSCurrentCapacityKey));
            const int max = cfInt(desc, CFSTR(kIOPSMaxCapacityKey));
            if ((current < 0) || (max <= 0)) {
                continue;
            }
            level = qBound(0, (current * 100) / max, 100);
            charging = (CFDictionaryGetValue(desc, CFSTR(kIOPSIsChargingKey)) == kCFBooleanTrue);
            break;
        }
        CFRelease(sources);
    }

    CFRelease(blob);
}

#elif defined(Q_OS_WIN)

void readBattery(int &level, bool &charging)
{
    SYSTEM_POWER_STATUS status = {};
    if (!GetSystemPowerStatus(&status)) {
        return;
    }
    if ((status.BatteryFlag & 128) || (status.BatteryLifePercent > 100)) {
        return;
    }
    level = status.BatteryLifePercent;
    charging = (status.ACLineStatus == 1);
}

#elif defined(Q_OS_ANDROID)

jint intExtra(const QJniObject &intent, const char *name, jint fallback)
{
    return intent.callMethod<jint>("getIntExtra", "(Ljava/lang/String;I)I",
                                   QJniObject::fromString(QLatin1String(name)).object<jstring>(), fallback);
}

void readBattery(int &level, bool &charging)
{
    const QJniObject context = QNativeInterface::QAndroidApplication::context();
    if (!context.isValid()) {
        return;
    }

    const QJniObject filter("android/content/IntentFilter", "(Ljava/lang/String;)V",
                            QJniObject::fromString(QStringLiteral("android.intent.action.BATTERY_CHANGED")).object<jstring>());
    const QJniObject intent = context.callObjectMethod(
        "registerReceiver",
        "(Landroid/content/BroadcastReceiver;Landroid/content/IntentFilter;)Landroid/content/Intent;",
        static_cast<jobject>(nullptr), filter.object());
    if (!intent.isValid()) {
        return;
    }

    const jint raw = intExtra(intent, "level", -1);
    const jint scale = intExtra(intent, "scale", -1);
    const jint status = intExtra(intent, "status", -1);
    if ((raw < 0) || (scale <= 0)) {
        return;
    }

    level = qBound(0, (raw * 100) / scale, 100);
    charging = (status == 2 /* BATTERY_STATUS_CHARGING */) || (status == 5 /* BATTERY_STATUS_FULL */);
}

#elif defined(Q_OS_LINUX)

QString readSysFile(const QString &path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return QString();
    }
    return QString::fromLatin1(file.readAll()).trimmed();
}

void readBattery(int &level, bool &charging)
{
    const QDir dir(QStringLiteral("/sys/class/power_supply"));
    const QStringList entries = dir.entryList(QDir::Dirs | QDir::NoDotAndDotDot);
    for (const QString &entry : entries) {
        if (readSysFile(dir.filePath(entry + QStringLiteral("/type"))) != QStringLiteral("Battery")) {
            continue;
        }
        bool ok = false;
        const int capacity = readSysFile(dir.filePath(entry + QStringLiteral("/capacity"))).toInt(&ok);
        if (!ok) {
            continue;
        }
        level = qBound(0, capacity, 100);
        const QString status = readSysFile(dir.filePath(entry + QStringLiteral("/status")));
        charging = (status == QStringLiteral("Charging")) || (status == QStringLiteral("Full"));
        break;
    }
}

#else

void readBattery(int &, bool &) {}

#endif

} // namespace

GCSBattery::GCSBattery(QObject *parent)
    : QObject(parent)
{
    (void) connect(&_timer, &QTimer::timeout, this, &GCSBattery::_poll);
    _timer.start(30000);
    _poll();
}

void GCSBattery::_poll()
{
    int level = -1;
    bool charging = false;
    readBattery(level, charging);

    if ((level == _level) && (charging == _charging)) {
        return;
    }

    _level = level;
    _charging = charging;
    emit stateChanged();
}
