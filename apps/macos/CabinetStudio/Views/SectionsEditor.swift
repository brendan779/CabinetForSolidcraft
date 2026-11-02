import AppKit
import SwiftUI

/// The cabinet's front drawn to scale and split into sections, as in Blum's configurator: split it
/// up or across, click a section, then pick its front, what goes inside and how it opens.
/// A split being dragged: where it would land, and what that makes of the sections either side.
private struct SplitDrag {
    let column: Int?
    let line: Int
    let sizes: [Double]
    let gap: Double
    let origin: Double
    let snaps: [Double]
    var position = 0.0
    var snapped = false

    private var start: Double { origin - sizes[line] }

    mutating func move(to wanted: Double, tolerance: Double) {
        let room = sizes[line] + sizes[line + 1]
        let free = min(max(wanted, start + 60), start + max(room - 60, 60))
        let near = snaps.min { abs($0 - free) < abs($1 - free) }
        if let near, abs(near - free) <= tolerance {
            (position, snapped) = (near, true)
        } else {
            (position, snapped) = (free, false)
        }
    }

    var preview: [Double] {
        var out = sizes
        out[line] = position - start
        out[line + 1] = sizes[line] + sizes[line + 1] - out[line]
        return out
    }

    var label: String {
        let p = preview
        return "\(Int(p[line].rounded())) · \(Int(p[line + 1].rounded())) mm"
    }
}

