import Foundation
import Observation

/// A construction standard saved under a name ("My shop"), to start jobs from or apply to one.
struct NamedStandard: Codable, Hashable, Identifiable {
    var name: String
    /// The core's `ConstructionStandard` as JSON.
    var standardJson: String
    var id: String { name }

    /// The choice of one picker field (`back`, `top_joint`...).
    func choice(_ field: String) -> String {
        let o = (try? JSONSerialization.jsonObject(with: Data(standardJson.utf8))) as? [String: Any]
        return o?[field] as? String ?? ""
    }
}

/// The saved standards, one JSON file each in Application Support/CabinetStudio/Standards, shared
/// by every job window.
@MainActor
@Observable
final class StandardLibrary {
    static let shared = StandardLibrary()

    private(set) var standards: [NamedStandard] = []

    /// The `@AppStorage` key of the standard new jobs start from (empty: the core's default).
    static let defaultKey = "defaultStandard"

    private let dir: URL = {
        let d = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("CabinetStudio/Standards", isDirectory: true)
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d
    }()

    init() {
        reload()
    }

    func reload() {
        let files = (try? FileManager.default.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil)) ?? []
        standards = files.filter { $0.pathExtension == "json" }
            .compactMap { try? JSONDecoder().decode(NamedStandard.self, from: Data(contentsOf: $0)) }
            .sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
    }

    func standard(named name: String) -> NamedStandard? {
        standards.first { $0.name == name }
    }

    /// Saves `standardJson` as `name`, replacing a standard of that name.
    func save(_ name: String, standardJson: String) throws {
        let s = NamedStandard(name: name, standardJson: standardJson)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        try encoder.encode(s).write(to: file(for: name), options: .atomic)
        reload()
    }

    func delete(_ name: String) {
        try? FileManager.default.removeItem(at: file(for: name))
        reload()
    }

    private func file(for name: String) -> URL {
        let safe = name.map { "/:\\".contains($0) ? "-" : $0 }
        return dir.appendingPathComponent(String(safe)).appendingPathExtension("json")
    }
}
