import Foundation

@main struct OffcutStoreTests {
    @MainActor static func main() throws {
        let dir = FileManager.default.temporaryDirectory.appending(path: UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: dir) }
        let url = dir.appending(path: "offcuts.json")
        let store = OffcutStore(url: url)
        let firstLayout = store.json
        let leftover = #"[{"board":"Oak","length":600,"width":300,"thickness":16,"grained":true}]"#
        try store.keep(leftover, using: firstLayout)
        do {
            try store.keep("[]", using: firstLayout)
            preconditionFailure("A stale layout consumed stock from another cut")
        } catch {
            precondition(store.json == leftover)
        }
        try store.keep(leftover, using: nil)
        precondition(store.count == 2)
        let reopened = OffcutStore(url: url)
        precondition(reopened.count == 2)
        try store.clear()
        do {
            try reopened.keep(leftover, using: reopened.json)
            preconditionFailure("A second store restored cleared stock")
        } catch {
            precondition(reopened.json.isEmpty && OffcutStore(url: url).count == 0)
        }
        print("Shared offcut inventory checks passed")
    }
}
