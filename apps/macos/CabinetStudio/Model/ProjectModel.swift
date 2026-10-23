import AppKit
import Foundation
import Observation
import SwiftUI

/// One cabinet in the project. `json` is the Rust core's cabinet JSON, the single source of truth;
/// the computed properties read and write it.
struct CabinetDoc: Identifiable, Hashable, Codable {
    var id: String
    var json: String
    /// Made from a box drawn in SolidCraft: it keeps the box's position instead of joining the run.
    var placed: Bool?

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
        set {
            let n = max(0, newValue)
            setNumber("adjustable_shelves", Double(n))
            fixedShelves = fixedShelves.filter { $0 <= n }
        }
    }

    /// Shelves (numbered from 1 at the bottom) fixed between the sides instead of on pins.
    var fixedShelves: [Int] {
        get { (object["fixed_shelves"] as? [NSNumber])?.map(\.intValue) ?? [] }
        set { update { $0["fixed_shelves"] = newValue.isEmpty ? nil : newValue.sorted() } }
    }

    private var blind: [String: Any] { object["blind"] as? [String: Any] ?? [:] }

    private mutating func setBlind(_ key: String, _ value: Any) {
        update { o in
            var b = o["blind"] as? [String: Any] ?? [:]
            b[key] = value
            o["blind"] = b
        }
    }

    /// Blind corner: the side the other run meets it on. The door hinges on the other side.
    var blindSide: String {
        get { blind["side"] as? String ?? "left" }
        set {
            setBlind("side", newValue)
            hingeSide = newValue == "left" ? "right" : "left"
        }
    }

    /// Blind corner: front width covered by the blind panel and filler.
    var blindWidth: Double {
        get { (blind["width"] as? NSNumber)?.doubleValue ?? 630 }
        set { setBlind("width", newValue) }
    }

    /// Blind corner: the filler between the blind panel and the door.
    var blindFiller: Double {
        get { (blind["filler"] as? NSNumber)?.doubleValue ?? 50 }
        set { setBlind("filler", newValue) }
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

    /// Lays the cabinets out side by side in list order and saves. Cabinets drawn in SolidCraft
    /// stay where they were drawn.
    func changed() {
        var x = 0.0
        for i in cabinets.indices where cabinets[i].placed != true {
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

    /// Where SolidCraft's installer puts the app.
    static let solidCraftApp = URL(fileURLWithPath: "/Applications/SolidCraft.app")

    /// Starts SolidCraft with its control port open and waits until it answers.
    func openSolidCraft() async {
        let app = Self.solidCraftApp
        lastError = nil
        guard FileManager.default.fileExists(atPath: app.path) else {
            lastError = "SolidCraft is not in Applications. Get it from github.com/tangency-labs/solidcraft/releases."
            return
        }
        // A running SolidCraft keeps the arguments it was started with.
        if NSWorkspace.shared.runningApplications.contains(where: {
            $0.bundleURL?.standardizedFileURL == app.standardizedFileURL
        }) {
            lastError = "SolidCraft is open without its control port. Quit it, then choose Open SolidCraft again."
            return
        }
        let config = NSWorkspace.OpenConfiguration()
        config.arguments = ["--control", "\(port)"]
        do {
            try await NSWorkspace.shared.openApplication(at: app, configuration: config)
        } catch {
            lastError = "Could not open SolidCraft: \(error.localizedDescription)"
            return
        }
        status = "Waiting for SolidCraft…"
        connection = .unknown
        for _ in 0..<30 {
            try? await Task.sleep(for: .seconds(1))
            await checkConnection()
            if connection == .connected {
                status = "SolidCraft is connected"
                return
            }
        }
        lastError = "SolidCraft opened but does not answer on port \(port)"
    }

    /// Turns the boxes selected in SolidCraft into cabinets in place, builds them and hides the
    /// boxes.
    func makeCabinetsFromSelection() async {
        isSending = true
        lastError = nil
        status = "Reading the selection in SolidCraft…"
        let port = self.port
        let taken = cabinets.map(\.id)
        let result = await Task.detached { () -> Result<BoxImport, Error> in
            Result { try cabinetsFromSelection(port: port, takenIds: taken) }
        }.value
        isSending = false
        let made: BoxImport
        switch result {
        case .success(let r): made = r
        case .failure(let e):
            lastError = "\(e)"
            status = "Could not read the selection in SolidCraft"
            await checkConnection()
            return
        }
        guard !made.cabinets.isEmpty else {
            lastError = made.skipped.first.map { "Not a box: \($0)" } ?? "Select one or more boxes in SolidCraft first."
            return
        }
        withAnimation(.snappy) {
            for c in made.cabinets {
                cabinets.append(CabinetDoc(id: c.id, json: c.json, placed: true))
            }
            selection = made.cabinets.last?.id
        }
        changed()
        await sendToSolidCraft()
        guard lastError == nil else { return }
        let sources = made.cabinets.map(\.source)
        _ = await Task.detached { try? hideBodies(port: port, bodies: sources) }.value
        var note = "Made \(made.cabinets.count) cabinets from boxes"
        if !made.skipped.isEmpty { note += " · skipped \(made.skipped.joined(separator: ", "))" }
        status = note
    }

    /// Loads the cabinets built into the design open in SolidCraft. A cabinet already in the
    /// project (same id) is replaced by the design's version; they keep their design positions.
    func loadCabinetsFromDesign() async {
        isSending = true
        lastError = nil
        status = "Reading cabinets from the SolidCraft design…"
        let port = self.port
        let result = await Task.detached { () -> Result<[String], Error> in
            Result { try cabinetsInDesign(port: port) }
        }.value
        isSending = false
        switch result {
        case .success(let jsons):
            guard !jsons.isEmpty else {
                status = "The SolidCraft design has no cabinets"
                return
            }
            withAnimation(.snappy) {
                for json in jsons {
                    guard let data = json.data(using: .utf8),
                          let id = (try? JSONSerialization.jsonObject(with: data) as? [String: Any])?["id"] as? String
                    else { continue }
                    let doc = CabinetDoc(id: id, json: json, placed: true)
                    if let i = cabinets.firstIndex(where: { $0.id == id }) {
                        cabinets[i] = doc
                    } else {
                        cabinets.append(doc)
                    }
                }
                selection = selection ?? cabinets.first?.id
            }
            changed()
            status = "Loaded \(jsons.count) cabinets from SolidCraft"
            connection = .connected
        case .failure(let e):
            lastError = "\(e)"
            status = "Could not read the SolidCraft design"
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
        case "blind_corner": "Blind corner"
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
        case "blind_corner": "rectangle.lefthalf.inset.filled"
        default: "cube"
        }
    }
}
