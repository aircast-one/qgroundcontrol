import SwiftUI

private let SECONDS_PER_MINUTE: Int64 = 60

struct SilentSeconds<Content: View>: View {
    let lost: Bool
    @ViewBuilder let content: (Int64?) -> Content
    @State private var since: Date?

    init(lost: Bool, @ViewBuilder content: @escaping (Int64?) -> Content) {
        self.lost = lost
        self.content = content
        _since = State(initialValue: lost ? Date() : nil)
    }

    var body: some View {
        ZStack(alignment: .topLeading) {
            TimelineView(.periodic(from: .now, by: since == nil ? 3600 : 1)) { context in
                content(since.map { Int64(max(0, context.date.timeIntervalSince($0))) })
            }
        }
        .onChange(of: lost) { _, now in since = now ? Date() : nil }
    }
}

func silenceDuration(_ seconds: Int64) -> String {
    seconds < SECONDS_PER_MINUTE ? "\(seconds) s" : "\(seconds / SECONDS_PER_MINUTE) min"
}

func lastSeenText(_ seconds: Int64) -> String { "Last seen \(silenceDuration(seconds)) ago" }
