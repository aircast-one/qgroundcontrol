import XCTest
import os
@testable import Aircast

private final class Queued: Sendable {
    private let count = OSAllocatedUnfairLock(initialState: 0)

    var size: Int { count.withLock { $0 } }

    func add(_ work: @escaping @Sendable () -> Void) { count.withLock { $0 += 1 } }
}

final class RcHolderTests: XCTestCase {
    func testAControlReleasesEveryChannelItSentNotTheOnesItIsSetToNow() {
        let queued = Queued()
        let holder = RcHolder(send: queued.add)
        holder.hold(7, 1600)
        holder.hold(8, 1600)
        XCTAssertEqual(holder.holding(), [7, 8], "tilt moved from channel 7 to 8 mid-flight; both were sent, so both are held")
        holder.release()
        XCTAssertEqual(holder.holding(), [])
        XCTAssertEqual(queued.size, 3, "two sends and one release, in that order on one queue")
    }

    func testAnUnassignedChannelIsNeverSentOrHeld() {
        let queued = Queued()
        let holder = RcHolder(send: queued.add)
        holder.hold(0, 1600)
        holder.release()
        XCTAssertEqual(holder.holding(), [])
        XCTAssertEqual(queued.size, 0)
    }

    func testAControlRebuiltByARotationReadsBackTheValueItsChannelIsHeldAt() {
        let holder = RcHolder(send: Queued().add)
        holder.hold(5, 1300)
        holder.hold(5, 1700)
        XCTAssertEqual(holder.held(5), 1700)
        XCTAssertNil(holder.held(6))
        holder.release()
        XCTAssertNil(holder.held(5), "leaving the Fly screen gives the channel back, so the control starts fresh")
    }

    func testGivingEverythingBackForgetsWhatWasHeldWithoutASecondRelease() {
        let queued = Queued()
        let holder = RcHolder(send: queued.add)
        holder.hold(9, 2000)
        holder.forget()
        holder.release()
        XCTAssertEqual(queued.size, 1)
    }
}
