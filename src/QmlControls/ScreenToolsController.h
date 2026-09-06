/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include <QtCore/QLoggingCategory>
#include <QtCore/QObject>
#include <QtQmlIntegration/QtQmlIntegration>

Q_DECLARE_LOGGING_CATEGORY(ScreenToolsControllerLog)

class ScreenToolsController : public QObject
{
    Q_OBJECT
    Q_PROPERTY(bool     isAndroid           READ isAndroid          CONSTANT)
    Q_PROPERTY(bool     isiOS               READ isiOS              CONSTANT)
    Q_PROPERTY(bool     isMobile            READ isMobile           CONSTANT)
    Q_PROPERTY(bool     fakeMobile          READ fakeMobile         CONSTANT)
    Q_PROPERTY(qreal    systemFontScale     READ systemFontScale    NOTIFY systemFontScaleChanged)
    Q_PROPERTY(bool     isDebug             READ isDebug            CONSTANT)
    Q_PROPERTY(bool     isMacOS             READ isMacOS            CONSTANT)
    Q_PROPERTY(bool     isLinux             READ isLinux            CONSTANT)
    Q_PROPERTY(bool     isWindows           READ isWindows          CONSTANT)
    Q_PROPERTY(bool     isSerialAvailable   READ isSerialAvailable  CONSTANT)
    Q_PROPERTY(bool     hasTouch            READ hasTouch           CONSTANT)
    Q_PROPERTY(QString  iOSDevice           READ iOSDevice          CONSTANT)
    Q_PROPERTY(QString  fixedFontFamily     READ fixedFontFamily    CONSTANT)
    Q_PROPERTY(QString  normalFontFamily    READ normalFontFamily   CONSTANT)
    Q_PROPERTY(qreal    controlRadiusRatio  READ controlRadiusRatio CONSTANT)
    Q_PROPERTY(bool     capsuleControls     READ capsuleControls    CONSTANT)
    Q_PROPERTY(qreal    safeAreaLeft        READ safeAreaLeft       NOTIFY safeAreaChanged)
    Q_PROPERTY(qreal    safeAreaTop         READ safeAreaTop        NOTIFY safeAreaChanged)
    Q_PROPERTY(qreal    safeAreaRight       READ safeAreaRight      NOTIFY safeAreaChanged)
    Q_PROPERTY(qreal    safeAreaBottom      READ safeAreaBottom     NOTIFY safeAreaChanged)

public:
    explicit ScreenToolsController(QObject *parent = nullptr);
    ~ScreenToolsController();

    Q_INVOKABLE static int mouseX();
    Q_INVOKABLE static int mouseY();

    Q_INVOKABLE static double defaultFontDescent(int pointSize);

    static qreal systemFontScale();
    static void setSystemFontScale(qreal scale);
    static void setFakeMobile(bool fake);

#if defined(Q_OS_ANDROID) || defined(Q_OS_IOS)
    static bool isMobile() { return true;  }
    static bool fakeMobile() { return false; }
#else
    static bool isMobile() { return fakeMobile(); }
    static bool fakeMobile();
#endif

#if defined (Q_OS_ANDROID)
    static bool isAndroid() { return true;  }
    static bool isiOS() { return false; }
    static bool isLinux() { return false; }
    static bool isMacOS() { return false; }
    static bool isWindows() { return false; }
#elif defined(Q_OS_IOS)
    static bool isAndroid() { return false; }
    static bool isiOS() { return true; }
    static bool isLinux() { return false; }
    static bool isMacOS() { return false; }
    static bool isWindows() { return false; }
#elif defined(Q_OS_MACOS)
    static bool isAndroid() { return false; }
    static bool isiOS() { return false; }
    static bool isLinux() { return false; }
    static bool isMacOS() { return true; }
    static bool isWindows() { return false; }
#elif defined(Q_OS_LINUX)
    static bool isAndroid() { return false; }
    static bool isiOS() { return false; }
    static bool isLinux() { return true; }
    static bool isMacOS() { return false; }
    static bool isWindows() { return false; }
#elif defined(Q_OS_WIN)
    static bool isAndroid() { return false; }
    static bool isiOS() { return false; }
    static bool isLinux() { return false; }
    static bool isMacOS() { return false; }
    static bool isWindows() { return true; }
#else
    static bool isAndroid() { return false; }
    static bool isiOS() { return false; }
    static bool isLinux() { return false; }
    static bool isMacOS() { return false; }
    static bool isWindows() { return false; }
#endif

#if defined(QGC_NO_SERIAL_LINK)
    static bool isSerialAvailable() { return false; }
#else
    static bool isSerialAvailable() { return true; }
#endif

#ifdef QT_DEBUG
    static bool isDebug() { return true; }
#else
    static bool isDebug() { return false; }
#endif

    static bool hasTouch();
    static QString iOSDevice();
    static qreal controlRadiusRatio();
    static bool capsuleControls();

    static QString fixedFontFamily();
    static QString normalFontFamily();

    static qreal safeAreaLeft();
    static qreal safeAreaTop();
    static qreal safeAreaRight();
    static qreal safeAreaBottom();
    static void setSafeAreaInsets(int left, int top, int right, int bottom);

signals:
    void safeAreaChanged();
    void systemFontScaleChanged();
};
