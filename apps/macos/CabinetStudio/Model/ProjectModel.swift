import Foundation
import Observation
import SwiftUI

/// One cabinet in the project. `json` is the Rust core's cabinet JSON, the single source of truth;
/// the computed properties read and write it.
struct CabinetDoc: Identifiable, Hashable, Codable {
    var id: String
    var json: String

    private var object: [String: Any] {
        (try? JSONSerialization.jsonObject(with: Data(json.utf8))) as? [String: Any] ?? [:]
    }

    private mutating func update(_ change: (inout [String: Any]) -> Void) {
        var o = object
        change(&o)
        if let data = try? JSONSerialization.data(withJSONObject: o, options: [.sortedKeys]),
           let s = String(data: data, encoding: .utf8) {
            json = s
        }
    }

    var kind: String { object["kind"] as? String ?? "base" }

    func number(_ key: String) -> Double { (object[key] as? NSNumber)?.doubleValue ?? 0 }

    mutating func setNumber(_ key: String, _ value: Double) { update { $0[key] = value } }

    var width: Double {
        get { number("width") }
        set { setNumber("width", newValue) }
    }
    var depth: Double {
        get { number("depth") }
        set { setNumber("depth", newValue) }
    }
    var height: Double {
        get { number("height") }
        set { setNumber("height", newValue) }
    }
    var shelves: Int {
        get { Int(number("adjustable_shelves")) }
        set { setNumber("adjustable_shelves", Double(max(0, newValue))) }
    }

    /// X offset of the cabinet in the run.
    var originX: Double {
        get { ((object["origin"] as? [String: Any])?["x"] as? NSNumber)?.doubleValue ?? 0 }
        set { update { $0["origin"] = ["x": newValue, "y": 0.0, "z": 0.0] } }
    }

    /// The front layout's type: open, door, door_pair, drawers.
    var frontType: String {
        get { (object["fronts"] as? [String: Any])?["type"] as? String ?? "open" }
        set {
            update { o in
                switch newValue {
                case "door": o["fronts"] = ["type": "door", "hinge_side": "left"]
                case "door_pair": o["fronts"] = ["type": "door_pair"]
                case "drawers": o["fronts"] = ["type": "drawers", "ratios": [1.0, 1.0, 1.0]]
                default: o["fronts"] = ["type": "open"]
                }
            }
        }
    }

    var drawerRatios: [Double] {
        get { ((object["fronts"] as? [String: Any])?["ratios"] as? [NSNumber])?.map(\.doubleValue) ?? [] }
        set { update { $0["fronts"] = ["type": "drawers", "ratios": newValue] } }
    }

    var hingeSide: String {
        get { (object["fronts"] as? [String: Any])?["hinge_side"] as? String ?? "left" }
        set { update { $0["fronts"] = ["type": "door", "hinge_side": newValue] } }
    }
}

enum ConnectionState: Equatable {
    case unknown, connected, offline
}

enum DetailLevel: String, CaseIterable, Identifiable {
    case design = "Design"
    case manufacturing = "Manufacturing"
    var id: String { rawValue }
}

enum Workspace: String, CaseIterable, Identifiable {
    case design = "Design"
    case cutList = "Cut List"
    case sheets = "Cut Sheets"
    case boards = "Boards"
    var id: String { rawValue }

    var symbol: String {
        switch self {
        case .design: "cube.transparent"
        case .cutList: "list.bullet.rectangle"
        case .sheets: "doc.richtext"
        case .boards: "square.stack.3d.up"
        }
    }
}

@MainActor
@Observable
final class ProjectModel {
    var cabinets: [CabinetDoc] = []
    var selection: CabinetDoc.ID?
    var workspace: Workspace = .design
    var detail: DetailLevel = .design
    var connection: ConnectionState = .unknown
    var status: String = ""
    var isSending = false
    var lastError: String?
    var jobName = "Untitled job"
    let port: UInt16 = 7878

    private var builds: [String: BuildSummary] = [:]

    init() {
        load()
        if cabinets.isEmpty {
            add(kind: "base")
            add(kind: "drawer_bank")
            add(kind: "base")
            selection = cabinets.first?.id
        }
    }

    var selected: CabinetDoc? {
        get { cabinets.first { $0.id == selection } }
        set {
            guard let newValue, let i = cabinets.firstIndex(where: { $0.id == newValue.id }) else { return }
            cabinets[i] = newValue
            changed()
        }
    }

