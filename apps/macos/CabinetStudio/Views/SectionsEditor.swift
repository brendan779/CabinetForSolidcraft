import SwiftUI

/// SEC-1: the cabinet's front drawn to scale, split into columns by dividers and rows by fixed
/// shelves. Click a section, then pick its front and what goes inside, as in Blum's configurator.
struct SectionsEditor: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    @State private var column = 0
    @State private var row = 0

    private let fronts: [(id: String, title: String)] = [
        ("open", "Open"), ("door_left", "Door L"), ("door_right", "Door R"), ("door_pair", "Pair"), ("drawers", "Drawers"),
    ]

    var body: some View {
        let columns = cab.sectionColumns
        let (c, r) = clamped(columns)
        let cell = columns.isEmpty ? SectionCell() : columns[c].rows[r]
        VStack(alignment: .leading, spacing: 12) {
            elevation(columns, selected: (c, r))
            HStack(spacing: 6) {
                Button { edit { $0.insert(SectionColumnDoc(rows: [SectionCell()]), at: c + 1) }; select(c + 1, 0) } label: {
                    Label("Divider", systemImage: "rectangle.split.2x1")
                }
                .help("Add a divider to the right of this section: a new column of sections")
                Button { edit { $0[c].rows.insert(SectionCell(), at: r + 1) }; select(c, r + 1) } label: {
                    Label("Shelf", systemImage: "rectangle.split.1x2")
                }
                .help("Add a fixed shelf over this section: a new section above it")
                Spacer(minLength: 4)
                Button(role: .destructive) { remove(c, r, from: columns) } label: {
                    Label("Remove", systemImage: "minus.rectangle")
                }
                .disabled(columns.count <= 1 && (columns.first?.rows.count ?? 0) <= 1)
                .help("Remove this section: the ones beside it take its room")
            }
            .controlSize(.small)
            .buttonStyle(.bordered)
            .labelStyle(.titleAndIcon)

            SectionTitle("Front")
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 5), spacing: 8) {
                ForEach(fronts, id: \.id) { front in
                    Button {
                        editCell(c, r) {
                            $0.front = front.id
                            if front.id == "drawers", $0.inside != "empty" { $0.inside = "empty" }
                        }
                    } label: {
                        VStack(spacing: 4) {
                            SectionCellThumbnail(cell: SectionCell(front: front.id))
                                .frame(width: 28, height: 34)
                            Text(front.title).font(.caption2).lineLimit(1).minimumScaleFactor(0.7)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 6)
                    }
                    .buttonStyle(TileStyle(selected: cell.front == front.id))
                    .accessibilityLabel("Front: \(front.title)")
                }
            }
            if cell.front == "drawers" {
                ChipRow("Drawers") {
                    ForEach(1...5, id: \.self) { n in
                        Button("\(n)") { editCell(c, r) { $0.drawers = n } }
                            .buttonStyle(ChipStyle(selected: cell.drawers == n))
                    }
                }
            }

            SectionTitle("Inside")
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 5), spacing: 8) {
                ForEach(cell.insideOptions, id: \.id) { inside in
                    Button { editCell(c, r) { $0.inside = inside.id } } label: {
                        VStack(spacing: 4) {
                            SectionCellThumbnail(cell: SectionCell(inside: inside.id))
                                .frame(width: 28, height: 34)
                            Text(inside.title).font(.caption2).lineLimit(1).minimumScaleFactor(0.7)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 6)
                    }
                    .buttonStyle(TileStyle(selected: cell.inside == inside.id))
                    .accessibilityLabel("Inside: \(inside.title)")
                }
            }
            if ["shelves", "shoe_shelves", "inner_drawers"].contains(cell.inside) {
                ChipRow(cell.inside == "inner_drawers" ? "Drawers" : "Shelves") {
                    ForEach(1...5, id: \.self) { n in
                        Button("\(n)") { editCell(c, r) { $0.insideCount = n } }
                            .buttonStyle(ChipStyle(selected: cell.insideCount == n))
                    }
                }
            }

            ChipRow("Height") {
                ForEach([0.0, 300, 450, 600, 900], id: \.self) { h in
                    Button(h == 0 ? "Auto" : "\(Int(h))") { editCell(c, r) { $0.height = h } }
                        .buttonStyle(ChipStyle(selected: abs(cell.height - h) < 0.5))
                }
            }
            .help("This section's fronts' height; Auto shares what the others leave")
            if columns.count > 1 {
                ChipRow("Width") {
                    ForEach([0.0, 300, 450, 600], id: \.self) { w in
                        Button(w == 0 ? "Auto" : "\(Int(w))") { edit { $0[c].width = w } }
                            .buttonStyle(ChipStyle(selected: abs(columns[c].width - w) < 0.5))
                    }
                }
                .help("This column's fronts' width; Auto shares what the others leave")
            }
        }
    }

    /// The front elevation: columns left to right, rows bottom to top, each a button.
    private func elevation(_ columns: [SectionColumnDoc], selected: (Int, Int)) -> some View {
        let widths = Sections.shares(columns.map(\.width), of: max(cab.width, 1))
        let aspect = max(cab.width, 1) / max(cab.height, 1)
        return GeometryReader { geo in
            let scale = geo.size.width / max(cab.width, 1)
            HStack(spacing: 3) {
                ForEach(columns.indices, id: \.self) { ci in
                    let col = columns[ci]
                    let heights = Sections.shares(col.rows.map(\.height), of: max(cab.height, 1))
                    VStack(spacing: 3) {
                        ForEach(col.rows.indices.reversed(), id: \.self) { ri in
                            let isSelected = selected == (ci, ri)
                            Button { select(ci, ri) } label: {
                                SectionCellThumbnail(cell: col.rows[ri], selected: isSelected)
                            }
                            .buttonStyle(.plain)
                            .frame(height: max(heights[ri] * geo.size.height / max(cab.height, 1) - 3, 8))
                            .accessibilityLabel("Section \(ci + 1), row \(ri + 1)")
                            .accessibilityAddTraits(isSelected ? .isSelected : [])
                        }
                    }
                    .frame(width: max(widths[ci] * scale - 3, 8))
                }
            }
            .frame(width: geo.size.width, height: geo.size.height, alignment: .leading)
        }
        .aspectRatio(aspect, contentMode: .fit)
        .frame(maxWidth: .infinity, maxHeight: 260)
        .animation(.snappy, value: columns)
    }

    private func clamped(_ columns: [SectionColumnDoc]) -> (Int, Int) {
        guard !columns.isEmpty else { return (0, 0) }
        let c = min(column, columns.count - 1)
        return (c, min(row, columns[c].rows.count - 1))
    }

    private func select(_ c: Int, _ r: Int) {
        withAnimation(.snappy) { column = c; row = r }
    }

    private func edit(_ change: (inout [SectionColumnDoc]) -> Void) {
        withAnimation(.snappy) {
            var doc = cab
            var columns = doc.sectionColumns
            change(&columns)
            doc.sectionColumns = columns
            project.selected = doc
        }
    }

    private func editCell(_ c: Int, _ r: Int, _ change: (inout SectionCell) -> Void) {
        edit { change(&$0[c].rows[r]) }
    }

    private func remove(_ c: Int, _ r: Int, from columns: [SectionColumnDoc]) {
        if columns[c].rows.count > 1 {
            edit { $0[c].rows.remove(at: r) }
            select(c, max(r - 1, 0))
        } else if columns.count > 1 {
            edit { $0.remove(at: c) }
            select(max(c - 1, 0), 0)
        }
    }
}

