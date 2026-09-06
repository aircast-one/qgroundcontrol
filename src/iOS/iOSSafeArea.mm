#include "iOSSafeArea.h"
#include "ScreenToolsController.h"

#import <UIKit/UIKit.h>

namespace iOSSafeArea
{

void report()
{
    UIWindow *window = nil;
    for (UIScene *scene in UIApplication.sharedApplication.connectedScenes) {
        if (![scene isKindOfClass:UIWindowScene.class]) {
            continue;
        }
        for (UIWindow *candidate in ((UIWindowScene *)scene).windows) {
            if (candidate.isKeyWindow) {
                window = candidate;
                break;
            }
        }
        if (window) {
            break;
        }
    }
    if (!window) {
        return;
    }

    const UIEdgeInsets insets = window.safeAreaInsets;
    const CGFloat scale = window.screen.scale;
    ScreenToolsController::setSafeAreaInsets(static_cast<int>(insets.left * scale),
                                             static_cast<int>(insets.top * scale),
                                             static_cast<int>(insets.right * scale),
                                             static_cast<int>(insets.bottom * scale));
}

} // namespace iOSSafeArea
