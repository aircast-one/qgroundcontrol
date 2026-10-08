import Foundation

enum UploadOutcome { case Sent, NotStarted }

func uploadOutcome(_ invoked: Bool) -> UploadOutcome { invoked ? .Sent : .NotStarted }

func uploadMessage(_ outcome: UploadOutcome) -> String {
    switch outcome {
    case .Sent: "Upload sent to vehicle"
    case .NotStarted: "Upload did not start"
    }
}

func downloadMessage(_ outcome: UploadOutcome) -> String {
    switch outcome {
    case .Sent: "Download requested from vehicle"
    case .NotStarted: "Download did not start"
    }
}
