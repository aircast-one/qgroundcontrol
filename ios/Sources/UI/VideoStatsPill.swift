import SwiftUI

let VIDEO_STATS_VIEW = "view.videoStats"
private let VIDEO_STATS_POLL_MS = 1000

func videoStatsText(_ view: JSON?) -> String { view?["text"].string ?? "" }

struct VideoStatsPill: View {
    @FlyIsPortrait private var portrait
    @State private var text = ""

    var body: some View {
        ZStack(alignment: .bottomLeading) {
            Color.clear
            if !text.isBlank && portrait {
                Text(text)
                    .font(.labelSmall)
                    .bold()
                    .foregroundStyle(.white)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 4)
                    .background(Color.black.opacity(0.6), in: Capsule())
                    .padding(8)
            }
        }
        .allowsHitTesting(false)
        .task {
            while !Task.isCancelled {
                text = await offMain { videoStatsText(Qgc.get(VIDEO_STATS_VIEW)) }
                try? await Task.sleep(for: .milliseconds(VIDEO_STATS_POLL_MS))
            }
        }
    }
}
