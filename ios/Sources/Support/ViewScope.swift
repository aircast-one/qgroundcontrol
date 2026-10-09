import Foundation

@MainActor
final class ViewScope {
    private var tasks: [UUID: Task<Void, Never>] = [:]

    nonisolated init() {}

    func launch(_ work: @escaping @MainActor () async -> Void) {
        let id = UUID()
        tasks[id] = Task { [weak self] in
            await work()
            self?.tasks[id] = nil
        }
    }

    func cancel() {
        tasks.values.forEach { $0.cancel() }
        tasks = [:]
    }
}
