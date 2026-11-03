#if os(macOS)
import SwiftUI

// Stand-ins for iOS-only SwiftUI and UIKit spellings, so code shared with the iOS app compiles unchanged on the
// native macOS app. A Mac has no on-screen keyboard, status bar or navigation bar, so most of these do nothing.

private let COVER_MIN_WIDTH: CGFloat = 520
private let COVER_MIN_HEIGHT: CGFloat = 560

enum UIKeyboardType: Int, Hashable, Sendable {
    case `default`
    case asciiCapable
    case numbersAndPunctuation
    case URL
    case numberPad
    case phonePad
    case namePhonePad
    case emailAddress
    case decimalPad
    case twitter
    case webSearch
    case asciiCapableNumberPad
}

enum TextInputAutocapitalization: Hashable, Sendable {
    case never
    case words
    case sentences
    case characters
}

enum NavigationBarTitleDisplayMode: Hashable, Sendable {
    case automatic
    case inline
    case large
}

extension View {
    func keyboardType(_ type: UIKeyboardType) -> some View { self }

    func textInputAutocapitalization(_ autocapitalization: TextInputAutocapitalization?) -> some View { self }

    func navigationBarTitleDisplayMode(_ displayMode: NavigationBarTitleDisplayMode) -> some View { self }

    func statusBarHidden(_ hidden: Bool = true) -> some View { self }

    // A Mac window has no full-screen cover; a sheet over the window, big enough for a scrolling page, is the nearest.
    func fullScreenCover<Content: View>(isPresented: Binding<Bool>, onDismiss: (() -> Void)? = nil, @ViewBuilder content: @escaping () -> Content) -> some View {
        sheet(isPresented: isPresented, onDismiss: onDismiss) {
            content().frame(minWidth: COVER_MIN_WIDTH, minHeight: COVER_MIN_HEIGHT)
        }
    }

    func fullScreenCover<Item: Identifiable, Content: View>(item: Binding<Item?>, onDismiss: (() -> Void)? = nil, @ViewBuilder content: @escaping (Item) -> Content) -> some View {
        sheet(item: item, onDismiss: onDismiss) { shown in
            content(shown).frame(minWidth: COVER_MIN_WIDTH, minHeight: COVER_MIN_HEIGHT)
        }
    }
}

extension ToolbarPlacement {
    // iOS hides the navigation bar; on a Mac the nearest bar is whatever toolbar the container shows.
    static var navigationBar: ToolbarPlacement { .automatic }
}
#endif