    func build(_ cab: CabinetDoc) -> BuildSummary? {
        if let b = builds[cab.json] { return b }
        do {
            let b = try CabinetStudio.build(cabinetJson: cab.json)
            builds[cab.json] = b
            return b
        } catch {
            return nil
        }
    }

    func error(for cab: CabinetDoc) -> String? {
        do {
            _ = try CabinetStudio.build(cabinetJson: cab.json)
            return nil
        } catch {
            return "\(error)"
        }
    }

    // MARK: Editing

    func add(kind: String) {
        let prefix = kind.prefix(1).uppercased()
        var n = 1
        while cabinets.contains(where: { $0.id == String(format: "%@%02d", prefix, n) }) { n += 1 }
        let id = String(format: "%@%02d", prefix, n)
        guard let json = try? newCabinet(id: id, kind: kind) else { return }
        cabinets.append(CabinetDoc(id: id, json: json))
        selection = id
        changed()
    }

    func remove(_ id: CabinetDoc.ID) {
        cabinets.removeAll { $0.id == id }
        if selection == id { selection = cabinets.first?.id }
        changed()
    }

    func move(from: IndexSet, to: Int) {
        cabinets.move(fromOffsets: from, toOffset: to)
        changed()
    }

    func option(_ field: String, of cab: CabinetDoc) -> String {
        (try? standardOption(cabinetJson: cab.json, field: field)) ?? ""
    }

    func setOption(_ field: String, _ choice: String) {
        guard var cab = selected,
              let json = try? setStandardOption(cabinetJson: cab.json, field: field, choice: choice) else { return }
        cab.json = json
        selected = cab
    }

    /// Lays the cabinets out side by side in list order and saves.
    func changed() {
        var x = 0.0
        for i in cabinets.indices {
            cabinets[i].originX = x
            x += cabinets[i].width
        }
        save()
    }

    // MARK: SolidCraft

    func checkConnection() async {
        let port = self.port
        let ok = await Task.detached { solidcraftReachable(port: port) }.value
        connection = ok ? .connected : .offline
    }

    func sendToSolidCraft() async {
        isSending = true
        lastError = nil
        status = "Building \(cabinets.count) cabinets in SolidCraft…"
        let jsons = cabinets.map(\.json)
        let mfg = detail == .manufacturing
        let port = self.port
        let result = await Task.detached { () -> Result<String, Error> in
            Result { try sendToSolidcraft(cabinetsJson: jsons, manufacturing: mfg, port: port) }
        }.value
        isSending = false
        switch result {
        case .success(let report):
            status = report.replacingOccurrences(of: "\n", with: " · ")
            connection = .connected
        case .failure(let e):
            lastError = "\(e)"
            status = "SolidCraft did not build the cabinets"
            await checkConnection()
        }
    }

    // MARK: Persistence

    private var saveURL: URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("CabinetStudio", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("project.json")
    }

    private struct Saved: Codable {
        var jobName: String
        var cabinets: [CabinetDoc]
    }

    func save() {
        let data = try? JSONEncoder().encode(Saved(jobName: jobName, cabinets: cabinets))
        try? data?.write(to: saveURL, options: .atomic)
    }

    private func load() {
        guard let data = try? Data(contentsOf: saveURL),
              let saved = try? JSONDecoder().decode(Saved.self, from: data) else { return }
        jobName = saved.jobName
        cabinets = saved.cabinets.filter { (try? CabinetStudio.build(cabinetJson: $0.json)) != nil }
        selection = cabinets.first?.id
    }

    /// The bundled board catalog.
    @ObservationIgnored private(set) lazy var boards: [BoardRecord] = {
        guard let dir = Bundle.main.url(forResource: "catalog", withExtension: nil) else { return [] }
        return (try? loadBoards(catalogDir: dir.path)) ?? []
    }()
}

extension CabinetDoc {
    static func kindTitle(_ kind: String) -> String {
        switch kind {
        case "base": "Base"
        case "wall": "Wall"
        case "tall": "Tall"
        case "drawer_bank": "Drawer bank"
        case "open_shelf": "Open shelf"
        default: kind.capitalized
        }
    }

    static func kindSymbol(_ kind: String) -> String {
        switch kind {
        case "base": "square.split.1x2"
        case "wall": "rectangle.portrait.topthird.inset.filled"
        case "tall": "rectangle.portrait"
        case "drawer_bank": "square.split.1x2.fill"
        case "open_shelf": "rectangle.split.3x1"
        default: "cube"
        }
    }
}
