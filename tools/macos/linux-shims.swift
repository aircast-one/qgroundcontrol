// Swift on Linux has no Combine, and one file in the pure-logic set (PageSelection.swift) is an
// ObservableObject with @Published properties. Nothing in these checks subscribes to a change, so
// the two names only have to exist: swift-checks.sh compiles this file when it is not on macOS,
// which lets the suite run on a Linux Swift toolchain. On macOS Combine is there and this is empty.
#if !canImport(Combine)
protocol ObservableObject: AnyObject {}

struct ObservableObjectPublisher {
    func send() {}
}

extension ObservableObject {
    var objectWillChange: ObservableObjectPublisher { ObservableObjectPublisher() }
}

@propertyWrapper
struct Published<Value> {
    var wrappedValue: Value

    init(wrappedValue: Value) {
        self.wrappedValue = wrappedValue
    }
}
#endif
