import Foundation

/// One section of a cabinet: its fronts and what is inside, as the core's `Section`. Heights and
/// widths of 0 share what the others leave.
struct SectionCell: Equatable {
    var height: Double = 0
    /// open, door_left, door_right, door_pair, drawers, lift, bi_fold, niche.
    var front = "open"
    var drawers = 2
    /// The drawers' height proportions, top to bottom. Empty: all the same.
    var ratios: [Double] = []
    /// The drawer system, where it differs from the job's.
    var system: String?
    /// A lifting front's movement, and its handle's weight (kg).
    var liftKind = "stay_lift"
    var handleWeight = SectionCell.defaultHandleWeight
    /// empty, shelves, shoe_shelves, hanging, inner_drawers, wardrobe.
    var inside = "empty"
    var insideCount = 2
    /// Which adjustable shelves are fixed instead (from 1), kept from older jobs.
    var fixed: [Int] = []
    /// A fixed shelf between this section and the one above.
    var shelfAbove = false
    /// How its fronts open, where it differs from the job's: handles, tip_on, servo_drive.
    var opening: String?

    /// The owner's typical 400 g handle.
    static let defaultHandleWeight = 0.4

    init(front: String = "open", inside: String = "empty") {
        self.front = front
        self.inside = inside
    }

    init(json: [String: Any]) {
        height = (json["height"] as? NSNumber)?.doubleValue ?? 0
        let f = json["front"] as? [String: Any] ?? [:]
        switch f["type"] as? String ?? "open" {
        case "door": front = (f["hinge_side"] as? String) == "right" ? "door_right" : "door_left"
        case "door_pair": front = "door_pair"
        case "drawers":
            front = "drawers"
            drawers = (f["count"] as? NSNumber)?.intValue ?? 2
            ratios = (f["ratios"] as? [NSNumber])?.map(\.doubleValue) ?? []
            system = f["system"] as? String
        case "lift":
            front = "lift"
            liftKind = f["kind"] as? String ?? "stay_lift"
            handleWeight = (f["handle_weight"] as? NSNumber)?.doubleValue ?? Self.defaultHandleWeight
        case "bi_fold":
            front = "bi_fold"
            handleWeight = (f["handle_weight"] as? NSNumber)?.doubleValue ?? Self.defaultHandleWeight
        case "niche": front = "niche"
        default: front = "open"
        }
        let i = json["inside"] as? [String: Any] ?? [:]
        inside = i["type"] as? String ?? "empty"
        insideCount = (i["count"] as? NSNumber)?.intValue ?? 2
        fixed = (i["fixed"] as? [NSNumber])?.map(\.intValue) ?? []
        // The core's rule for jobs saved before the choice existed.
        shelfAbove = (json["shelf_above"] as? Bool) ?? true
        opening = json["opening"] as? String
    }

    var json: [String: Any] {
        var f: [String: Any]
        switch front {
        case "door_left": f = ["type": "door", "hinge_side": "left"]
        case "door_right": f = ["type": "door", "hinge_side": "right"]
        case "door_pair": f = ["type": "door_pair"]
        case "drawers":
            f = ["type": "drawers", "count": drawers]
            if !ratios.isEmpty { f["ratios"] = ratios }
            if let system { f["system"] = system }
        case "lift": f = ["type": "lift", "kind": liftKind, "handle_weight": handleWeight]
        case "bi_fold": f = ["type": "bi_fold", "handle_weight": handleWeight]
        case "niche": f = ["type": "niche"]
        default: f = ["type": "open"]
        }
        var i: [String: Any] = ["type": inside]
        if ["shelves", "shoe_shelves", "inner_drawers"].contains(inside) { i["count"] = insideCount }
        if inside == "shelves", !fixed.isEmpty { i["fixed"] = fixed }
        var out: [String: Any] = ["height": height, "front": f, "inside": i, "shelf_above": shelfAbove]
        if let opening { out["opening"] = opening }
        return out
    }

