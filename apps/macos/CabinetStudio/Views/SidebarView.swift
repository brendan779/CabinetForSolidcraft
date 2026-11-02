import SwiftUI

struct SidebarView: View {
    @Environment(ProjectModel.self) private var project

    var body: some View {
        @Bindable var project = project
        List(selection: $project.selection) {
            Section {
                ForEach(project.cabinets) { cab in
                    CabinetRow(cab: cab)
                        .tag(cab.id)
                        .contextMenu {
                            Button("Delete", role: .destructive) { project.remove(cab.id) }
                        }
                }
                .onMove { project.move(from: $0, to: $1) }
                .onDelete { idx in idx.map { project.cabinets[$0].id }.forEach(project.remove) }
            } header: {
                HStack {
                    Text(project.jobName)
                    Spacer()
                    Menu {
                        ForEach(cabinetKinds(), id: \.self) { kind in
                            Button {
                                project.add(kind: kind)
                            } label: {
                                Label(CabinetDoc.kindTitle(kind), systemImage: CabinetDoc.kindSymbol(kind))
                            }
                        }
                        Divider()
                        ForEach(["filler", "end_panel", "benchtop", "floating_shelf", "cornice", "light_rail"], id: \.self) { type in
                            Button {
                                project.addPiece(type)
                            } label: {
                                Label(CabinetDoc.kindTitle(type), systemImage: CabinetDoc.kindSymbol(type))
                            }
                        }
                        Divider()
                        Button {
                            project.addPiece("walls")
                        } label: {
                            Label(CabinetDoc.kindTitle("walls"), systemImage: CabinetDoc.kindSymbol("walls"))
                        }
                        .disabled(project.cabinets.contains { $0.kind == "walls" })
                    } label: {
                        Image(systemName: "plus")
                    }
                    .menuStyle(.borderlessButton)
                    .menuIndicator(.hidden)
                    .fixedSize()
                    .help("Add a cabinet")
                }
            }
        }
        .listStyle(.sidebar)
        .scrollContentBackground(.hidden)
        .background(Theme.panel)
    }
}

