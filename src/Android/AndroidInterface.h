/****************************************************************************
 *
 * Copyright (C) 2018 Pinecone Inc. All rights reserved.
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include <QtCore/QString>
#include <QtCore/QLoggingCategory>
#include <QtGui/QColor>

#include <jni.h>

#include <functional>

Q_DECLARE_LOGGING_CATEGORY(AndroidInterfaceLog)

namespace AndroidInterface
{
    bool cleanJavaException();
    jclass getActivityClass();
    void setNativeMethods();
    void jniLogDebug(JNIEnv *envA, jobject thizA, jstring messageA);
    void jniLogWarning(JNIEnv *envA, jobject thizA, jstring messageA);
    void jniDeepLink(JNIEnv *envA, jobject thizA, jstring urlA);
    void jniFontScaleChanged(JNIEnv *envA, jobject thizA, jfloat scaleA);
    void jniSafeAreaInsets(JNIEnv *envA, jobject thizA, jint leftA, jint topA, jint rightA, jint bottomA);
    using SafeAreaHandler = std::function<void(int left, int top, int right, int bottom)>;
    void setSafeAreaHandler(SafeAreaHandler handler);
    QString getLaunchDeepLink();
bool isEmbeddedHost();
    qreal systemFontScale();
    bool checkStoragePermissions();
    QString getSDCardPath();
    void setKeepScreenOn(bool on);
    QColor systemColor(const QString &resourceName);
    void setSystemBarAppearance(bool lightBars);

    constexpr const char *kJniQGCActivityClassName = "org/mavlink/qgroundcontrol/QGCActivity";
};
