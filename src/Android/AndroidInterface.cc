/****************************************************************************
 *
 * Copyright (C) 2018 Pinecone Inc. All rights reserved.
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "AndroidInterface.h"
#include "QGCBridge.h"
#include "QGCApplication.h"
#include "QGCLoggingCategory.h"
#include "ScreenToolsController.h"

#include <QtCore/QJniObject>
#include <QtCore/QJniEnvironment>
#include <QtCore/QMetaObject>
#include <QtCore/QUrl>
#include <mutex>

QGC_LOGGING_CATEGORY(AndroidInterfaceLog, "qgc.android.src.androidinterface")

namespace AndroidInterface
{

bool cleanJavaException()
{
    QJniEnvironment jniEnv;
    const bool result = jniEnv.checkAndClearExceptions();
    return result;
}

jclass getActivityClass()
{
    static jclass javaClass = nullptr;

    if (!javaClass) {
        QJniEnvironment env;
        if (!env.isValid()) {
            qCWarning(AndroidInterfaceLog) << "Invalid QJniEnvironment";
            return nullptr;
        }

        if (!QJniObject::isClassAvailable(kJniQGCActivityClassName)) {
            qCWarning(AndroidInterfaceLog) << "Class Not Available";
            return nullptr;
        }

        javaClass = env.findClass(kJniQGCActivityClassName);
        if (!javaClass) {
            qCWarning(AndroidInterfaceLog) << "Class Not Found";
            return nullptr;
        }

        env.checkAndClearExceptions();
    }

    return javaClass;
}

void setNativeMethods()
{
    qCDebug(AndroidInterfaceLog) << "Registering Native Functions";

    JNINativeMethod javaMethods[] {
        {"qgcLogDebug",   "(Ljava/lang/String;)V", reinterpret_cast<void *>(jniLogDebug)},
        {"qgcLogWarning", "(Ljava/lang/String;)V", reinterpret_cast<void *>(jniLogWarning)},
        {"nativeDeepLink", "(Ljava/lang/String;)V", reinterpret_cast<void *>(jniDeepLink)},
        {"nativeSafeAreaInsets", "(IIII)V", reinterpret_cast<void *>(jniSafeAreaInsets)},
        {"nativeFontScaleChanged", "(F)V", reinterpret_cast<void *>(jniFontScaleChanged)}
    };

    (void) AndroidInterface::cleanJavaException();

    jclass objectClass = AndroidInterface::getActivityClass();
    if(!objectClass) {
        qCWarning(AndroidInterfaceLog) << "Couldn't find class:" << objectClass;
        return;
    }

    QJniEnvironment jniEnv;
    jint val = jniEnv->RegisterNatives(objectClass, javaMethods, std::size(javaMethods));

    if (val < 0) {
        qCWarning(AndroidInterfaceLog) << "Error registering methods:" << val;
    } else {
        qCDebug(AndroidInterfaceLog) << "Native Functions Registered";
    }

    (void) AndroidInterface::cleanJavaException();
}

void jniLogDebug(JNIEnv *envA, jobject thizA, jstring messageA)
{
    Q_UNUSED(thizA);

    const char * const stringL = envA->GetStringUTFChars(messageA, nullptr);
    const QString logMessage = QString::fromUtf8(stringL);
    envA->ReleaseStringUTFChars(messageA, stringL);
    (void) QJniEnvironment::checkAndClearExceptions(envA);
    qCDebug(AndroidInterfaceLog) << logMessage;
}

void jniLogWarning(JNIEnv *envA, jobject thizA, jstring messageA)
{
    Q_UNUSED(thizA);

    const char * const stringL = envA->GetStringUTFChars(messageA, nullptr);
    const QString logMessage = QString::fromUtf8(stringL);
    envA->ReleaseStringUTFChars(messageA, stringL);
    (void) QJniEnvironment::checkAndClearExceptions(envA);
    qCWarning(AndroidInterfaceLog) << logMessage;
}

void jniDeepLink(JNIEnv *envA, jobject thizA, jstring urlA)
{
    Q_UNUSED(thizA);

    const char * const stringL = envA->GetStringUTFChars(urlA, nullptr);
    const QString url = QString::fromUtf8(stringL);
    envA->ReleaseStringUTFChars(urlA, stringL);
    (void) QJniEnvironment::checkAndClearExceptions(envA);

    QGCApplication *app = qgcApp();
    if (!app) {
        qCWarning(AndroidInterfaceLog) << "Deep link received before app ready" << url;
        return;
    }

    (void) QMetaObject::invokeMethod(app, [app, url]() {
        app->handleDeepLink(QUrl(url));
    }, Qt::QueuedConnection);
}

void jniFontScaleChanged(JNIEnv *envA, jobject thizA, jfloat scaleA)
{
    Q_UNUSED(envA);
    Q_UNUSED(thizA);
    ScreenToolsController::setSystemFontScale(scaleA);
}

bool isEmbeddedHost()
{
    QJniEnvironment env;
    const QJniObject context = QNativeInterface::QAndroidApplication::context();
    const jclass qtActivityClass = env.findClass("org/qtproject/qt/android/bindings/QtActivity");
    return context.isValid() && qtActivityClass && !env->IsInstanceOf(context.object(), qtActivityClass);
}

QString getLaunchDeepLink()
{
    const QJniObject activity = QNativeInterface::QAndroidApplication::context();
    if (!activity.isValid()) {
        return QString();
    }

    const QJniObject intent = activity.callObjectMethod("getIntent", "()Landroid/content/Intent;");
    (void) cleanJavaException();
    if (!intent.isValid()) {
        return QString();
    }

    const QJniObject action = intent.callObjectMethod("getAction", "()Ljava/lang/String;");
    if (!action.isValid() || action.toString() != QStringLiteral("android.intent.action.VIEW")) {
        return QString();
    }

    const QJniObject data = intent.callObjectMethod("getData", "()Landroid/net/Uri;");
    if (!data.isValid()) {
        return QString();
    }

    const QJniObject url = data.callObjectMethod("toString", "()Ljava/lang/String;");
    (void) cleanJavaException();
    if (!url.isValid()) {
        return QString();
    }

    return url.toString();
}

bool checkStoragePermissions()
{
    const bool hasPermission = QJniObject::callStaticMethod<jboolean>(
        kJniQGCActivityClassName, 
        "checkStoragePermissions", 
        "()Z"
    );
    
    if (hasPermission) {
        qCDebug(AndroidInterfaceLog) << "Storage permissions granted";
    } else {
        qCWarning(AndroidInterfaceLog) << "Storage permissions not granted";
    }
    
    return hasPermission;
}

qreal systemFontScale()
{
    const QJniObject context = QNativeInterface::QAndroidApplication::context();
    if (!context.isValid()) {
        qCWarning(AndroidInterfaceLog) << "No Android context; assuming font scale 1.0";
        return 1.0;
    }
    const QJniObject resources = context.callObjectMethod("getResources", "()Landroid/content/res/Resources;");
    (void) cleanJavaException();
    if (!resources.isValid()) {
        qCWarning(AndroidInterfaceLog) << "Resources unavailable; assuming font scale 1.0";
        return 1.0;
    }
    const QJniObject configuration = resources.callObjectMethod("getConfiguration", "()Landroid/content/res/Configuration;");
    (void) cleanJavaException();
    if (!configuration.isValid()) {
        qCWarning(AndroidInterfaceLog) << "Configuration unavailable; assuming font scale 1.0";
        return 1.0;
    }
    const float scale = configuration.getField<jfloat>("fontScale");
    if (cleanJavaException() || !(scale > 0)) {
        qCWarning(AndroidInterfaceLog) << "Configuration.fontScale unreadable; assuming font scale 1.0";
        return 1.0;
    }
    return scale;
}

QString getSDCardPath()
{
    if (!checkStoragePermissions()) {
        qCWarning(AndroidInterfaceLog) << "Storage Permission Denied";
        return QString();
    }

    const QJniObject result = QJniObject::callStaticObjectMethod(kJniQGCActivityClassName, "getSDCardPath", "()Ljava/lang/String;");
    if (!result.isValid()) {
        qCWarning(AndroidInterfaceLog) << "Call to java getSDCardPath failed: Invalid Result";
        return QString();
    }

    return result.toString();
}

QColor systemColor(const QString &resourceName)
{
    const QJniObject activity = QNativeInterface::QAndroidApplication::context();
    if (!activity.isValid()) {
        return QColor();
    }

    const QJniObject resources = activity.callObjectMethod("getResources", "()Landroid/content/res/Resources;");
    if (!resources.isValid()) {
        return QColor();
    }

    const jint resourceId = resources.callMethod<jint>(
        "getIdentifier",
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
        QJniObject::fromString(resourceName).object<jstring>(),
        QJniObject::fromString(QStringLiteral("color")).object<jstring>(),
        QJniObject::fromString(QStringLiteral("android")).object<jstring>());
    (void) cleanJavaException();

    if (resourceId == 0) {
        return QColor();
    }

    const jint argb = activity.callMethod<jint>("getColor", "(I)I", resourceId);
    if (cleanJavaException()) {
        return QColor();
    }

    return QColor::fromRgb(static_cast<QRgb>(static_cast<unsigned int>(argb)));
}

namespace {
    struct SafeAreaInsets {
        int left = 0;
        int top = 0;
        int right = 0;
        int bottom = 0;
        bool known = false;
    };

    std::mutex safeAreaMutex;
    AndroidInterface::SafeAreaHandler safeAreaHandler;
    SafeAreaInsets safeAreaInsets;
}

void setSafeAreaHandler(SafeAreaHandler handler)
{
    SafeAreaHandler replay;
    SafeAreaInsets insets;
    {
        const std::lock_guard<std::mutex> lock(safeAreaMutex);
        safeAreaHandler = std::move(handler);
        insets = safeAreaInsets;
        if (safeAreaHandler && insets.known) {
            replay = safeAreaHandler;
        }
    }

    if (replay) {
        qCDebug(AndroidInterfaceLog) << "Replaying window insets to new handler";
        replay(insets.left, insets.top, insets.right, insets.bottom);
    }
}

void jniSafeAreaInsets(JNIEnv *envA, jobject thizA, jint leftA, jint topA, jint rightA, jint bottomA)
{
    Q_UNUSED(envA);
    Q_UNUSED(thizA);

    qCDebug(AndroidInterfaceLog) << "Window insets" << leftA << topA << rightA << bottomA;

    SafeAreaHandler handler;
    {
        const std::lock_guard<std::mutex> lock(safeAreaMutex);
        safeAreaInsets = SafeAreaInsets { leftA, topA, rightA, bottomA, true };
        handler = safeAreaHandler;
    }

    if (handler) {
        handler(leftA, topA, rightA, bottomA);
    }
}

void setSystemBarAppearance(bool lightBars)
{
    if (QGCBridge::setSystemBarAppearance(lightBars)) {
        return;
    }

    QJniObject::callStaticMethod<void>(kJniQGCActivityClassName, "setSystemBarAppearance", "(Z)V", static_cast<jboolean>(lightBars));
    (void) cleanJavaException();
}

void setKeepScreenOn(bool on)
{
    Q_UNUSED(on);

}

}
