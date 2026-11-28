import Combine
import Foundation

/// All job windows share the inventory. A cut may consume stock only from the layout it used.
@MainActor final class OffcutStore: ObservableObject {
    static let shared = OffcutStore(url: FileManager.default.urls(for: .applicationSupportDirectory,
                                                                   in: .userDomainMask)[0]
        .appending(path: "CabinetStudio/offcuts.json"))

    private let url: URL
    @Published private(set) var json: String

    init(url: URL) {
        self.url = url
        json = (try? Self.read(url)) ?? ""
    }

    private static func read(_ url: URL) throws -> String {
        do { return try String(contentsOf: url, encoding: .utf8) }
        catch let error as CocoaError where error.code == .fileReadNoSuchFile { return "" }
    }

    private static func pieces(_ json: String) throws -> [[String: Any]] {
        if json.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { return [] }
        let value = try JSONSerialization.jsonObject(with: Data(json.utf8))
        guard let pieces = value as? [[String: Any]] else { throw CocoaError(.fileReadCorruptFile) }
        return pieces
    }

    var count: Int { (try? Self.pieces(json).count) ?? 0 }

    /// Without a stock snapshot this cut used new sheets only, so its leftovers are appended.
    func keep(_ leftovers: String, using snapshot: String?) throws {
        let current = try Self.read(url)
        if let snapshot, snapshot != current {
            json = current
            throw NSError(domain: "CabinetStudio.Offcuts", code: 1, userInfo: [
                NSLocalizedDescriptionKey: "Kept offcuts changed. Review the updated layout before saving."
            ])
        }
        let kept: String
        if snapshot == nil {
            let pieces = try Self.pieces(current) + Self.pieces(leftovers)
            kept = String(decoding: try JSONSerialization.data(withJSONObject: pieces), as: UTF8.self)
        } else {
            kept = leftovers
        }
        try write(kept)
    }

    func clear() throws { try write("") }

    private func write(_ value: String) throws {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try value.write(to: url, atomically: true, encoding: .utf8)
        json = value
    }
}
