/****************************************************************************
 *
 * Copyright (C) 2018 Pinecone Inc. All rights reserved.
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "AndroidInterface.h"
#include "QGCApplication.h"
#include "QGCLoggingCategory.h"

#include <QtCore/QJniObject>
#include <QtCore/QJniEnvironment>
#include <QtCore/QMetaObject>
#include <QtCore/QUrl>

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
        {"nativeDeepLink", "(Ljava/lang/String;)V", reinterpret_cast<void *>(jniDeepLink)}
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
    // Call the Java method to check and request storage permissions
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
        return 1.0;
    }
    const QJniObject resources = context.callObjectMethod("getResources", "()Landroid/content/res/Resources;");
    (void) cleanJavaException();
    if (!resources.isValid()) {
        return 1.0;
    }
    const QJniObject configuration = resources.callObjectMethod("getConfiguration", "()Landroid/content/res/Configuration;");
    (void) cleanJavaException();
    if (!configuration.isValid()) {
        return 1.0;
    }
    const float scale = configuration.getField<jfloat>("fontScale");
    (void) cleanJavaException();
    return scale > 0 ? scale : 1.0;
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

void setKeepScreenOn(bool on)
{
    Q_UNUSED(on);

    //-- Screen is locked on while QGC is running on Android
}

} // namespace AndroidInterface