struct SectionsEditor: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc
    @State private var dragging: SplitDrag?
    @AppStorage("sectionExactSizes") private var exactSizes = false

    @State private var column = 0
    @State private var row = 0

    /// Lift and bi-fold fronts are on hold (LIFT-HOLD). A section already using one still shows it,
    /// so an old job can be seen and changed.
    private func fronts(showing current: String) -> [(id: String, title: String)] {
        var list: [(id: String, title: String)] = [
            ("open", "Open"), ("door_left", "Door L"), ("door_right", "Door R"), ("door_pair", "Pair"),
            ("drawers", "Drawers"), ("sliding", "Sliding"), ("niche", "Niche"),
        ]
        if current == "lift" { list.append(("lift", "Lift")) }
        if current == "bi_fold" { list.append(("bi_fold", "Bi-fold")) }
        return list
    }

    private let systems: [(id: String, title: String)] = [
        ("legrabox", "LEGRABOX"), ("merivobox", "MERIVOBOX"), ("tandembox", "TANDEMBOX"),
        ("movento", "Wood, MOVENTO"), ("tandem", "Wood, TANDEM"),
    ]

    private let openings: [(id: String, title: String)] = [
        ("handles", "Handles"), ("tip_on", "TIP-ON"), ("servo_drive", "SERVO-DRIVE"),
    ]

    private var layouts: [(id: String, title: String)] {
        var l = [("door", "Door"), ("door_pair", "Pair"), ("drawers", "Drawers"),
                 ("sliding", "Sliding"), ("open", "Open")]
        if cab.kind == "tall" { l += [("oven", "Oven"), ("fridge", "Fridge")] }
        return l
    }

    var body: some View {
        let columns = cab.sectionColumns
        let (c, r) = clamped(columns)
        let cell = columns.isEmpty ? SectionCell() : columns[c].rows[r]
        VStack(alignment: .leading, spacing: 12) {
            layoutRow
            elevation(columns, selected: (c, r))
            HStack(spacing: 6) {
                Button { edit { $0.insert(SectionColumnDoc(rows: [SectionCell()]), at: c + 1) }; select(c + 1, 0) } label: {
                    Label("Split up", systemImage: "rectangle.split.2x1")
                }
                .help("Split with an upright divider: a new column of sections to the right of this one")
                Button { edit { $0[c].splitAcross(r) }; select(c, r + 1) } label: {
                    Label("Split across", systemImage: "rectangle.split.1x2")
                }
                .help("Split with a gap across: a new section above this one, with its own front")
                Spacer(minLength: 4)
                Button(role: .destructive) { remove(c, r, from: columns) } label: {
                    Label("Remove", systemImage: "minus.rectangle")
                }
                .labelStyle(.iconOnly)
                .disabled(columns.count <= 1 && (columns.first?.rows.count ?? 0) <= 1)
                .help("Remove this section: the ones beside it take its room")
            }
            .controlSize(.small)
            .buttonStyle(.bordered)
            .labelStyle(.titleAndIcon)

            Toggle("Exact sizes", isOn: $exactSizes.animation(.snappy))
                .help("Type a section's height and width in millimetres, as well as dragging its lines")
            if exactSizes, !columns.isEmpty {
                exactFields(columns, c, r)
            }

            if !columns.isEmpty, r + 1 < columns[c].rows.count {
                Toggle("Fixed shelf above", isOn: Binding(get: { cell.shelfAbove }, set: { v in
                    editCell(c, r) { $0.shelfAbove = v }
                }))
                .help("A fixed shelf between this section and the one above. Splitting the fronts does not need one.")
            }

            SectionTitle("Front")
            tiles(fronts(showing: cell.front), columns: 4, selected: cell.front, label: "Front") { id in
                SectionCellThumbnail(cell: SectionCell(front: id))
            } onPick: { id in
                editCell(c, r) {
                    $0.front = id
                    if id == "drawers" || id == "niche" { $0.inside = "empty" }
                    if !$0.opens { $0.opening = nil }
                }
            }
            if cell.front == "drawers" {
                DrawerStackPicker(selected: cell.drawerRatios) { stack in
                    editCell(c, r) {
                        $0.drawers = stack.count
                        $0.ratios = Set(stack).count > 1 ? stack : []
                    }
                }
                SectionTitle("Drawer system")
                optionTiles(systems, field: "drawer_system", selected: cell.system) { id in
                    editCell(c, r) { $0.system = id }
                }
                // Wooden boxes are sized by their runners and sides.
                if !["movento", "tandem"].contains(cell.system ?? project.option("drawer_system", of: cab)) {
                    boxChips(cell, c, r)
                }
            }
            if cell.front == "sliding" {
                ChipRow("Doors") {
                    ForEach(2...4, id: \.self) { n in
                        Button("\(n)") { editCell(c, r) { $0.slidingCount = n } }
                            .buttonStyle(ChipStyle(selected: cell.slidingCount == n))
                    }
                }
                ChipRow("Overlap") {
                    ForEach([20.0, 30, 40, 50], id: \.self) { o in
                        Button("\(Int(o))") { editCell(c, r) { $0.slidingOverlap = o } }
                            .buttonStyle(ChipStyle(selected: cell.slidingOverlap == o))
                    }
                }
                .help("How far each door passes behind the next when closed")
            }
            if cell.front == "lift" {
                LiftKindPicker(selected: cell.liftKind) { id in editCell(c, r) { $0.liftKind = id } }
            }
            if cell.opens {
                SectionTitle("Opening")
                optionTiles(openings, field: "opening", selected: cell.opening) { id in
                    editCell(c, r) { $0.opening = id }
                }
            }

            SectionTitle("Inside")
            tiles(cell.insideOptions(wardrobe: cab.kind == "wardrobe"), columns: 4, selected: cell.inside, label: "Inside") { id in
                SectionCellThumbnail(cell: SectionCell(inside: id))
            } onPick: { id in
                editCell(c, r) { $0.inside = id }
            }
            if ["shelves", "shoe_shelves", "inner_drawers"].contains(cell.inside) {
                ChipRow(cell.inside == "inner_drawers" ? "Drawers" : "Shelves") {
                    // Shoe shelves run to 8, for a 2400 high cabinet.
                    ForEach(1...(cell.inside == "shoe_shelves" ? 8 : 5), id: \.self) { n in
                        Button("\(n)") { editCell(c, r) { $0.insideCount = n } }
                            .buttonStyle(ChipStyle(selected: cell.insideCount == n))
                    }
                }
            }

            if cell.inside == "shoe_shelves" {
                ChipRow("Slope") {
                    ForEach([0.0, 5, 15], id: \.self) { a in
                        Button(a == 0 ? "Flat" : "\(Int(a))°") { editCell(c, r) { $0.shoeAngle = a } }
                            .buttonStyle(ChipStyle(selected: cell.shoeAngle == a))
                    }
                }
                if cell.shoeAngle > 0 {
                    ChipRow("Toe lip") {
                        ForEach([20.0, 30, 40, 50], id: \.self) { l in
                            Button("\(Int(l))") { editCell(c, r) { $0.shoeLip = l } }
                                .buttonStyle(ChipStyle(selected: cell.shoeLip == l))
                        }
                    }
                    .help("How far the lip stands above the shelf, to stop the toes sliding off")
                }
            }

            ChipRow(cell.front == "niche" ? "Clear height" : "Height") {
                ForEach([0.0, 150, 300, 450, 600, 900], id: \.self) { h in
                    Button(h == 0 ? "Auto" : "\(Int(h))") { editCell(c, r) { $0.height = h } }
                        .buttonStyle(ChipStyle(selected: abs(cell.height - h) < 0.5))
                }
            }
            .help("This section's height; Auto shares what the others leave")
            if columns.count > 1 {
                ChipRow("Width") {
                    ForEach([0.0, 300, 450, 600], id: \.self) { w in
                        Button(w == 0 ? "Auto" : "\(Int(w))") { edit { $0[c].width = w } }
                            .buttonStyle(ChipStyle(selected: abs(columns[c].width - w) < 0.5))
                    }
                }
                .help("This column's width; Auto shares what the others leave")
            }
        }
    }

    /// Whole-cabinet layouts to start from.
    private var layoutRow: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                ForEach(layouts, id: \.id) { layout in
                    Button(layout.title) {
                        edit { $0 = Sections.layout(layout.id) }
                        select(0, 0)
                    }
                    .buttonStyle(ChipStyle(selected: false))
                    .accessibilityLabel("Start from \(layout.title)")
                }
            }
        }
        .help("Replace the sections with a usual layout, to change from there")
    }

    private func tiles<Thumb: View>(
        _ items: [(id: String, title: String)],
        columns: Int,
        selected: String,
        label: String,
        @ViewBuilder thumbnail: @escaping (String) -> Thumb,
        onPick: @escaping (String) -> Void
    ) -> some View {
        LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: columns), spacing: 8) {
            ForEach(items, id: \.id) { item in
                Button { onPick(item.id) } label: {
                    VStack(spacing: 4) {
                        thumbnail(item.id).frame(width: 28, height: 34)
                        Text(item.title).font(.caption2).lineLimit(1).minimumScaleFactor(0.7)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
                }
                .buttonStyle(TileStyle(selected: selected == item.id))
                .accessibilityLabel("\(label): \(item.title)")
            }
        }
    }

    /// A choice this section can make for itself, or leave to the job's.
    /// A steel box's height and length, where the tallest and longest that fit won't do.
    private func boxChips(_ cell: SectionCell, _ c: Int, _ r: Int) -> some View {
        Group {
            ChipRow("Box height") {
                Button("Auto") { editCell(c, r) { $0.boxHeight = nil } }
                    .buttonStyle(ChipStyle(selected: cell.boxHeight == nil))
                ForEach(["n", "m", "k", "c", "f"], id: \.self) { h in
                    Button(h.uppercased()) { editCell(c, r) { $0.boxHeight = h } }
                        .buttonStyle(ChipStyle(selected: cell.boxHeight == h))
                }
            }
            .help("Auto takes the tallest box that fits")
            ChipRow("Length") {
                Button("Auto") { editCell(c, r) { $0.nominalLength = nil } }
                    .buttonStyle(ChipStyle(selected: cell.nominalLength == nil))
                ForEach([300.0, 400, 450, 500, 550, 600], id: \.self) { nl in
                    Button("\(Int(nl))") { editCell(c, r) { $0.nominalLength = nl } }
                        .buttonStyle(ChipStyle(selected: cell.nominalLength == nl))
                }
            }
            .help("Nominal length; Auto takes the longest that fits")
        }
    }

    private func optionTiles(
        _ items: [(id: String, title: String)],
        field: String,
        selected: String?,
        onPick: @escaping (String?) -> Void
    ) -> some View {
        // The tile that follows the whole job says what that means now.
        let chosen = project.option(field, of: cab)
        let current = items.first(where: { $0.id == chosen })?.title
        return LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 3), spacing: 8) {
            Button { onPick(nil) } label: {
                VStack(spacing: 4) {
                    Image(systemName: "arrow.uturn.backward.circle")
                        .font(.title2)
                        .frame(height: 40)
                    Text(current.map { "Job: \($0)" } ?? "Job's choice")
                        .font(.caption2).lineLimit(1).minimumScaleFactor(0.7)
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 6)
            }
            .buttonStyle(TileStyle(selected: selected == nil))
            .help("Use the whole job's choice\(current.map { " (now \($0))" } ?? ""), and follow it if it changes. The other tiles set this section on its own.")
            ForEach(items, id: \.id) { item in
                Button { onPick(item.id) } label: {
                    VStack(spacing: 4) {
                        ConstructionThumbnail(field: field, choice: item.id, highlighted: selected == item.id)
                            .frame(height: 40)
                        Text(item.title).font(.caption2).lineLimit(1).minimumScaleFactor(0.7)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
                }
                .buttonStyle(TileStyle(selected: selected == item.id))
                .accessibilityLabel(item.title)
            }
        }
    }

    /// The front elevation: columns left to right, rows bottom to top, each a button.
    private func elevation(_ columns: [SectionColumnDoc], selected: (Int, Int)) -> some View {
        let splits = sectionSplits(json: cab.json)
        let widthTotal = splits?.widthTotal ?? max(cab.width, 1)
        let heightTotal = splits?.heightTotal ?? max(cab.height, 1)
        let gap = splits?.gap ?? 0
        let rowGap = splits?.rowGap ?? 0
        let baseWidths = splits?.widths ?? Sections.shares(columns.map(\.width), of: widthTotal)
        let widths = dragging?.column == nil ? (dragging?.preview ?? baseWidths) : baseWidths
        let drag = dragging
        let heights: (Int) -> [Double] = { ci in
            let base = splits?.heights[ci] ?? Sections.shares(columns[ci].rows.map(\.height), of: heightTotal)
            return drag?.column == ci ? (drag?.preview ?? base) : base
        }
        let aspect = max(cab.width, 1) / max(cab.height, 1)
        return GeometryReader { geo in
            let scaleX = geo.size.width / widthTotal
            let scaleY = geo.size.height / heightTotal
            ZStack(alignment: .topLeading) {
                HStack(spacing: 3) {
                    ForEach(columns.indices, id: \.self) { ci in
                        let col = columns[ci]
                        let rowHeights = heights(ci)
                        VStack(spacing: 3) {
                            ForEach(col.rows.indices.reversed(), id: \.self) { ri in
                                let isSelected = selected == (ci, ri)
                                Button { select(ci, ri) } label: {
                                    SectionCellThumbnail(cell: col.rows[ri], selected: isSelected)
                                }
                                .buttonStyle(.plain)
                                .frame(height: max(rowHeights[ri] * scaleY - 3, 8))
                                .accessibilityLabel("Section \(ci + 1), row \(ri + 1)")
                                .accessibilityAddTraits(isSelected ? .isSelected : [])
                            }
                        }
                        .frame(width: max(widths[ci] * scaleX - 3, 8))
                    }
                }
                .frame(width: geo.size.width, height: geo.size.height, alignment: .topLeading)

                // The lines between sections, to drag.
                if splits != nil {
                    ForEach(0..<max(columns.count - 1, 0), id: \.self) { line in
                        splitHandle(column: nil, line: line, sizes: widths, gap: gap, others: [], scale: scaleX)
                            .frame(width: 10, height: geo.size.height)
                            .position(x: (linePosition(widths, line, gap) + gap / 2) * scaleX, y: geo.size.height / 2)
                    }
                    ForEach(columns.indices, id: \.self) { ci in
                        let rowHeights = heights(ci)
                        let left = widths[..<ci].reduce(0, +) + gap * Double(ci)
                        let others = columns.indices.filter { $0 != ci }.flatMap { other -> [Double] in
                            let h = heights(other)
                            return (0..<max(h.count - 1, 0)).map { linePosition(h, $0, rowGap) }
                        }
                        ForEach(0..<max(columns[ci].rows.count - 1, 0), id: \.self) { line in
                            splitHandle(column: ci, line: line, sizes: rowHeights, gap: rowGap, others: others, scale: scaleY)
                                .frame(width: max(widths[ci] * scaleX, 8), height: 10)
                                .position(
                                    x: (left + widths[ci] / 2) * scaleX,
                                    y: geo.size.height - (linePosition(rowHeights, line, rowGap) + rowGap / 2) * scaleY
                                )
                        }
                    }
                }
                if let d = dragging {
                    Text(d.label)
                        .font(.caption2.monospacedDigit())
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(d.snapped ? Color.accentColor : Color.black.opacity(0.75), in: Capsule())
                        .foregroundStyle(.white)
                        .position(x: geo.size.width / 2, y: 10)
                        .allowsHitTesting(false)
                }
            }
            .coordinateSpace(name: "elevation")
        }
        .aspectRatio(aspect, contentMode: .fit)
        .frame(maxWidth: .infinity, maxHeight: 260)
        .animation(dragging == nil ? .snappy : nil, value: columns)
    }

    private func linePosition(_ sizes: [Double], _ line: Int, _ gap: Double) -> Double {
        sizes[...line].reduce(0, +) + gap * Double(line)
    }

    /// A split's grab area: follows the drag, snapping where it means something, and moves the
    /// split only on release, so the cabinet rebuilds once.
    private func splitHandle(column: Int?, line: Int, sizes: [Double], gap: Double, others: [Double], scale: Double) -> some View {
        let dragged = dragging?.column == column && dragging?.line == line && dragging != nil
        return Rectangle()
            .fill(dragged ? (dragging?.snapped == true ? Color.accentColor : Color.white.opacity(0.8)) : Color.clear)
            .padding(column == nil ? .horizontal : .vertical, 4)
            .contentShape(Rectangle())
            .onHover { inside in
                if inside {
                    (column == nil ? NSCursor.resizeLeftRight : NSCursor.resizeUpDown).push()
                } else {
                    NSCursor.pop()
                }
            }
            .gesture(
                DragGesture(minimumDistance: 1, coordinateSpace: .named("elevation"))
                    .onChanged { value in
                        var d = dragging ?? SplitDrag(
                            column: column, line: line, sizes: sizes, gap: gap,
                            origin: linePosition(sizes, line, gap),
                            snaps: sectionSplitSnaps(sizes: sizes, line: UInt32(line), gap: gap, others: others)
                        )
                        let moved = column == nil ? value.translation.width : -value.translation.height
                        d.move(to: d.origin + moved / scale, tolerance: 6 / scale)
                        dragging = d
                    }
                    .onEnded { _ in
                        if let d = dragging { moveSplit(column: column, line: line, to: d.position) }
                        dragging = nil
                    }
            )
            .help(column == nil ? "Drag to move this divider" : "Drag to move this split")
    }

    private func moveSplit(column: Int?, line: Int, to position: Double) {
        withAnimation(.snappy) {
            var doc = cab
            doc.json = moveSectionSplit(json: cab.json, column: column.map(UInt32.init), line: UInt32(line), position: position)
            project.selected = doc
        }
    }

    /// A size typed for a section: the line above it moves, or below it in the top section, so
    /// the section beyond takes the difference.
    private func setSize(_ size: Double, of index: Int, in sizes: [Double], total: Double, gap: Double, column: Int?) {
        guard sizes.count > 1, size > 0 else { return }
        if index + 1 < sizes.count {
            moveSplit(column: column, line: index, to: linePosition(sizes, index, gap) - sizes[index] + size)
        } else {
            moveSplit(column: column, line: index - 1, to: total - size - gap)
        }
    }

    /// The boxes for typing sizes, shown only with Exact sizes on.
    @ViewBuilder private func exactFields(_ columns: [SectionColumnDoc], _ c: Int, _ r: Int) -> some View {
        if let s = sectionSplits(json: cab.json), c < s.heights.count, r < s.heights[c].count {
            HStack(spacing: 12) {
                if s.heights[c].count > 1 {
                    // The lowest section reads from the floor, as a cabinet's height does, so it
                    // takes the number of the base cabinet it lines up with. A niche's is its opening.
                    let fromFloor = r == 0 && s.floor > 0 && columns[c].rows[r].front != "niche"
                    let floor = fromFloor ? s.floor : 0
                    sizeField(fromFloor ? "From floor" : "Height", value: s.heights[c][r] + floor) {
                        setSize($0 - floor, of: r, in: s.heights[c], total: s.heightTotal, gap: s.rowGap, column: c)
                    }
                }
                if s.widths.count > 1 {
                    sizeField("Width", value: s.widths[c]) {
                        setSize($0, of: c, in: s.widths, total: s.widthTotal, gap: s.gap, column: nil)
                    }
                }
                if s.heights[c].count <= 1, s.widths.count <= 1 {
                    Text("Split this section to set its sizes").font(.caption).foregroundStyle(.secondary)
                }
            }
        }
    }

    private func sizeField(_ title: String, value: Double, onSet: @escaping (Double) -> Void) -> some View {
        HStack(spacing: 4) {
            Text(title).font(.caption).foregroundStyle(.secondary)
            TextField(title, value: Binding(get: { (value * 10).rounded() / 10 }, set: onSet), format: .number)
                .textFieldStyle(.roundedBorder)
                .labelsHidden()
                .frame(width: 64)
            Text("mm").font(.caption).foregroundStyle(.secondary)
        }
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
            let solid = !["open", "niche"].contains(cell.front)
            ctx.fill(frame, with: .color(selected ? Color.accentColor.opacity(0.15) : .primary.opacity(solid ? 0.10 : 0.03)))
            ctx.stroke(frame, with: .color(selected ? Color.accentColor : .primary.opacity(0.7)), lineWidth: selected ? 2 : 1.1)
            let faint = GraphicsContext.Shading.color(.primary.opacity(0.35))
            let edge = GraphicsContext.Shading.color(.primary.opacity(0.7))
            let accent = GraphicsContext.Shading.color(.accentColor)
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
                    line(CGPoint(x: r.minX + 3, y: y), CGPoint(x: r.maxX - 3, y: y), faint, cell.inside == "shoe_shelves" ? 2 : 1)
                }
            case "hanging", "wardrobe":
                let y = r.minY + min(8, r.height * 0.2)
                if cell.inside == "wardrobe" {
                    line(CGPoint(x: r.minX + 3, y: y - 3), CGPoint(x: r.maxX - 3, y: y - 3), faint)
                }
                line(CGPoint(x: r.minX + 3, y: y + 1), CGPoint(x: r.maxX - 3, y: y + 1), faint, 2)
                var hanger = Path()
                hanger.move(to: CGPoint(x: r.midX, y: y + 1))
                hanger.addLine(to: CGPoint(x: r.midX - r.width * 0.25, y: y + 1 + r.height * 0.18))
                hanger.addLine(to: CGPoint(x: r.midX + r.width * 0.25, y: y + 1 + r.height * 0.18))
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
                line(CGPoint(x: r.midX, y: r.minY), CGPoint(x: r.midX, y: r.maxY), edge)
                handle(r.midX - 4, r.midY, vertical: true)
                handle(r.midX + 4, r.midY, vertical: true)
            case "sliding":
                // Overlapping doors, with the track along the top.
                line(CGPoint(x: r.minX, y: r.minY + 3), CGPoint(x: r.maxX, y: r.minY + 3), accent, 2)
                let n = max(cell.slidingCount, 2)
                for i in 1..<n {
                    let x = r.minX + r.width * CGFloat(i) / CGFloat(n)
                    line(CGPoint(x: x - 2, y: r.minY + 3), CGPoint(x: x - 2, y: r.maxY), edge)
                    line(CGPoint(x: x + 2, y: r.minY + 3), CGPoint(x: x + 2, y: r.maxY), faint)
                }
            case "drawers":
                let ratios = cell.drawerRatios
                let total = ratios.reduce(0, +)
                var y = r.minY
                for (i, ratio) in ratios.enumerated() {
                    let h = r.height * ratio / max(total, 0.001)
                    if i > 0 { line(CGPoint(x: r.minX, y: y), CGPoint(x: r.maxX, y: y), edge) }
                    handle(r.midX, y + min(h / 2, 8), vertical: false)
                    y += h
                }
            case "lift":
                var arc = Path()
                arc.addArc(center: CGPoint(x: r.midX, y: r.minY + 2), radius: r.height * 0.6,
                           startAngle: .degrees(100), endAngle: .degrees(160), clockwise: false)
                ctx.stroke(arc, with: accent, lineWidth: 1.2)
                handle(r.midX, r.maxY - 5, vertical: false)
            case "bi_fold":
                line(CGPoint(x: r.minX, y: r.midY), CGPoint(x: r.maxX, y: r.midY), edge)
                line(CGPoint(x: r.midX, y: r.midY - 3), CGPoint(x: r.midX, y: r.minY + 4), accent, 1.2)
                handle(r.midX, r.maxY - 5, vertical: false)
            case "niche":
                // An appliance's dark glass, in the clear opening.
                ctx.fill(Path(r.insetBy(dx: 4, dy: 4)), with: .color(.primary.opacity(0.4)))
            default: break
            }
        }
        .accessibilityHidden(true)
    }
}