    /// The stack of drawers as proportions, top to bottom.
    var drawerRatios: [Double] {
        ratios.isEmpty ? Array(repeating: 1, count: max(drawers, 1)) : ratios
    }

    /// What can go behind this front: drawers and niches leave no room. A wardrobe's own interior
    /// is the whole cabinet's.
    func insideOptions(wardrobe: Bool) -> [(id: String, title: String)] {
        if front == "drawers" || front == "niche" { return [("empty", "Empty")] }
        var all = [("empty", "Empty"), ("shelves", "Adjustable"), ("shoe_shelves", "Fixed"),
                   ("hanging", "Hanging"), ("inner_drawers", "Inner drawers")]
        if wardrobe { all.append(("wardrobe", "Wardrobe")) }
        return all
    }

    /// Fronts that open with handles, TIP-ON or SERVO-DRIVE.
    var opens: Bool { ["door_left", "door_right", "door_pair", "drawers"].contains(front) }
}

/// A column of sections, bottom to top, between the cabinet's sides or dividers.
struct SectionColumnDoc: Equatable {
    var width: Double = 0
    var rows: [SectionCell]

    init(width: Double = 0, rows: [SectionCell]) {
        self.width = width
        self.rows = rows
    }

    init(json: [String: Any]) {
        width = (json["width"] as? NSNumber)?.doubleValue ?? 0
        rows = (json["rows"] as? [[String: Any]] ?? []).map(SectionCell.init(json:))
        if rows.isEmpty { rows = [SectionCell()] }
    }

    var json: [String: Any] { ["width": width, "rows": rows.map(\.json)] }

    /// A new section above row `r`. The split makes no shelf: one the row already had over it
    /// moves up, to stay over the pair, and the top row's unused flag is dropped.
    mutating func splitAcross(_ r: Int) {
        var fresh = SectionCell()
        fresh.shelfAbove = r + 1 < rows.count && rows[r].shelfAbove
        rows[r].shelfAbove = false
        rows.insert(fresh, at: r + 1)
    }
}

enum Sections {
    /// Sizes for drawing: a size of 0 takes an even share of what the set sizes leave.
    static func shares(_ asks: [Double], of total: Double) -> [Double] {
        let set = asks.filter { $0 > 0 }.reduce(0, +)
        let free = asks.filter { $0 <= 0 }.count
        let each = free > 0 ? max(total - set, 0) / Double(free) : 0
        let sizes = asks.map { $0 > 0 ? $0 : each }
        // Asks that overrun the cabinet still draw inside it.
        let sum = sizes.reduce(0, +)
        return sum > total && sum > 0 ? sizes.map { $0 * total / sum } : sizes
    }

    /// Whole-cabinet layouts to start from; each is ordinary sections to split and change.
    static func layout(_ id: String) -> [SectionColumnDoc] {
        func one(_ cell: SectionCell) -> [SectionColumnDoc] { [SectionColumnDoc(rows: [cell])] }
        switch id {
        case "door": return one(SectionCell(front: "door_left"))
        case "door_pair": return one(SectionCell(front: "door_pair"))
        case "drawers":
            var c = SectionCell(front: "drawers")
            c.drawers = 3
            return one(c)
        case "lift": return one(SectionCell(front: "lift"))
        case "bi_fold": return one(SectionCell(front: "bi_fold"))
        case "oven":
            // Two drawers, a 600 mm oven niche, a door above, on fixed shelves.
            var drawers = SectionCell(front: "drawers")
            drawers.shelfAbove = true
            var niche = SectionCell(front: "niche")
            niche.height = 600
            niche.shelfAbove = true
            return [SectionColumnDoc(rows: [drawers, niche, SectionCell(front: "door_left")])]
        case "fridge":
            var niche = SectionCell(front: "niche")
            niche.shelfAbove = true
            var door = SectionCell(front: "door_left")
            door.height = 350
            return [SectionColumnDoc(rows: [niche, door])]
        default: return one(SectionCell())
        }
    }
}
