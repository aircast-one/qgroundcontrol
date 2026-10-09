import SwiftUI

private let SECONDS_PER_MINUTE: Int64 = 60
private let TICK_SECONDS: TimeInterval = 1

struct SilenceSchedule: TimelineSchedule {
    let ticking: Bool

    func entries(from startDate: Date, mode: TimelineScheduleMode) -> AnySequence<Date> {
        ticking ? AnySequence(sequence(first: startDate) { $0.addingTimeInterval(TICK_SECONDS) }) : AnySequence([startDate])
    }
}

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
            TimelineView(SilenceSchedule(ticking: since != nil)) { context in
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