/// One section's front with what is inside showing faintly through it.
struct SectionCellThumbnail: View {
    let cell: SectionCell
    var selected = false

    var body: some View {
        Canvas { ctx, size in
            let r = CGRect(origin: .zero, size: size).insetBy(dx: 1, dy: 1)
            let frame = Path(roundedRect: r, cornerRadius: 3)
            ctx.fill(frame, with: .color(selected ? Color.accentColor.opacity(0.15) : .primary.opacity(cell.front == "open" ? 0.03 : 0.10)))
            ctx.stroke(frame, with: .color(selected ? Color.accentColor : .primary.opacity(0.7)), lineWidth: selected ? 2 : 1.1)
            let faint = GraphicsContext.Shading.color(.primary.opacity(0.35))
            func line(_ a: CGPoint, _ b: CGPoint, _ shading: GraphicsContext.Shading, _ width: CGFloat = 1) {
                var p = Path(); p.move(to: a); p.addLine(to: b)
                ctx.stroke(p, with: shading, lineWidth: width)
            }
            func handle(_ x: CGFloat, _ y: CGFloat, vertical: Bool) {
                let s = min(r.width, r.height) * 0.2
                let rect = vertical ? CGRect(x: x - 1, y: y - s, width: 2, height: 2 * s) : CGRect(x: x - s, y: y - 1, width: 2 * s, height: 2)
                ctx.fill(Path(roundedRect: rect, cornerRadius: 1), with: .color(.accentColor))
            }
            // Inside, behind the front.
            let n = max(cell.insideCount, 1)
            switch cell.inside {
            case "shelves", "shoe_shelves":
                for i in 1...n {
                    let y = r.maxY - r.height * CGFloat(i) / CGFloat(n + 1)
                    if cell.inside == "shoe_shelves" {
                        line(CGPoint(x: r.minX + 3, y: y + 3), CGPoint(x: r.maxX - 3, y: y - 3), faint)
                    } else {
                        line(CGPoint(x: r.minX + 3, y: y), CGPoint(x: r.maxX - 3, y: y), faint)
                    }
                }
            case "hanging":
                let y = r.minY + min(8, r.height * 0.2)
                line(CGPoint(x: r.minX + 3, y: y), CGPoint(x: r.maxX - 3, y: y), faint, 2)
                var hanger = Path()
                hanger.move(to: CGPoint(x: r.midX, y: y))
                hanger.addLine(to: CGPoint(x: r.midX - r.width * 0.25, y: y + r.height * 0.18))
                hanger.addLine(to: CGPoint(x: r.midX + r.width * 0.25, y: y + r.height * 0.18))
                hanger.closeSubpath()
                ctx.stroke(hanger, with: faint, lineWidth: 1)
            case "inner_drawers":
                for i in 0..<n {
                    let h = min(r.height / CGFloat(n + 1), 12)
                    let box = CGRect(x: r.minX + 4, y: r.maxY - 3 - h * CGFloat(i + 1), width: r.width - 8, height: h - 2)
                    ctx.stroke(Path(box), with: faint, lineWidth: 1)
                }
            default: break
            }
            // The front.
            switch cell.front {
            case "door_left": handle(r.maxX - 5, r.midY, vertical: true)
            case "door_right": handle(r.minX + 5, r.midY, vertical: true)
            case "door_pair":
                line(CGPoint(x: r.midX, y: r.minY), CGPoint(x: r.midX, y: r.maxY), .color(.primary.opacity(0.7)))
                handle(r.midX - 4, r.midY, vertical: true)
                handle(r.midX + 4, r.midY, vertical: true)
            case "drawers":
                let k = max(cell.drawers, 1)
                for i in 0..<k {
                    let y0 = r.minY + r.height * CGFloat(i) / CGFloat(k)
                    if i > 0 { line(CGPoint(x: r.minX, y: y0), CGPoint(x: r.maxX, y: y0), .color(.primary.opacity(0.7))) }
                    handle(r.midX, y0 + min(r.height / CGFloat(k) / 2, 8), vertical: false)
                }
            default: break
            }
        }
        .accessibilityHidden(true)
    }
}
