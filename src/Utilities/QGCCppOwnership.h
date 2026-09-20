#pragma once

#ifdef QGC_HEADLESS_CORE

class QObject;

inline void qgcCppOwnership(QObject *) {}

#else

#include <QtQml/QQmlEngine>

inline void qgcCppOwnership(QObject *object)
{
    QQmlEngine::setObjectOwnership(object, QQmlEngine::CppOwnership);
}

#endif
