import Foundation

/// One section of a sectioned cabinet (SEC-1): its fronts and what is inside, as the core's
/// `Section`. Heights and widths of 0 share what the others leave.
struct SectionCell: Equatable {
    var height: Double = 0
    /// open, door_left, door_right, door_pair, drawers.
    var front = "open"
    var drawers = 2
    /// empty, shelves, shoe_shelves, hanging, inner_drawers.
    var inside = "empty"
    var insideCount = 2

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
        default: front = "open"
        }
        let i = json["inside"] as? [String: Any] ?? [:]
        inside = i["type"] as? String ?? "empty"
        insideCount = (i["count"] as? NSNumber)?.intValue ?? 2
    }

    var json: [String: Any] {
        let f: [String: Any] = switch front {
        case "door_left": ["type": "door", "hinge_side": "left"]
        case "door_right": ["type": "door", "hinge_side": "right"]
        case "door_pair": ["type": "door_pair"]
        case "drawers": ["type": "drawers", "count": drawers]
        default: ["type": "open"]
        }
        var i: [String: Any] = ["type": inside]
        if ["shelves", "shoe_shelves", "inner_drawers"].contains(inside) { i["count"] = insideCount }
        return ["height": height, "front": f, "inside": i]
    }

    /// Inner drawers only go behind doors or in the open.
    var insideOptions: [(id: String, title: String)] {
        let all = [("empty", "Empty"), ("shelves", "Shelves"), ("shoe_shelves", "Shoes"),
                   ("hanging", "Hanging"), ("inner_drawers", "Inner drawers")]
        return front == "drawers" ? Array(all.prefix(1)) : all
    }
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
}
