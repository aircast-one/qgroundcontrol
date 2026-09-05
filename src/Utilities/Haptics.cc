#include "Haptics.h"

#ifdef Q_OS_ANDROID
#include <QtCore/QCoreApplication>
#include <QtCore/QJniObject>
#endif

void Haptics::tap()
{
#ifdef Q_OS_ANDROID
    QNativeInterface::QAndroidApplication::runOnAndroidMainThread([] {
        const QJniObject activity = QNativeInterface::QAndroidApplication::context();
        const QJniObject window = activity.callObjectMethod("getWindow", "()Landroid/view/Window;");
        const QJniObject view = window.callObjectMethod("getDecorView", "()Landroid/view/View;");
        view.callMethod<jboolean>("performHapticFeedback", "(I)Z", 0);
    });
#endif
}