struct CabinetRow: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    let cab: CabinetDoc

    var body: some View {
        HStack(spacing: 10) {
            CabinetGlyph(doc: cab)
                .frame(width: 26, height: 30)
            VStack(alignment: .leading, spacing: 1) {
                HStack(spacing: 6) {
                    Text(cab.id).font(.headline).monospaced()
                    Text(CabinetDoc.kindTitle(cab.kind)).foregroundStyle(.secondary)
                }
                Text(dimensions)
                    .font(.caption)
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if project.error(for: cab) != nil {
                Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(Theme.warning)
            }
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }

    private var dimensions: String {
        switch cab.kind {
        case "filler": "\(unit.format(cab.width)) + \(unit.format(cab.scribe, suffix: false)) scribe"
        case "end_panel": cab.scribe > 0 ? "\(unit.format(cab.scribe)) scribe" : "no scribe"
        case "benchtop": "\(unit.format(cab.topThickness)) thick, \(unit.format(cab.frontOverhang, suffix: false)) overhang"
        case "floating_shelf": "\(unit.format(cab.width)) × \(unit.format(cab.shelfDepth, suffix: false))"
        case "cornice", "light_rail": "\(unit.format(cab.mouldingHeight)) high\(cab.returns ? ", returned" : "")"
        case "walls":
            switch (cab.wallLeft, cab.wallRight) {
            case (true, true): cab.wallLength > 0 ? "\(unit.format(cab.wallLength)) between" : "Both ends"
            case (true, false): "Left end"
            case (false, true): "Right end"
            default: "Back wall only"
            }
        default: [cab.width, cab.height, cab.depth].map { unit.format($0, suffix: false) }.joined(separator: " × ")
        }
    }
}

/// A tiny front elevation of a cabinet, drawn from its fronts.
struct CabinetGlyph: View {
    let doc: CabinetDoc

    var body: some View {
        Canvas { ctx, size in
            if doc.kind == "walls" {
                WallPlan.draw(ctx, size, left: doc.wallLeft, right: doc.wallRight)
                return
            }
            if ["benchtop", "cornice", "light_rail"].contains(doc.kind) {
                let y = switch doc.kind {
                case "cornice": size.height * 0.05
                case "light_rail": size.height * 0.75
                default: size.height * 0.35
                }
                let r = CGRect(x: 0, y: y, width: size.width, height: size.height * 0.2)
                ctx.fill(Path(roundedRect: r, cornerRadius: 1.5), with: .color(.primary.opacity(0.3)))
                ctx.stroke(Path(roundedRect: r, cornerRadius: 1.5), with: .color(.primary.opacity(0.8)), lineWidth: 1)
                return
            }
            if doc.isPiece {
                // A narrow upright strip; an end panel's scribe edge hatched.
                let w = size.width * (doc.kind == "filler" ? 0.3 : 0.22)
                let r = CGRect(x: (size.width - w) / 2, y: 0, width: w, height: size.height)
                ctx.fill(Path(roundedRect: r, cornerRadius: 1.5), with: .color(.primary.opacity(0.15)))
                ctx.stroke(Path(roundedRect: r, cornerRadius: 1.5), with: .color(.primary.opacity(0.8)), lineWidth: 1)
                if doc.scribe > 0 {
                    var p = Path()
                    p.move(to: CGPoint(x: r.maxX + 2, y: r.minY))
                    p.addLine(to: CGPoint(x: r.maxX + 2, y: r.maxY))
                    ctx.stroke(p, with: .color(.accentColor), style: StrokeStyle(lineWidth: 1, dash: [2, 2]))
                }
                return
            }
            let aspect = doc.width / max(doc.height, 1)
            let h = size.height
            let w = min(size.width, h * aspect)
            let r = CGRect(x: (size.width - w) / 2, y: 0, width: w, height: h)
            ctx.stroke(Path(roundedRect: r, cornerRadius: 1.5), with: .color(.primary.opacity(0.8)), lineWidth: 1)
            // The cabinet's sections, as in the inspector's drawing.
            let columns = doc.sectionColumns
            let widths = Sections.shares(columns.map(\.width), of: r.width)
            var x = r.minX
            func line(_ a: CGPoint, _ b: CGPoint, _ opacity: Double = 0.6) {
                var p = Path()
                p.move(to: a)
                p.addLine(to: b)
                ctx.stroke(p, with: .color(.primary.opacity(opacity)), lineWidth: 0.8)
            }
            for (i, column) in columns.enumerated() {
                let heights = Sections.shares(column.rows.map(\.height), of: r.height)
                var y = r.maxY
                for (row, h) in zip(column.rows, heights) {
                    if y < r.maxY { line(CGPoint(x: x, y: y), CGPoint(x: x + widths[i], y: y)) }
                    let cell = CGRect(x: x, y: y - h, width: widths[i], height: h)
                    switch row.front {
                    case "door_pair":
                        line(CGPoint(x: cell.midX, y: cell.minY), CGPoint(x: cell.midX, y: cell.maxY))
                    case "bi_fold":
                        line(CGPoint(x: cell.minX, y: cell.midY), CGPoint(x: cell.maxX, y: cell.midY))
                    case "sliding":
                        for k in 1..<max(row.slidingCount, 2) {
                            let sx = cell.minX + cell.width * CGFloat(k) / CGFloat(max(row.slidingCount, 2))
                            line(CGPoint(x: sx, y: cell.minY), CGPoint(x: sx, y: cell.maxY))
                        }
                    case "drawers":
                        let ratios = row.drawerRatios
                        let total = ratios.reduce(0, +)
                        var dy = cell.minY
                        for ratio in ratios.dropLast() {
                            dy += cell.height * ratio / max(total, 0.001)
                            line(CGPoint(x: cell.minX, y: dy), CGPoint(x: cell.maxX, y: dy))
                        }
                    case "open":
                        if row.inside == "shelves" || row.inside == "shoe_shelves" {
                            for k in 1...max(row.insideCount, 1) {
                                let sy = cell.maxY - cell.height * CGFloat(k) / CGFloat(max(row.insideCount, 1) + 1)
                                line(CGPoint(x: cell.minX + 2, y: sy), CGPoint(x: cell.maxX - 2, y: sy), 0.4)
                            }
                        }
                    default:
                        break
                    }
                    y -= h
                }
                x += widths[i]
                if i + 1 < columns.count { line(CGPoint(x: x, y: r.minY), CGPoint(x: x, y: r.maxY)) }
            }
        }
        .accessibilityHidden(true)
    }
}
