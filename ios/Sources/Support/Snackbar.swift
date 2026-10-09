import SwiftUI

let SNACKBAR_MAX_WIDTH: CGFloat = 600

enum SnackbarDuration: Equatable {
    case Short, Long, Indefinite

    var millis: Int? {
        switch self {
        case .Short: 4000
        case .Long: 10000
        case .Indefinite: nil
        }
    }
}

enum SnackbarResult: Equatable {
    case Dismissed, ActionPerformed
}

@MainActor
final class SnackbarData: Identifiable {
    let id = UUID()
    let message: String
    let actionLabel: String?
    let withDismissAction: Bool
    let duration: SnackbarDuration
    fileprivate var finish: ((SnackbarResult) -> Void)?

    fileprivate init(message: String, actionLabel: String?, withDismissAction: Bool, duration: SnackbarDuration) {
        self.message = message
        self.actionLabel = actionLabel
        self.withDismissAction = withDismissAction
        self.duration = duration
    }

    func dismiss() { resolve(.Dismissed) }

    func performAction() { resolve(.ActionPerformed) }

    private func resolve(_ result: SnackbarResult) {
        let done = finish
        finish = nil
        done?(result)
    }
}

@MainActor
@Observable
final class SnackbarHostState {
    private(set) var currentSnackbarData: SnackbarData?
    @ObservationIgnored private var waiters: [(id: UUID, turn: CheckedContinuation<Bool, Never>)] = []
    @ObservationIgnored private var busy = false

    @discardableResult
    func showSnackbar(_ message: String, actionLabel: String? = nil, withDismissAction: Bool = false, duration: SnackbarDuration? = nil) async -> SnackbarResult {
        guard await acquire() else { return .Dismissed }
        let data = SnackbarData(
            message: message,
            actionLabel: actionLabel,
            withDismissAction: withDismissAction,
            duration: duration ?? (actionLabel == nil ? .Short : .Indefinite)
        )
        currentSnackbarData = data
        let timer = data.duration.millis.map { millis in
            Task {
                guard (try? await Task.sleep(for: .milliseconds(millis))) != nil else { return }
                data.dismiss()
            }
        }
        let result = await withTaskCancellationHandler {
            await withCheckedContinuation { done in data.finish = { done.resume(returning: $0) } }
        } onCancel: {
            Task { @MainActor in data.dismiss() }
        }
        timer?.cancel()
        currentSnackbarData = nil
        release()
        return result
    }

    private func acquire() async -> Bool {
        guard busy else {
            busy = true
            return true
        }
        let id = UUID()
        return await withTaskCancellationHandler {
            await withCheckedContinuation { waiters.append((id, $0)) }
        } onCancel: {
            Task { @MainActor in self.abandon(id) }
        }
    }

    private func abandon(_ id: UUID) {
        guard let at = waiters.firstIndex(where: { $0.id == id }) else { return }
        waiters.remove(at: at).turn.resume(returning: false)
    }

    private func release() {
        guard !waiters.isEmpty else {
            busy = false
            return
        }
        waiters.removeFirst().turn.resume(returning: true)
    }
}

struct SnackbarHost<Content: View>: View {
    let hostState: SnackbarHostState
    @ViewBuilder let snackbar: (SnackbarData) -> Content

    var body: some View {
        ZStack {
            if let data = hostState.currentSnackbarData {
                snackbar(data)
                    .id(data.id)
                    .transition(.move(edge: .bottom).combined(with: .opacity))
            }
        }
        .animation(.easeOut(duration: 0.2), value: hostState.currentSnackbarData?.id)
    }
}

struct Snackbar: View {
    let data: SnackbarData
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: Space.s2) {
            Text(data.message)
                .font(.bodyMedium)
                .foregroundStyle(theme.colors.inverseOnSurface)
                .frame(maxWidth: .infinity, alignment: .leading)
            if let action = data.actionLabel {
                Button(action) { data.performAction() }
                    .font(.labelLarge)
                    .foregroundStyle(theme.dark ? Theme.lightTheme.colors.primary : Theme.darkTheme.colors.primary)
                    .buttonStyle(.text)
            }
            if data.withDismissAction {
                Button { data.dismiss() } label: { Image(.close) }
                    .foregroundStyle(theme.colors.inverseOnSurface)
                    .buttonStyle(.borderless)
                    .accessibilityLabel("Dismiss")
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 14)
        .background(theme.colors.inverseSurface, in: RoundedRectangle(cornerRadius: Corner.extraSmall))
        .frame(maxWidth: SNACKBAR_MAX_WIDTH)
        .padding(12)
    }
}
