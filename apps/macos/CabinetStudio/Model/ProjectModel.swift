import AppKit
import Foundation
import Observation
import SwiftUI
import UniformTypeIdentifiers

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

    /// The cabinet kind, or `filler` / `end_panel` for those run pieces.
    var kind: String { object["kind"] as? String ?? object["type"] as? String ?? "base" }

    /// A filler or end panel rather than a cabinet.
    var isPiece: Bool { object["type"] != nil }

    /// Filler and end panel: extra to cut to the wall on site.
    var scribe: Double {
        get { number("scribe") }
        set { setNumber("scribe", newValue) }
    }

    /// End panel: down to the floor over the toe kick.
    var toFloor: Bool {
        get { (object["to_floor"] as? Bool) ?? true }
        set { update { $0["to_floor"] = newValue } }
    }

    /// A new filler, end panel, benchtop, shelf or moulding as run item JSON.
    static func piece(_ type: String, id: String) -> CabinetDoc {
        let json = switch type {
        case "filler": #"{"id":"\#(id)","scribe":10,"type":"filler","width":50}"#
        case "benchtop": #"{"id":"\#(id)","type":"benchtop","thickness":33,"front_overhang":20,"side_overhang":0}"#
        case "floating_shelf": #"{"id":"\#(id)","type":"floating_shelf","width":900,"depth":250,"thickness":40,"height":1500}"#
        case "cornice", "light_rail": #"{"id":"\#(id)","type":"moulding","kind":"\#(type)","height":60,"returns":true}"#
        case "gap": #"{"id":"\#(id)","type":"gap","width":600,"line":"upper"}"#
        case "splashback": ##"{"id":"\##(id)","type":"splashback","tile_width":300,"tile_height":75,"grout":2,"pattern":"stack","colour":"#F2EFE8","grout_colour":"#B9B4AA","thickness":10}"##
        case "walls": #"{"id":"\#(id)","type":"walls","left":true,"right":false,"length":3600,"height":2400,"left_run":600,"right_run":600,"offset":0}"#
        default: #"{"id":"\#(id)","scribe":20,"to_floor":true,"type":"end_panel"}"#
        }
        return CabinetDoc(id: id, json: json)
    }

    /// A floating shelf's board, and its light's profile.
    var shelfBoard: String {
        get { object["board"] as? String ?? "" }
        set { update { $0["board"] = newValue } }
    }
    var shelfProfile: String {
        get { object["light_profile"] as? String ?? "" }
        set { update { $0["light_profile"] = newValue } }
    }

    // A splashback's finish. A tile width of 0 is one sheet with no joints.
    var tileWidth: Double {
        get { number("tile_width") }
        set { setNumber("tile_width", newValue) }
    }
    var tileHeight: Double {
        get { number("tile_height") }
        set { setNumber("tile_height", newValue) }
    }
    var grout: Double {
        get { number("grout") }
        set { setNumber("grout", newValue) }
    }
    var tilePattern: String {
        get { object["pattern"] as? String ?? "stack" }
        set { update { $0["pattern"] = newValue } }
    }
    var tileColour: String {
        get { object["colour"] as? String ?? "#F2EFE8" }
        set { update { $0["colour"] = newValue } }
    }
    var groutColour: String {
        get { object["grout_colour"] as? String ?? "#B9B4AA" }
        set { update { $0["grout_colour"] = newValue } }
    }

    /// Return walls: at the run's left and right ends, and the gap between them (0: not measured).
    var wallLeft: Bool {
        get { (object["left"] as? Bool) ?? false }
        set { update { $0["left"] = newValue } }
    }
    var wallRight: Bool {
        get { (object["right"] as? Bool) ?? false }
        set { update { $0["right"] = newValue } }
    }
    var wallLength: Double {
        get { number("length") }
        set { setNumber("length", newValue) }
    }
    var wallHeight: Double {
        get { (object["height"] as? NSNumber)?.doubleValue ?? 2400 }
        set { setNumber("height", newValue) }
    }
    var wallLeftRun: Double {
        get { (object["left_run"] as? NSNumber)?.doubleValue ?? 600 }
        set { setNumber("left_run", newValue) }
    }
    var wallRightRun: Double {
        get { (object["right_run"] as? NSNumber)?.doubleValue ?? 600 }
        set { setNumber("right_run", newValue) }
    }
    var wallOffset: Double {
        get { number("offset") }
        set { setNumber("offset", newValue) }
    }

    /// Hung above the floor line, not standing on it.
    var isUpper: Bool {
        kind == "gap" ? gapIsUpper : ["wall", "corner_wall", "rangehood", "floating_shelf"].contains(kind)
    }

    /// Gap: among the wall cabinets, or among the floor cabinets.
    var gapIsUpper: Bool {
        get { (object["line"] as? String ?? "upper") == "upper" }
        set { update { $0["line"] = newValue ? "upper" : "floor" } }
    }

    /// A wall cabinet's height off the floor, 0 for the job's usual one.
    var hangHeight: Double {
        get { ((object["origin"] as? [String: Any])?["z"] as? NSNumber)?.doubleValue ?? 0 }
        set {
            update {
                var o = ($0["origin"] as? [String: Any]) ?? ["x": 0, "y": 0, "z": 0]
                o["z"] = newValue
                $0["origin"] = o
            }
        }
    }


    /// Benchtop: thickness and overhangs.
    var topThickness: Double {
        get { (object["thickness"] as? NSNumber)?.doubleValue ?? 33 }
        set { setNumber("thickness", newValue) }
    }
    var frontOverhang: Double {
        get { (object["front_overhang"] as? NSNumber)?.doubleValue ?? 20 }
        set { setNumber("front_overhang", newValue) }
    }
    var sideOverhang: Double {
        get { number("side_overhang") }
        set { setNumber("side_overhang", newValue) }
    }

    /// Cornice or light rail: its face's height, and whether it returns to the wall at open ends.
    var mouldingHeight: Double {
        get { (object["height"] as? NSNumber)?.doubleValue ?? 60 }
        set { setNumber("height", newValue) }
    }
    var returns: Bool {
        get { object["returns"] as? Bool ?? true }
        set { update { $0["returns"] = newValue } }
    }

    /// Floating shelf: depth, height of its underside, LED power per metre (0: none).
    var shelfDepth: Double {
        get { (object["depth"] as? NSNumber)?.doubleValue ?? 250 }
        set { setNumber("depth", newValue) }
    }
    var shelfHeight: Double {
        get { (object["height"] as? NSNumber)?.doubleValue ?? 1500 }
        set { setNumber("height", newValue) }
    }
    var shelfLight: Double {
        get { number("light_watts_per_metre") }
        set { setNumber("light_watts_per_metre", newValue) }
    }

    /// Benchtop: the edge profile (`square`, `pencil_round`, `bullnose`, `bevel`, `mitred_apron`).
    var edgeProfile: String {
        get { (object["edge"] as? [String: Any])?["type"] as? String ?? "square" }
        set {
            update { o in
                o["edge"] = newValue == "mitred_apron"
                    ? ["type": newValue, "depth": 60.0] as [String: Any]
                    : ["type": newValue]
            }
        }
    }

    /// Benchtop: seating overhang past the cabinets' backs (0: to the wall).
    var backOverhang: Double {
        get { number("back_overhang") }
        set { setNumber("back_overhang", newValue) }
    }

    /// Benchtop: waterfall ends, `left` or `right`.
    func waterfall(_ side: String) -> Bool { object["waterfall_\(side)"] as? Bool ?? false }

    mutating func setWaterfall(_ side: String, _ on: Bool) { update { $0["waterfall_\(side)"] = on } }

    /// Benchtop: the radius of the free corners at each piece's ends (0: square).
    var endRadius: Double {
        get { number("end_radius") }
        set { setNumber("end_radius", newValue) }
    }

    /// Benchtop: a slab splashback's height on the top (0: none).
    var splashback: Double {
        get { number("splashback") }
        set { setNumber("splashback", newValue) }
    }

    /// Benchtop: the kind of cut-out over cabinet `id` (`sink`, `cooktop`), if any.
    func cutout(over id: String) -> String? {
        (object["cutouts"] as? [[String: Any]])?.first { $0["over"] as? String == id }?["kind"] as? String
    }

    /// Benchtop: the name of the template the cut-out over cabinet `id` was made from, if any.
    func cutoutName(over id: String) -> String? {
        guard let c = (object["cutouts"] as? [[String: Any]])?.first(where: { $0["over"] as? String == id }) else {
            return nil
        }
        return c["name"] as? String ?? (c["kind"] as? String).map { $0 == "sink" ? "Sink" : "Cooktop" }
    }

    /// Sets the cut-out over a cabinet to appliance `template` (the core's `Appliance`), or clears
    /// it.
    mutating func setCutout(over id: String, template: [String: Any]?) {
        update { o in
            var list = (o["cutouts"] as? [[String: Any]] ?? []).filter { $0["over"] as? String != id }
            if var t = template {
                t["over"] = id
                list.append(t)
            }
            o["cutouts"] = list.isEmpty ? nil : list
        }
    }

    /// Re-cuts the cut-outs made from the template named `name` to `template`.
    mutating func recut(from name: String, to template: [String: Any]) {
        update { o in
            guard var list = o["cutouts"] as? [[String: Any]] else { return }
            for i in list.indices where list[i]["name"] as? String == name {
                var t = template
                t["over"] = list[i]["over"]
                list[i] = t
            }
            o["cutouts"] = list
        }
    }

    /// Benchtop: whether the splashback has an opening of `field` (`niches`, `power_points`)
    /// over cabinet `id`.
    func hasOpening(_ field: String, over id: String) -> Bool {
        (object[field] as? [[String: Any]])?.contains { $0["over"] as? String == id } ?? false
    }

    /// Adds or removes a splashback opening over a cabinet, at the core's default sizes (a niche
    /// with one shelf; a double power point).
    mutating func setOpening(_ field: String, over id: String, _ on: Bool) {
        let plate = powerPointPlate
        update { o in
            var list = (o[field] as? [[String: Any]] ?? []).filter { $0["over"] as? String != id }
            if on {
                var new: [String: Any] = ["over": id]
                if field == "niches" { new["shelves"] = 1 } else if let plate { new["plate"] = plate }
                list.append(new)
            }
            o[field] = list.isEmpty ? nil : list
        }
    }

    /// Benchtop: its material family (`natural-stone`, `laminate`...).
    var material: String {
        get { object["material"] as? String ?? "laminate" }
        set { update { $0["material"] = newValue } }
    }

    /// Benchtop: changes one of its material's rules (nil: back to the material's).
    mutating func setMaterialRule(_ field: String, _ value: Double?) {
        update { o in
            var rules = o["material_rules"] as? [String: Any] ?? [:]
            rules[field] = value
            o["material_rules"] = rules.isEmpty ? nil : rules
        }
    }

    /// Benchtop: whether its splashback has a power point.
    var hasAnyPowerPoint: Bool { !((object["power_points"] as? [[String: Any]]) ?? []).isEmpty }

    /// Benchtop: the plate (catalog id) its power points are cut for, if it has any.
    var powerPointPlate: String? {
        (object["power_points"] as? [[String: Any]])?.first?["plate"] as? String
    }

    /// Cuts every power point in the splashback for plate `id`.
    mutating func setPowerPointPlate(_ id: String) {
        update { o in
            guard var list = o["power_points"] as? [[String: Any]] else { return }
            for i in list.indices { list[i]["plate"] = id }
            o["power_points"] = list
        }
    }

    /// The cabinet's construction standard as JSON values, field by field.
    var standard: [String: Any] {
        get { object["standard"] as? [String: Any] ?? [:] }
        set { update { $0["standard"] = newValue } }
    }

    /// The cabinet's `id` field, read from its JSON.
    static func id(of json: String) -> String? {
        ((try? JSONSerialization.jsonObject(with: Data(json.utf8))) as? [String: Any])?["id"] as? String
    }

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

    /// Doors: the arched top's rise (0: square).
    var archedTop: Double {
        get { number("arched_top") }
        set { setNumber("arched_top", newValue) }
    }

    /// A Blum CABLOXX locking bar over all the drawers.
    var lock: Bool {
        get { object["lock"] as? Bool ?? false }
        set { update { $0["lock"] = newValue ? true : nil } }
    }

    /// Blum inner dividing sets (AMBIA-LINE, ORGA-LINE) in the steel-sided drawers.
    var drawerInserts: Bool {
        get { object["drawer_inserts"] as? Bool ?? false }
        set { update { $0["drawer_inserts"] = newValue ? true : nil } }
    }

    /// Blum inner drawers behind the doors; they take the shelves' place.
    var innerDrawers: Int {
        get { Int(number("inner_drawers")) }
        set { update { $0["inner_drawers"] = newValue > 0 ? newValue : nil } }
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

    private var corner: [String: Any] { object["corner"] as? [String: Any] ?? [:] }

    private mutating func setCorner(_ key: String, _ value: Any) {
        update { o in
            var c = o["corner"] as? [String: Any] ?? [:]
            c[key] = value
            o["corner"] = c
        }
    }

    /// L-corner: the wall the second leg runs along.
    var cornerSide: String {
        get { corner["side"] as? String ?? "left" }
        set { setCorner("side", newValue) }
    }

    /// L-corner: outside length of the second leg, from the back wall.
    var cornerWidth: Double {
        get { (corner["width"] as? NSNumber)?.doubleValue ?? 900 }
        set { setCorner("width", newValue) }
    }

    /// Whether the cabinet's `left` or `right` side is seen from the room (no screws through it).
    func exposed(_ side: String) -> Bool {
        ((object["exposed"] as? [String: Any])?[side] as? Bool) ?? false
    }

    mutating func setExposed(_ side: String, _ on: Bool) {
        update { o in
            var e = o["exposed"] as? [String: Any] ?? [:]
            e[side] = on
            o["exposed"] = e
        }
    }

    /// Sink cabinet options: `back_cutout`, `liner`.
    func sinkOption(_ key: String) -> Bool {
        ((object["sink"] as? [String: Any])?[key] as? Bool) ?? true
    }

    mutating func setSinkOption(_ key: String, _ on: Bool) {
        update { o in
            var sink = o["sink"] as? [String: Any] ?? [:]
            sink[key] = on
            o["sink"] = sink
        }
    }

    private var cladding: [String: Any] { object["cladding"] as? [String: Any] ?? [:] }

    /// Whether face `left`, `right` or `back` is clad.
    func clad(_ face: String) -> Bool { cladding[face] as? Bool ?? false }

    /// A face seen from the room. Old jobs may have set a side under cladding; it reads the same.
    func seenFace(_ face: String) -> Bool {
        face == "back" ? clad("back") : (exposed(face) || clad(face))
    }

    mutating func setSeenFace(_ face: String, _ on: Bool) {
        if face == "back" {
            setClad("back", on)
        } else {
            setExposed(face, on)
            if clad(face) { setClad(face, on) }
        }
    }

    mutating func setClad(_ face: String, _ on: Bool) {
        update { o in
            var c = o["cladding"] as? [String: Any] ?? ["thickness": 12.0]
            c[face] = on
            let any = ["left", "right", "back"].contains { c[$0] as? Bool ?? false } || !(c["board"] as? String ?? "").isEmpty
            o["cladding"] = any ? c : nil
        }
    }

    /// The board on the seen faces (empty: the fronts' board).
    var cladBoard: String {
        get { cladding["board"] as? String ?? "" }
        set { update { o in var c = o["cladding"] as? [String: Any] ?? [:]; c["board"] = newValue; o["cladding"] = c } }
    }

    var cladThickness: Double {
        get { (cladding["thickness"] as? NSNumber)?.doubleValue ?? 12 }
        set { update { o in var c = o["cladding"] as? [String: Any] ?? [:]; c["thickness"] = newValue; o["cladding"] = c } }
    }

    /// A rangehood canopy's insert cut-out.
    var insertWidth: Double {
        get { ((object["insert"] as? [String: Any])?["width"] as? NSNumber)?.doubleValue ?? 520 }
        set { update { o in var i = o["insert"] as? [String: Any] ?? ["depth": 260.0]; i["width"] = newValue; o["insert"] = i } }
    }
    var insertDepth: Double {
        get { ((object["insert"] as? [String: Any])?["depth"] as? NSNumber)?.doubleValue ?? 260 }
        set { update { o in var i = o["insert"] as? [String: Any] ?? ["width": 520.0]; i["depth"] = newValue; o["insert"] = i } }
    }

    /// A wardrobe's hanging: `long` or `double`.
    var hanging: String {
        get { object["wardrobe"] as? String ?? "long" }
        set { update { $0["wardrobe"] = newValue } }
    }

    /// LED strips (zone, voltage, W/m), as the core's `lights`.
    var lights: [[String: Any]] {
        get { object["lights"] as? [[String: Any]] ?? [] }
        set { update { $0["lights"] = newValue.isEmpty ? nil : newValue } }
    }

    func hasLight(_ zone: String) -> Bool { lights.contains { $0["zone"] as? String == zone } }

    /// Switches a zone's strip on or off; a new strip takes the cabinet's other strips' voltage and
    /// power, or 24 V at 9.6 W/m.
    mutating func toggleLight(_ zone: String) {
        if hasLight(zone) {
            lights = lights.filter { $0["zone"] as? String != zone }
        } else {
            var light: [String: Any] = ["zone": zone, "voltage": lightVoltage, "watts_per_metre": lightWatts]
            if !lightProfile.isEmpty { light["profile"] = lightProfile }
            lights.append(light)
        }
    }

    /// "12v" or "24v": every strip of the cabinet together.
    var lightVoltage: String {
        get { lights.first?["voltage"] as? String ?? "24v" }
        set { lights = lights.map { var l = $0; l["voltage"] = newValue; return l } }
    }

    var lightWatts: Double {
        get { (lights.first?["watts_per_metre"] as? NSNumber)?.doubleValue ?? 9.6 }
        set { lights = lights.map { var l = $0; l["watts_per_metre"] = newValue; return l } }
    }

    /// The catalog's aluminium profile every strip of the cabinet sits in; empty for bare strips.
    var lightProfile: String {
        get { lights.first?["profile"] as? String ?? "" }
        set {
            lights = lights.map {
                var l = $0
                l["profile"] = newValue.isEmpty ? nil : newValue
                return l
            }
        }
    }

    /// The catalog board id the cabinet uses for `slot` (carcass, front, back); empty for generic.
    func board(for slot: String) -> String {
        (object["standard"] as? [String: Any])?["\(slot)_board"] as? String ?? ""
    }

    /// A sectioned front's columns, left to right (SEC-1).
    var sectionColumns: [SectionColumnDoc] {
        get {
            let fronts = object["fronts"] as? [String: Any]
            return (fronts?["columns"] as? [[String: Any]] ?? []).map(SectionColumnDoc.init(json:))
        }
        set { update { $0["fronts"] = ["type": "sections", "columns": newValue.map(\.json)] } }
    }

    /// Whether any section has this front, for options that only mean something with it.
    func hasFront(_ front: String) -> Bool {
        sectionColumns.contains { $0.rows.contains { $0.front.hasPrefix(front) } }
    }

    /// The front layout's type: open, door, door_pair, drawers, lift, bi_fold_lift, sections.
    var frontType: String {
        get {
            let fronts = object["fronts"] as? [String: Any]
            let type = fronts?["type"] as? String ?? "open"
            guard type == "stack" else { return type }
            // A stack is one of the app's presets: drawers in it make an oven tower.
            let sections = fronts?["sections"] as? [[String: Any]] ?? []
            return sections.contains { $0["type"] as? String == "drawers" } ? "oven_tower" : "fridge"
        }
        set {
            update { o in
                switch newValue {
                case "door": o["fronts"] = ["type": "door", "hinge_side": "left"]
                case "door_pair": o["fronts"] = ["type": "door_pair"]
                case "drawers": o["fronts"] = ["type": "drawers", "ratios": [1.0, 1.0, 1.0]]
                case "bi_fold_lift": o["fronts"] = ["type": "bi_fold_lift", "handle_weight": Self.handleWeight]
                case "lift": o["fronts"] = ["type": "lift", "kind": "stay_lift", "handle_weight": Self.handleWeight]
                // Two drawers, a 600 mm oven niche, a door above; a fridge niche under a door.
                case "oven_tower": o["fronts"] = ["type": "stack", "sections": [
                    ["type": "drawers", "count": 2, "height": 0],
                    ["type": "niche", "height": 600],
                    ["type": "door", "hinge_side": "left", "height": 0],
                ] as [[String: Any]]]
                case "fridge": o["fronts"] = ["type": "stack", "sections": [
                    ["type": "niche", "height": 0],
                    ["type": "door", "hinge_side": "left", "height": 350],
                ] as [[String: Any]]]
                default: o["fronts"] = ["type": "open"]
                }
                // The stacked sections bring their own fixed shelves.
                if ["oven_tower", "fridge"].contains(newValue) { o["adjustable_shelves"] = 0 }
            }
        }
    }

    var drawerRatios: [Double] {
        get { ((object["fronts"] as? [String: Any])?["ratios"] as? [NSNumber])?.map(\.doubleValue) ?? [] }
        set { update { $0["fronts"] = ["type": "drawers", "ratios": newValue] } }
    }

    /// A lifting front's movement: stay_lift, up_and_over, lift_up.
    var liftKind: String {
        get { (object["fronts"] as? [String: Any])?["kind"] as? String ?? "stay_lift" }
        set {
            update { o in
                let weight = (o["fronts"] as? [String: Any])?["handle_weight"] as? Double ?? Self.handleWeight
                o["fronts"] = ["type": "lift", "kind": newValue, "handle_weight": weight]
            }
        }
    }

    /// Handle weight (kg) for new lifting fronts: the owner's typical 400 g handle.
    static let handleWeight = 0.4

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
    case exact = "Exact"
    var id: String { rawValue }

    /// The core's name for the level.
    var core: String { rawValue.lowercased() }
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
    /// The welcome sheet is up (first run, or Help › Welcome to Cabinet Studio).
    var showWelcome = false
    var connection: ConnectionState = .unknown
    var status: String = ""
    var isSending = false
    var isMakingScene = false
    var lastError: String?
    var jobName = "Untitled job"
    /// The job's construction standard (JSON): every cabinet follows it except in the fields it
    /// overrides.
    var standardJson: String
    /// The job's sink and cooktop templates (a JSON array of the core's `Appliance`), which
    /// benchtop cut-outs are made from.
    var appliancesJson: String = defaultAppliancesJson()
    /// Picker changes go to the selected cabinet only, as an override, instead of the whole job.
    var onlyThisCabinet = false
    /// Where the job is saved; nil until it is.
    var fileURL: URL?
    /// SolidCraft's control port, from Settings.
    var port: UInt16 { Preferences.port }

    /// The last job build, by the items and placements it was built from.
    @ObservationIgnored private var jobBuild: (key: [String], items: [ItemBuild])?
    /// The window's undo manager. Every edit registers its reverse here, which is also how the
    /// document knows it has changes to save.
    @ObservationIgnored weak var undoManager: UndoManager?
    /// The cabinets as of the last registered edit: what undo goes back to.
    @ObservationIgnored private var committed = Committed(cabinets: [], standardJson: "", appliancesJson: "")
    /// Receives the job's file contents after every edit.
    @ObservationIgnored var onEdit: (@MainActor (Data) -> Void)?

    /// A new job: the cabinets of the job the app kept before it had documents, once, or a short
    /// sample run.
    init() {
        let legacy = Self.takeLegacyJob()
        let preferred = UserDefaults.standard.string(forKey: StandardLibrary.defaultKey)
            .flatMap { StandardLibrary.shared.standard(named: $0)?.standardJson }
        standardJson = legacy?.standardJson ?? preferred ?? JobContents.defaultStandard
        if let legacy {
            cabinets = legacy.cabinets
        }
        if cabinets.isEmpty {
            for kind in ["base", "drawer_bank", "base"] { add(kind: kind) }
        }
        selection = cabinets.first?.id
        committed = Committed(cabinets: cabinets, standardJson: standardJson, appliancesJson: appliancesJson)
    }

    init(contents: JobContents) {
        jobName = contents.name
        standardJson = contents.standardJson
        appliancesJson = contents.appliancesJson
        cabinets = contents.cabinets
        selection = cabinets.first?.id
        committed = Committed(cabinets: cabinets, standardJson: standardJson, appliancesJson: appliancesJson)
    }

    var selected: CabinetDoc? {
        get { cabinets.first { $0.id == selection } }
        set {
            guard let newValue, let i = cabinets.firstIndex(where: { $0.id == newValue.id }) else { return }
            cabinets[i] = newValue
            changed()
        }
    }

    /// The job as the core takes it: every item in order, and the cabinets kept where drawn.
    var jobItems: JobItems {
        JobItems(itemsJson: cabinets.map(\.json), placed: cabinets.filter { $0.placed == true }.map(\.id))
    }

    private var built: [ItemBuild] {
        let job = jobItems
        let key = job.itemsJson + ["placed"] + job.placed
        if let b = jobBuild, b.key == key { return b.items }
        let items = buildJob(job: job)
        jobBuild = (key, items)
        return items
    }

    /// The item's parts where the job puts them.
    func build(_ cab: CabinetDoc) -> BuildSummary? {
        guard let i = cabinets.firstIndex(where: { $0.id == cab.id }) else { return nil }
        let b = built
        return i < b.count ? b[i].summary : nil
    }

    func error(for cab: CabinetDoc) -> String? {
        guard let i = cabinets.firstIndex(where: { $0.id == cab.id }) else { return nil }
        let b = built
        return i < b.count ? b[i].error : nil
    }

    /// The job's whole STEP file, at the detail chosen. SolidCraft need not be open.
    func stepFile() async throws -> Data {
        let job = jobItems
        let level = detail.core
        let cli = Preferences.solidcraftCLI
        return try await Task.detached { try jobStep(items: job, detail: level, cli: cli) }.value
    }

    /// From the left end of the run to its right end.
    var runLength: Double {
        let xs = zip(cabinets, built).filter { $0.0.placed != true }
            .flatMap { $0.1.summary?.panels ?? [] }
            .flatMap { [$0.x, $0.x + $0.width] }
        guard let lo = xs.min(), let hi = xs.max() else { return 0 }
        return hi - lo
    }

    // MARK: Editing

    func add(kind: String) {
        let prefix = kind.prefix(1).uppercased()
        var n = 1
        while cabinets.contains(where: { $0.id == String(format: "%@%02d", prefix, n) }) { n += 1 }
        let id = String(format: "%@%02d", prefix, n)
        guard let json = try? newCabinet(id: id, kind: kind) else { return }
        var cab = CabinetDoc(id: id, json: json)
        cab.standard = projectStandard
        cabinets.append(cab)
        selection = id
        changed()
    }

    /// Adds a filler or end panel after the selected item (an end panel with nothing selected goes
    /// first, as the run's left end).
    func addPiece(_ type: String) {
        let prefix = switch type {
        case "filler": "F"
        case "benchtop": "BT"
        case "floating_shelf": "FS"
        case "cornice": "CO"
        case "light_rail": "LR"
        case "walls": "WL"
        case "splashback": "SB"
        case "gap": "G"
        default: "E"
        }
        var n = 1
        while cabinets.contains(where: { $0.id == String(format: "%@%02d", prefix, n) }) { n += 1 }
        var piece = CabinetDoc.piece(type, id: String(format: "%@%02d", prefix, n))
        // A gap belongs in the line of what it follows.
        if type == "gap", let s = selected, ["base", "drawer_bank", "sink", "blind_corner", "corner_base"].contains(s.kind) {
            piece.gapIsUpper = false
        }
        // A benchtop or moulding takes no place in the run: it goes last.
        let at = ["benchtop", "cornice", "light_rail", "walls", "splashback"].contains(type)
            ? cabinets.count
            : cabinets.firstIndex { $0.id == selection }.map { $0 + 1 } ?? 0
        withAnimation(.snappy) {
            cabinets.insert(piece, at: at)
            selection = piece.id
        }
        changed()
    }

    /// Selects the item `step` places after (or before) the selected one, stopping at the ends.
    func selectNeighbour(_ step: Int) {
        guard !cabinets.isEmpty else { return }
        let i = cabinets.firstIndex { $0.id == selection } ?? (step > 0 ? -1 : cabinets.count)
        let j = min(max(i + step, 0), cabinets.count - 1)
        withAnimation(.snappy) { selection = cabinets[j].id }
    }

    /// Deletes the selected item and selects its neighbour.
    func deleteSelection() {
        guard let id = selection, let i = cabinets.firstIndex(where: { $0.id == id }) else { return }
        withAnimation(.snappy) {
            cabinets.remove(at: i)
            selection = cabinets.isEmpty ? nil : cabinets[min(i, cabinets.count - 1)].id
        }
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

    /// The ABI handle style the job uses (`handle_style`), or nil before one is chosen.
    func handleStyleId(of cab: CabinetDoc) -> String? {
        let style = cab.standard["handle_style"] as? String ?? ""
        return style.isEmpty ? nil : style
    }

    /// A number in the cabinet's construction standard (mm).
    func standardNumber(_ field: String, of cab: CabinetDoc) -> Double {
        (cab.standard[field] as? NSNumber)?.doubleValue ?? 0
    }

    /// Sets a number in the selected cabinet's standard, for the whole job or only this cabinet
    /// as the construction scope says.
    func setStandardNumber(_ field: String, _ value: Double) {
        guard let cab = selected else { return }
        var edited = cab
        var standard = edited.standard
        standard[field] = value
        edited.standard = standard
        applyStandard(from: cab, to: edited)
    }

    func setOption(_ field: String, _ choice: String) {
        guard let cab = selected,
              let json = try? setStandardOption(cabinetJson: cab.json, field: field, choice: choice) else { return }
        var edited = cab
        edited.json = json
        // Switching to the ABI range lands on a style straight away: without one the core warns
        // and falls back to the generic handle. Switching back clears the choice.
        if field == "handle_range" {
            var standard = edited.standard
            let style = standard["handle_style"] as? String ?? ""
            if choice == "abi" {
                if style.isEmpty || !handleRecords.contains(where: { $0.id == style }) {
                    standard["handle_style"] = defaultAbiStyleId
                }
            } else {
                standard["handle_style"] = ""
            }
            edited.standard = standard
        }
        applyStandard(from: cab, to: edited)
    }

    /// The ABI style a job starts on when the range is switched without a choice: the Tezra 220
    /// pull in brushed brass, else whatever the catalog leads with.
    private var defaultAbiStyleId: String {
        handleRecords.first { $0.id == "handle.abi.tezra-220.brushed-brass" }?.id
            ?? handleRecords.first?.id ?? ""
    }

    // MARK: Handles

    /// The ABI handle catalog, loaded once (ABI-3).
    @ObservationIgnored private var handleRecordsCache: [HandleRecord]?
    var handleRecords: [HandleRecord] {
        if let c = handleRecordsCache { return c }
        let records = (try? CabinetStudio.abiHandles()) ?? []
        handleRecordsCache = records
        return records
    }

    /// Uses the ABI `record` (one family, size and finish) as the job's handle; fronts bore at
    /// its centres and the bill carries its SKU and price.
    func useHandle(_ record: HandleRecord) {
        guard let old = selected else {
            lastError = "Select a cabinet to choose a handle for."
            return
        }
        var cab = old
        var standard = cab.standard
        standard["handle_range"] = "abi"
        standard["handle_style"] = record.id
        // A pull over the edge has one place; a pull on the face can't stay there.
        if record.edgeMounted {
            standard["handle_place"] = "edge_pull"
        } else if standard["handle_place"] as? String == "edge_pull" {
            standard["handle_place"] = "corner"
        }
        cab.standard = standard
        withAnimation(.snappy) { applyStandard(from: old, to: cab) }
        lastError = nil
        status = "\(record.family) \(record.sizeName), \(record.variant)"
    }

    // MARK: Appliance library

    /// The job's sink and cooktop templates, each the core's `Appliance` as a dictionary.
    var appliances: [[String: Any]] {
        get { (try? JSONSerialization.jsonObject(with: Data(appliancesJson.utf8))) as? [[String: Any]] ?? [] }
        set {
            if let data = try? JSONSerialization.data(withJSONObject: newValue, options: [.sortedKeys]) {
                appliancesJson = String(decoding: data, as: UTF8.self)
            }
        }
    }

    /// Adds a template of `kind` (`sink`, `cooktop`), copied from the first of that kind.
    func addAppliance(kind: String) {
        var list = appliances
        var new = list.first { $0["kind"] as? String == kind } ?? ["kind": kind, "width": 600.0, "depth": 450.0]
        let names = Set(list.compactMap { $0["name"] as? String })
        let base = kind == "sink" ? "Sink" : "Cooktop"
        new["name"] = (1...).lazy.map { "\(base) \($0)" }.first { !names.contains($0) }!
        list.append(new)
        withAnimation(.snappy) { appliances = list }
        changed()
    }

    /// Sets `field` of template `index`, and re-cuts every benchtop cut-out made from it. A new
    /// name is carried to those cut-outs too.
    func setAppliance(_ index: Int, _ field: String, _ value: Any?) {
        var list = appliances
        guard list.indices.contains(index) else { return }
        let oldName = list[index]["name"] as? String ?? ""
        list[index][field] = value
        appliances = list
        let template = list[index]
        for i in cabinets.indices where cabinets[i].kind == "benchtop" {
            cabinets[i].recut(from: oldName, to: template)
        }
        changed()
    }

    /// Drops template `index`; cut-outs made from it stay as they are.
    func removeAppliance(_ index: Int) {
        var list = appliances
        guard list.indices.contains(index) else { return }
        list.remove(at: index)
        withAnimation(.snappy) { appliances = list }
        changed()
    }

    // MARK: Project standard

    var projectStandard: [String: Any] {
        get { (try? JSONSerialization.jsonObject(with: Data(standardJson.utf8))) as? [String: Any] ?? [:] }
        set {
            if let data = try? JSONSerialization.data(withJSONObject: newValue, options: [.sortedKeys]),
               let s = String(data: data, encoding: .utf8) {
                standardJson = s
            }
        }
    }

    private static func same(_ a: Any?, _ b: Any?) -> Bool {
        switch (a, b) {
        case (nil, nil): true
        case let (a as NSObject, b as NSObject): a.isEqual(b)
        default: false
        }
    }

    /// The standard fields in which `cab` differs from the job.
    func overrides(of cab: CabinetDoc) -> Set<String> {
        let job = projectStandard, own = cab.standard
        return Set(own.keys.filter { !Self.same(own[$0], job[$0]) })
    }

    /// Puts the selected cabinet's `fields` back to the job's values.
    func resetToProject(_ fields: [String]) {
        guard let cab = selected else { return }
        let job = projectStandard
        var c = cab
        var standard = c.standard
        for f in fields { standard[f] = job[f] }
        c.standard = standard
        withAnimation(.snappy) { selected = c }
    }

    /// Makes `json` the job's standard. Cabinets follow it except where they override a field.
    func applyJobStandard(_ json: String) {
        guard let new = (try? JSONSerialization.jsonObject(with: Data(json.utf8))) as? [String: Any] else { return }
        let job = projectStandard
        let fields = Set(new.keys).union(job.keys).filter { !Self.same(new[$0], job[$0]) }
        withAnimation(.snappy) {
            for i in cabinets.indices where !cabinets[i].isPiece {
                var standard = cabinets[i].standard
                for f in fields where Self.same(standard[f], job[f]) { standard[f] = new[f] }
                cabinets[i].standard = standard
            }
            projectStandard = new
        }
        changed()
    }

    /// Whether the job's standard is `json`, field for field.
    func jobStandardIs(_ json: String) -> Bool {
        guard let other = (try? JSONSerialization.jsonObject(with: Data(json.utf8))) as? [String: Any] else { return false }
        let job = projectStandard
        return Set(other.keys).union(job.keys).allSatisfy { Self.same(other[$0], job[$0]) }
    }

    /// Takes the standard fields that changed between `old` and `new` (one cabinet, before and after
    /// an edit) to the whole job: the job's standard and every cabinet that does not override them.
    /// With `onlyThisCabinet` the edit stays on that cabinet as an override.
    private func applyStandard(from old: CabinetDoc, to new: CabinetDoc) {
        guard !onlyThisCabinet else {
            selected = new
            return
        }
        let before = old.standard, after = new.standard
        let fields = after.keys.filter { !Self.same(before[$0], after[$0]) }
        var job = projectStandard
        for i in cabinets.indices where cabinets[i].id != new.id && !cabinets[i].isPiece {
            var standard = cabinets[i].standard
            for f in fields where Self.same(standard[f], job[f]) { standard[f] = after[f] }
            cabinets[i].standard = standard
        }
        for f in fields { job[f] = after[f] }
        projectStandard = job
        if let i = cabinets.firstIndex(where: { $0.id == new.id }) { cabinets[i] = new }
        changed()
    }

    /// Records an edit. The core lays the run out (`build_job`); cabinets drawn in SolidCraft stay
    /// where they were drawn.
    func changed() {
        save()
    }

    // MARK: SolidCraft

    func checkConnection() async {
        let port = self.port
        let ok = await Task.detached { solidcraftReachable(port: port) }.value
        connection = ok ? .connected : .offline
    }

    // MARK: Supplier hardware

    /// What the supplier CAD folder holds, after `prepareSupplierCAD` (nil before, or without a
    /// folder).
    var supplierCAD: SupplierCadReport?
    var isPreparingCAD = false
    @ObservationIgnored private var hardwareCache: (key: [String], models: [HardwareModel], shapes: [StandInShape])?

    /// Turns new supplier STEP files into meshes (once; later runs only check dates).
    func prepareSupplierCAD() async {
        guard let dir = Preferences.vendorCADDir?.path,
              FileManager.default.isExecutableFile(atPath: Preferences.solidcraftCLI) else {
            supplierCAD = nil
            return
        }
        isPreparingCAD = true
        let cli = Preferences.solidcraftCLI
        let result = await Task.detached { () -> Result<SupplierCadReport, Error> in
            Result { try prepareSupplierCad(vendorDir: dir, cli: cli) }
        }.value
        isPreparingCAD = false
        switch result {
        case .success(let r):
            supplierCAD = r
            hardwareCache = nil
            if r.converted > 0 { status = "Prepared \(r.converted) supplier models" }
        case .failure(let e):
            lastError = "Supplier CAD: \(e)"
        }
    }

    /// The job's hardware to draw: the supplier's own models where they are prepared, and stand-in
    /// shapes (legs, and hinges or runners without a model).
    func hardware() -> (models: [HardwareModel], shapes: [StandInShape]) {
        let job = jobItems
        let dir = Preferences.vendorCADDir?.path ?? ""
        let key = job.itemsJson + ["placed"] + job.placed + [dir, "\(supplierCAD?.models ?? 0)"]
        if let c = hardwareCache, c.key == key { return (c.models, c.shapes) }
        let models = (try? hardwareModels(job: job, vendorDir: dir)) ?? []
        let shapes = (try? standInShapes(job: job, vendorDir: dir)) ?? []
        hardwareCache = (key, models, shapes)
        return (models, shapes)
    }

    func sendToSolidCraft() async {
        // The colours the viewport shows, so SolidCraft looks the same.
        let colours = Dictionary(uniqueKeysWithValues: boards.map { ($0.id, $0.displayColour.hexString) })
        isSending = true
        lastError = nil
        status = "Building \(cabinets.count) cabinets in SolidCraft…"
        let job = jobItems
        let level = detail.core
        let port = self.port
        let vendor = Preferences.vendorCADDir?.path ?? ""
        let result = await Task.detached { () -> Result<String, Error> in
            Result { try sendToSolidcraft(job: job, detail: level, port: port, vendorDir: vendor, boardColours: colours) }
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

    /// Makes the job's scene in Blender: solidcraft-cli builds the cabinets, Blender dresses them in
    /// their boards in a lit kitchen, and the file opens. Beside the job file, once it has one.
    func makeBlenderScene() async {
        let workspace = NSWorkspace.shared
        func app(_ id: String, _ fallback: String) -> URL {
            workspace.urlForApplication(withBundleIdentifier: id) ?? URL(fileURLWithPath: fallback)
        }
        let solidCraft = app("io.tangencylabs.solidcraft", "/Applications/SolidCraft.app")
        let blender = app("org.blenderfoundation.blender", "/Applications/Blender.app")
        let cli = solidCraft.appending(path: "Contents/MacOS/solidcraft-cli")
        let blenderTool = blender.appending(path: "Contents/MacOS/Blender")
        guard FileManager.default.isExecutableFile(atPath: cli.path) else {
            lastError = "SolidCraft is not in Applications: it builds the cabinets for Blender."
            return
        }
        guard FileManager.default.isExecutableFile(atPath: blenderTool.path) else {
            lastError = "Blender is not in Applications. Install it from blender.org, then try again."
            return
        }
        guard let script = Bundle.main.url(forResource: "build_scene", withExtension: "py", subdirectory: "blender"),
              let catalog = Preferences.catalogDir else {
            lastError = "This copy of Cabinet Studio is missing its Blender script."
            return
        }
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let cache = support.appending(path: "CabinetStudio/polytec")
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let parent = fileURL?.deletingLastPathComponent() ?? documents.appending(path: "Cabinet Studio Scenes")
        let folder = parent.appending(path: "\(jobName) scene")
        let blend = folder.appending(path: "\(jobName).blend")
        let colours = Dictionary(uniqueKeysWithValues: boards.map { ($0.id, $0.displayColour.hexString) })
        let job = jobItems
        let vendor = Preferences.vendorCADDir?.path ?? ""
        isMakingScene = true
        lastError = nil
        status = "Building the cabinets for Blender…"
        let built = await Task.detached { () -> Result<Void, Error> in
            do {
                try writeBlenderJob(
                    job: job, catalogDir: catalog.path, vendorDir: vendor, colours: colours, folder: folder.path)
                try Self.run(cli, ["run", folder.appending(path: "build.json").path,
                                   "--out", folder.appending(path: "job.obj").path, "--quiet"])
                return .success(())
            } catch {
                return .failure(error)
            }
        }.value
        if case .failure(let e) = built {
            isMakingScene = false
            lastError = "\(e)"
            status = "SolidCraft did not build the cabinets for Blender"
            return
        }
        status = "Making the scene in Blender. The first use of a decor downloads it from Polytec…"
        let made = await Task.detached { () -> Result<Void, Error> in
            do {
                try Self.run(blenderTool, ["-b", "-P", script.path, "--",
                                           folder.appending(path: "job.json").path,
                                           folder.appending(path: "job.obj").path,
                                           blend.path, cache.path])
                return .success(())
            } catch {
                return .failure(error)
            }
        }.value
        isMakingScene = false
        switch made {
        case .success:
            status = "Scene saved as \(blend.lastPathComponent)"
            workspace.open([blend], withApplicationAt: blender, configuration: NSWorkspace.OpenConfiguration()) { _, _ in }
        case .failure(let e):
            lastError = "\(e)"
            status = "Blender did not make the scene"
        }
    }

    private struct ToolFailure: LocalizedError, CustomStringConvertible {
        let description: String
        var errorDescription: String? { description }
    }

    /// Runs a tool to the end. Its last lines say why, if it fails.
    nonisolated private static func run(_ tool: URL, _ arguments: [String]) throws {
        let process = Process()
        process.executableURL = tool
        process.arguments = arguments
        let log = FileManager.default.temporaryDirectory.appending(path: "cabinet-studio-\(UUID().uuidString).log")
        FileManager.default.createFile(atPath: log.path, contents: nil)
        defer { try? FileManager.default.removeItem(at: log) }
        let handle = try FileHandle(forWritingTo: log)
        defer { try? handle.close() }
        process.standardOutput = handle
        process.standardError = handle
        try process.run()
        process.waitUntilExit()
        let text = (try? String(contentsOf: log, encoding: .utf8)) ?? ""
        // Blender goes on with a script's error and exits 0, so the script's own word is checked too.
        if process.terminationStatus != 0 || text.contains("Traceback (most recent call last)") {
            let tail = text.split(separator: "\n").suffix(4).joined(separator: "\n")
            throw ToolFailure(description: "\(tool.lastPathComponent) failed: \(tail)")
        }
    }

    /// Renders the design in SolidCraft to a PNG next to the job file (or where the user picks
    /// for a job not saved yet) and shows it in the Finder.
    func render() async {
        let url: URL
        if let file = fileURL {
            url = file.deletingLastPathComponent().appendingPathComponent("\(jobName) render.png")
        } else {
            let panel = NSSavePanel()
            panel.allowedContentTypes = [.png]
            panel.nameFieldStringValue = "\(jobName) render.png"
            guard panel.runModal() == .OK, let picked = panel.url else { return }
            url = picked
        }
        isSending = true
        lastError = nil
        status = "Rendering in SolidCraft…"
        let port = self.port
        let path = url.path
        let job = jobItems
        let result = await Task.detached { () -> Result<String, Error> in
            Result { try renderPng(items: job, port: port, path: path, width: 2000, height: 1250) }
        }.value
        isSending = false
        switch result {
        case .success:
            status = "Rendered \(url.lastPathComponent)"
            NSWorkspace.shared.activateFileViewerSelecting([url])
        case .failure(let e):
            lastError = "\(e)"
            status = "SolidCraft did not render"
            await checkConnection()
        }
    }

    /// Uses `board` for the selected cabinet's carcass, fronts or back; the thickness follows.
    func useBoard(_ board: BoardRecord, for slot: String) {
        guard var cab = selected else {
            lastError = "Select a cabinet to use this board for."
            return
        }
        guard let dir = Preferences.catalogDir else { return }
        do {
            let old = cab
            cab.json = try CabinetStudio.assignBoard(cabinetJson: cab.json, slot: slot, boardId: board.id, catalogDir: dir.path)
            withAnimation(.snappy) { applyStandard(from: old, to: cab) }
            lastError = nil
            status = "\(cab.id) \(slot): \(board.decor) \(board.finish), \(Int(board.thickness)) mm"
        } catch {
            lastError = "\(error)"
        }
    }

    /// A framed door's panel board, empty for the same look as the fronts.
    func useShakerPanelBoard(_ id: String) {
        guard let old = selected else { return }
        var cab = old
        var standard = cab.standard
        standard["shaker_panel_board"] = id
        cab.standard = standard
        withAnimation(.snappy) { applyStandard(from: old, to: cab) }
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

    /// What undo goes back to.
    private struct Committed: Equatable {
        var cabinets: [CabinetDoc]
        var standardJson: String
        var appliancesJson: String
    }

    /// Registers the edit since the last one with the undo manager, which marks the document
    /// edited; the system saves it.
    func save() {
        let now = Committed(cabinets: cabinets, standardJson: standardJson, appliancesJson: appliancesJson)
        guard now != committed else { return }
        let before = committed
        committed = now
        undoManager?.registerUndo(withTarget: self) { model in
            MainActor.assumeIsolated { model.restore(before) }
        }
        if let data = try? encoded() { onEdit?(data) }
    }

    private func restore(_ state: Committed) {
        withAnimation(.snappy) {
            cabinets = state.cabinets
            standardJson = state.standardJson
            appliancesJson = state.appliancesJson
            if !cabinets.contains(where: { $0.id == selection }) { selection = cabinets.first?.id }
        }
        changed()
    }

    /// The job file: the core's project JSON.
    func encoded() throws -> Data {
        let text = try projectJson(name: jobName, standardJson: standardJson, itemsJson: cabinets.map(\.json),
                                   placed: cabinets.filter { $0.placed == true }.map(\.id),
                                   appliancesJson: appliancesJson)
        return Data(text.utf8)
    }

    /// The single job file the app kept in Application Support before jobs were documents. It is
    /// read into the first new job and then renamed, so it is imported once.
    private static func takeLegacyJob() -> JobContents? {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("CabinetStudio", isDirectory: true)
        let url = dir.appendingPathComponent("project.json")
        guard let data = try? Data(contentsOf: url), let job = try? JobContents(data: data) else { return nil }
        try? FileManager.default.moveItem(at: url, to: dir.appendingPathComponent("project-imported.json"))
        return job
    }

    /// The bundled board catalog by id.
    @ObservationIgnored private(set) lazy var boardsById: [String: BoardRecord] =
        Dictionary(boards.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })

    /// The bundled board catalog.
    @ObservationIgnored private(set) lazy var boards: [BoardRecord] = {
        guard let dir = Preferences.catalogDir else { return [] }
        return (try? loadBoards(catalogDir: dir.path)) ?? []
    }()
}

/// A job as read from its file.
struct JobContents: Sendable {
    var name: String
    var standardJson: String
    var cabinets: [CabinetDoc]
    var appliancesJson = defaultAppliancesJson()

    /// The core's default standard, for jobs that predate the job standard.
    static var defaultStandard: String {
        guard let json = try? newCabinet(id: "X01", kind: "base"),
              let o = (try? JSONSerialization.jsonObject(with: Data(json.utf8))) as? [String: Any],
              let s = o["standard"],
              let data = try? JSONSerialization.data(withJSONObject: s, options: [.sortedKeys])
        else { return "{}" }
        return String(decoding: data, as: UTF8.self)
    }

    /// Before jobs had one standard, a file listed each cabinet's full JSON.
    private struct Legacy: Codable {
        var jobName: String
        var cabinets: [CabinetDoc]
    }

    init(data: Data) throws {
        if let p = try? openProject(json: String(decoding: data, as: UTF8.self)) {
            name = p.name
            standardJson = p.standardJson
            appliancesJson = p.appliancesJson
            cabinets = p.itemsJson.compactMap { json in
                CabinetDoc.id(of: json).map { CabinetDoc(id: $0, json: json, placed: p.placed.contains($0) ? true : nil) }
            }
            return
        }
        let legacy = try JSONDecoder().decode(Legacy.self, from: data)
        name = legacy.jobName
        cabinets = legacy.cabinets.map { var c = $0; c.json = sectionedCabinet(json: c.json); return c }
        // The first cabinet's standard becomes the job's; the others keep theirs as overrides.
        if let first = legacy.cabinets.first,
           let data = try? JSONSerialization.data(withJSONObject: first.standard, options: [.sortedKeys]) {
            standardJson = String(decoding: data, as: UTF8.self)
        } else {
            standardJson = Self.defaultStandard
        }
    }
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
        case "corner_base": "L-corner"
        case "corner_wall": "Corner wall"
        case "wardrobe": "Wardrobe"
        case "rangehood": "Rangehood canopy"
        case "sink": "Sink"
        case "filler": "Filler"
        case "end_panel": "End panel"
        case "benchtop": "Benchtop"
        case "floating_shelf": "Floating shelf"
        case "cornice": "Cornice"
        case "light_rail": "Light rail"
        case "walls": "Return walls"
        case "splashback": "Splashback"
        case "gap": "Gap"
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
        case "corner_base": "l.square"
        case "corner_wall": "l.square.fill"
        case "wardrobe": "cabinet"
        case "rangehood": "wind"
        case "sink": "drop"
        case "filler": "rectangle.portrait.split.2x1"
        case "end_panel": "rectangle.portrait.lefthalf.filled"
        case "benchtop": "rectangle.tophalf.filled"
        case "floating_shelf": "minus.rectangle"
        case "cornice": "rectangle.topthird.inset.filled"
        case "light_rail": "rectangle.bottomthird.inset.filled"
        case "walls": "square.leftthird.inset.filled"
        case "splashback": "square.grid.3x3.fill"
        case "gap": "square.dashed"
        default: "cube"
        }
    }
}
