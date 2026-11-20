import Foundation

enum UploadOutcome { case Sent, NotStarted }

func uploadOutcome(_ invoked: Bool) -> UploadOutcome { invoked ? .Sent : .NotStarted }

func uploadMessage(_ outcome: UploadOutcome) -> String {
    switch outcome {
    case .Sent: "Upload sent to vehicle"
    case .NotStarted: "Upload did not start"
    }
}

func pauseRefusedText(_ reason: String) -> String { "The aircraft did not pause, so the plan was not uploaded. \(reason)" }

func refusalIn(_ answer: JSON?, _ fallback: String) -> String? {
    answer?["ok"].bool == true ? nil : answer?["reason"].string.nonBlank ?? fallback
}

func downloadMessage(_ outcome: UploadOutcome) -> String {
    switch outcome {
    case .Sent: "Download requested from vehicle"
    case .NotStarted: "Download did not start"
    }
}
