import SwiftUI

struct InspectorView: View {
    @Environment(ProjectModel.self) private var project

    var body: some View {
        if let cab = project.selected, cab.isPiece {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    header(cab)
                    PieceSection(cab: cab)
                }
                .padding(16)
            }
        } else if let cab = project.selected {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    header(cab)
                    SizeSection(cab: cab)
                    if cab.kind == "blind_corner" {
                        BlindSection(cab: cab)
                    }
                    if cab.kind == "sink" {
                        SinkSection(cab: cab)
                    }
                    if cab.kind == "corner_base" {
                        CornerSection(cab: cab)
                    } else {
                        FrontsSection(cab: cab)
                    }
                    BoardsSection(cab: cab)
                    ConstructionScope(cab: cab)
                    let overrides = project.overrides(of: cab)
                    ForEach(optionGroups(), id: \.field) { group in
                        VisualPicker(group: group, selected: project.option(group.field, of: cab),
                                     overridden: overrides.contains(group.field),
                                     onReset: { project.resetToProject([group.field]) }) { choice in
                            withAnimation(.snappy) { project.setOption(group.field, choice) }
                        }
                    }
                    if let b = project.build(cab), !b.hardware.isEmpty {
                        HardwareSection(items: b.hardware)
                    }
                }
                .padding(16)
            }
        } else {
            ContentUnavailableView("No cabinet selected", systemImage: "cube.transparent",
                                   description: Text("Pick a cabinet in the sidebar or the viewport."))
        }
    }

    private func header(_ cab: CabinetDoc) -> some View {
        HStack(spacing: 12) {
            CabinetGlyph(doc: cab).frame(width: 34, height: 40)
            VStack(alignment: .leading) {
                Text(cab.id).font(.title2.weight(.semibold)).monospaced()
                Text(CabinetDoc.kindTitle(cab.kind)).foregroundStyle(.secondary)
            }
            Spacer()
            if let err = project.error(for: cab) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(.orange)
                    .help(err)
            }
        }
    }
}

/// A filler's width or an end panel's reach, and the scribe left on to fit the wall.
struct PieceSection: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    let cab: CabinetDoc

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            if cab.kind == "filler" {
                SectionTitle("Filler")
                chips("Width", value: cab.width, presets: [25, 38, 50, 75, 100, 150]) { c, v in c.width = v }
            } else {
                SectionTitle("End panel")
                HStack(spacing: 8) {
                    ForEach([true, false], id: \.self) { floor in
                        Button {
                            withAnimation(.snappy) { var c = cab; c.toFloor = floor; project.selected = c }
                        } label: {
                            VStack(spacing: 6) {
                                EndPanelThumbnail(toFloor: floor).frame(width: 44, height: 52)
                                Text(floor ? "To the floor" : "On the kick").font(.caption)
                            }
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 8)
                        }
                        .buttonStyle(TileStyle(selected: cab.toFloor == floor))
                        .help(floor ? "The panel runs down over the toe kick to the floor."
                                    : "The panel stops on top of the toe kick, like the doors.")
                    }
                }
            }
            chips("Scribe", value: cab.scribe, presets: [0, 10, 20, 30, 40]) { c, v in c.scribe = v }
            Text("The scribe is extra board, cut off on site to follow the wall. It is not part of the run's length.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private func chips(_ title: String, value: Double, presets: [Double],
                       set: @escaping (inout CabinetDoc, Double) -> Void) -> some View {
        HStack(spacing: 6) {
            Text(title).foregroundStyle(.secondary)
            Spacer()
            ForEach(presets, id: \.self) { p in
                Button(unit.format(p, suffix: false)) {
                    withAnimation(.snappy) { var c = cab; set(&c, p); project.selected = c }
                }
                .buttonStyle(ChipStyle(selected: abs(p - value) < 0.5))
                .accessibilityLabel("\(title) \(unit.format(p))")
            }
        }
    }
}

/// A cabinet side seen from the side with an end panel on it, to the floor or stopping at the kick.
struct EndPanelThumbnail: View {
    let toFloor: Bool

    var body: some View {
        Canvas { ctx, size in
            let w = size.width, h = size.height, kick = h * 0.16
            let carcass = CGRect(x: w * 0.15, y: 0, width: w * 0.7, height: h - kick)
            ctx.stroke(Path(carcass), with: .color(.primary.opacity(0.4)), lineWidth: 1)
            var notch = Path()
            notch.addRect(CGRect(x: w * 0.25, y: h - kick, width: w * 0.6, height: kick))
            ctx.stroke(notch, with: .color(.primary.opacity(0.25)), lineWidth: 1)
            let panel = CGRect(x: w * 0.08, y: 0, width: w * 0.84, height: toFloor ? h : h - kick)
            ctx.fill(Path(panel), with: .color(.accentColor.opacity(0.3)))
            ctx.stroke(Path(panel), with: .color(.accentColor), lineWidth: 1.4)
        }
        .accessibilityHidden(true)
    }
}

/// Whether construction choices change the whole job or only this cabinet, drawn as a run of
/// cabinets with all of them or just one lit.
struct ConstructionScope: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    var body: some View {
        @Bindable var project = project
        let overrides = project.overrides(of: cab)
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                SectionTitle("Construction")
                Spacer()
                if !overrides.isEmpty {
                    Button("Match job", systemImage: "arrow.uturn.backward") {
                        project.resetToProject(Array(overrides))
                    }
                    .buttonStyle(.borderless)
                    .font(.caption)
                    .help("\(cab.id) differs from the job in \(overrides.count) choices. Click to follow the job's standard again.")
                }
            }
            HStack(spacing: 8) {
                ForEach([false, true], id: \.self) { only in
                    Button {
                        withAnimation(.snappy) { project.onlyThisCabinet = only }
                    } label: {
                        VStack(spacing: 6) {
                            RunThumbnail(lit: only ? .one : .all).frame(width: 64, height: 30)
                            Text(only ? "Only \(cab.id)" : "Whole job").font(.caption)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                    }
                    .buttonStyle(TileStyle(selected: project.onlyThisCabinet == only))
                    .help(only ? "Choices below change only \(cab.id); the rest of the job keeps the job's standard."
                               : "Choices below change every cabinet in the job, except where a cabinet has its own choice.")
                    .accessibilityLabel(only ? "Change only \(cab.id)" : "Change the whole job")
                }
            }
            StandardPresets()
        }
    }
}

/// Saved standards as tiles: each drawn with its back and top joint. Click one to make it the
/// job's standard; the last tile saves the job's standard under a name.
struct StandardPresets: View {
    @Environment(ProjectModel.self) private var project
    @State private var library = StandardLibrary.shared
    @State private var naming = false
    @State private var name = ""

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(library.standards) { preset in
                    let current = project.jobStandardIs(preset.standardJson)
                    Button {
                        project.applyJobStandard(preset.standardJson)
                        project.status = "Job standard: \(preset.name)"
                    } label: {
                        VStack(spacing: 4) {
                            HStack(spacing: 2) {
                                ConstructionThumbnail(field: "top_joint", choice: preset.choice("top_joint"), highlighted: current)
                                ConstructionThumbnail(field: "back", choice: preset.choice("back"), highlighted: current)
                            }
                            .frame(width: 76, height: 30)
                            Text(preset.name).font(.caption).lineLimit(1)
                        }
                        .frame(width: 90)
                        .padding(.vertical, 6)
                    }
                    .buttonStyle(TileStyle(selected: current))
                    .help("Use \(preset.name) for this job")
                    .contextMenu {
                        Button("Update from This Job") { try? library.save(preset.name, standardJson: project.standardJson) }
                        Button("Delete", role: .destructive) { library.delete(preset.name) }
                    }
                }
                Button {
                    name = ""
                    naming = true
                } label: {
                    VStack(spacing: 4) {
                        Image(systemName: "plus").font(.title3).frame(height: 30)
                        Text("Save…").font(.caption)
                    }
                    .frame(width: 70)
                    .padding(.vertical, 6)
                }
                .buttonStyle(TileStyle(selected: false))
                .help("Save this job's construction as a named standard, to use on other jobs")
            }
        }
        .alert("Save the job's construction as", isPresented: $naming) {
            TextField("My shop", text: $name)
            Button("Save") {
                let n = name.trimmingCharacters(in: .whitespaces)
                guard !n.isEmpty else { return }
                do {
                    try library.save(n, standardJson: project.standardJson)
                    project.status = "Saved standard \(n)"
                } catch {
                    project.lastError = "Could not save the standard: \(error.localizedDescription)"
                }
            }
            Button("Cancel", role: .cancel) {}
        }
    }
}

/// Four cabinets in a row, all or only the second one highlighted.
struct RunThumbnail: View {
    enum Lit { case all, one }
    let lit: Lit

    var body: some View {
        Canvas { ctx, size in
            let n = 4, gap: CGFloat = 3
            let w = (size.width - gap * CGFloat(n - 1)) / CGFloat(n)
            for i in 0..<n {
                let r = CGRect(x: CGFloat(i) * (w + gap), y: 0, width: w, height: size.height)
                let on = lit == .all || i == 1
                ctx.fill(Path(roundedRect: r, cornerRadius: 2), with: .color(on ? .accentColor.opacity(0.35) : .primary.opacity(0.06)))
                ctx.stroke(Path(roundedRect: r, cornerRadius: 2), with: .color(on ? .accentColor : .primary.opacity(0.3)), lineWidth: 1)
            }
        }
        .accessibilityHidden(true)
    }
}

/// Size presets as chips, with a fine-tune stepper behind them. Most cabinets are a standard size,
/// so the common case is one click.
struct SizeSection: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    let cab: CabinetDoc

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            SectionTitle("Size")
            dimension("Width", \.width, presets: [300, 400, 450, 500, 600, 800, 900, 1000, 1200])
            dimension("Height", \.height, presets: heightPresets)
            dimension("Depth", \.depth, presets: [300, 330, 350, 450, 560, 580, 600])
            HStack {
                Text("Shelves").foregroundStyle(.secondary)
                Spacer()
                Stepper("\(cab.shelves)", value: Binding(get: { cab.shelves }, set: { v in
                    var c = cab; c.shelves = v; project.selected = c
                }), in: 0...8)
                .monospacedDigit()
            }
            HStack(spacing: 6) {
                Text("Seen ends").foregroundStyle(.secondary)
                Spacer()
                ForEach(["left", "right"], id: \.self) { side in
                    let on = cab.exposed(side)
                    Button {
                        withAnimation(.snappy) { var c = cab; c.setExposed(side, !on); project.selected = c }
                    } label: {
                        Image(systemName: side == "left" ? "rectangle.lefthalf.inset.filled" : "rectangle.righthalf.inset.filled")
                    }
                    .buttonStyle(ChipStyle(selected: on))
                    .help(on ? "The \(side) side is seen from the room: no screws through it (biscuits, dowels and glue)."
                             : "The \(side) side is hidden: screws through it are fine. Click if it will be seen.")
                    .accessibilityLabel("\(side.capitalized) side \(on ? "seen" : "hidden")")
                }
            }
            if cab.shelves > 0 {
                HStack(spacing: 6) {
                    Text("Fixed").foregroundStyle(.secondary)
                    Spacer()
                    ForEach(1...cab.shelves, id: \.self) { i in
                        let fixed = cab.fixedShelves.contains(i)
                        Button {
                            withAnimation(.snappy) {
                                var c = cab
                                c.fixedShelves = fixed ? c.fixedShelves.filter { $0 != i } : c.fixedShelves + [i]
                                project.selected = c
                            }
                        } label: {
                            Image(systemName: fixed ? "lock.fill" : "\(i).circle")
                        }
                        .buttonStyle(ChipStyle(selected: fixed))
                        .help(fixed ? "Shelf \(i) is fixed: biscuited and screwed between the sides. Click to put it on pins."
                                    : "Shelf \(i) sits on shelf pins. Click to fix it between the sides.")
                        .accessibilityLabel("Shelf \(i) \(fixed ? "fixed" : "on pins")")
                    }
                }
            }
        }
    }

    private var heightPresets: [Double] {
        switch cab.kind {
        case "wall", "open_shelf": [360, 450, 600, 720, 900]
        case "tall": [1800, 2100, 2250, 2400]
        default: [720, 820, 870, 900]
        }
    }

    // A key path rather than a setter closure: Swift 6.3 crashes converting a @MainActor setter
    // closure for Binding.
    private func dimension(_ title: String, _ size: WritableKeyPath<CabinetDoc, Double> & Sendable, presets: [Double]) -> some View {
        let value = cab[keyPath: size]
        let set = { (v: Double) in
            var c = cab; c[keyPath: size] = v; project.selected = c
        }
        return VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(title).foregroundStyle(.secondary)
                Spacer()
                Stepper(value: Binding(get: { value }, set: { v in
                    var c = cab; c[keyPath: size] = v; project.selected = c
                }), in: 100...3000, step: 1) {
                    Text(unit.format(value)).monospacedDigit().fontWeight(.medium)
                }
            }
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(presets, id: \.self) { p in
                        Button(unit.format(p, suffix: false)) { withAnimation(.snappy) { set(p) } }
                            .buttonStyle(ChipStyle(selected: abs(p - value) < 0.5))
                            .accessibilityLabel("\(title) \(unit.format(p))")
                    }
                }
            }
        }
    }
}

/// A sink cabinet's options, each a tile with a drawing.
struct SinkSection: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    private let options: [(key: String, title: String, symbol: String, help: String)] = [
        ("back_cutout", "Pipe cut-out", "rectangle.bottomhalf.inset.filled",
         "An opening low in the back for the waste and water pipes"),
        ("liner", "Base liner", "drop.triangle",
         "A waterproof liner on the bottom panel, in the hardware list"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("Sink")
            HStack(spacing: 8) {
                ForEach(options, id: \.key) { o in
                    let on = cab.sinkOption(o.key)
                    Button {
                        withAnimation(.snappy) { var c = cab; c.setSinkOption(o.key, !on); project.selected = c }
                    } label: {
                        VStack(spacing: 6) {
                            Image(systemName: o.symbol).font(.title2).frame(height: 30)
                            Text(o.title).font(.caption)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                    }
                    .buttonStyle(TileStyle(selected: on))
                    .help(o.help)
                    .accessibilityValue(on ? "on" : "off")
                }
            }
        }
    }
}

/// A blind corner's blind side and widths, drawn as the front seen from the room.
struct BlindSection: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("Blind corner")
            HStack(spacing: 8) {
                ForEach(["left", "right"], id: \.self) { side in
                    Button {
                        withAnimation(.snappy) { var c = cab; c.blindSide = side; project.selected = c }
                    } label: {
                        VStack(spacing: 6) {
                            BlindThumbnail(side: side, blind: cab.blindWidth / max(cab.width, 1))
                                .frame(width: 64, height: 44)
                            Text(side == "left" ? "Blind left" : "Blind right").font(.caption)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                    }
                    .buttonStyle(TileStyle(selected: cab.blindSide == side))
                    .help("The other run meets this cabinet on its \(side); the door hinges on the other side.")
                }
            }
            chips("Blind", value: cab.blindWidth, presets: [580, 600, 630, 650, 700]) { c, v in c.blindWidth = v }
            chips("Filler", value: cab.blindFiller, presets: [25, 50, 75, 100]) { c, v in c.blindFiller = v }
        }
    }

    private func chips(_ title: String, value: Double, presets: [Double],
                       set: @escaping (inout CabinetDoc, Double) -> Void) -> some View {
        HStack(spacing: 6) {
            Text(title).foregroundStyle(.secondary)
            Spacer()
            ForEach(presets, id: \.self) { p in
                Button("\(Int(p))") {
                    withAnimation(.snappy) { var c = cab; set(&c, p); project.selected = c }
                }
                .buttonStyle(ChipStyle(selected: abs(p - value) < 0.5))
                .accessibilityLabel("\(title) \(Int(p)) millimetres")
            }
        }
    }
}

struct CornerSection: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("L-corner")
            HStack(spacing: 8) {
                ForEach(["left", "right"], id: \.self) { side in
                    Button {
                        withAnimation(.snappy) { var c = cab; c.cornerSide = side; project.selected = c }
                    } label: {
                        VStack(spacing: 6) {
                            CornerThumbnail(side: side)
                                .frame(width: 56, height: 56)
                            Text(side == "left" ? "Corner left" : "Corner right").font(.caption)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                    }
                    .buttonStyle(TileStyle(selected: cab.cornerSide == side))
                    .accessibilityLabel(side == "left" ? "Second leg along the left wall" : "Second leg along the right wall")
                    .help("The second leg runs along the \(side) wall; bi-fold doors close both fronts.")
                }
            }
            HStack(spacing: 6) {
                Text("Second leg").foregroundStyle(.secondary)
                Spacer()
                ForEach([800.0, 900, 1000, 1100, 1200], id: \.self) { p in
                    Button("\(Int(p))") {
                        withAnimation(.snappy) { var c = cab; c.cornerWidth = p; project.selected = c }
                    }
                    .buttonStyle(ChipStyle(selected: abs(p - cab.cornerWidth) < 0.5))
                    .accessibilityLabel("Second leg \(Int(p)) millimetres")
                }
            }
        }
    }
}

/// An L-corner from above: both legs against their walls, the bi-fold pair across the inside
/// corner in the accent colour.
struct CornerThumbnail: View {
    let side: String

    var body: some View {
        Canvas { ctx, size in
            let s = min(size.width, size.height)
            let leg = s * 0.42
            let flip = side == "right"
            func x(_ v: CGFloat) -> CGFloat { flip ? s - v : v }
            var body = Path()
            body.move(to: CGPoint(x: x(0), y: 0))
            body.addLine(to: CGPoint(x: x(s), y: 0))
            body.addLine(to: CGPoint(x: x(s), y: leg))
            body.addLine(to: CGPoint(x: x(leg), y: leg))
            body.addLine(to: CGPoint(x: x(leg), y: s))
            body.addLine(to: CGPoint(x: x(0), y: s))
            body.closeSubpath()
            ctx.fill(body, with: .color(.primary.opacity(0.10)))
            ctx.stroke(body, with: .color(.primary.opacity(0.6)), lineWidth: 1)
            var doors = Path()
            doors.move(to: CGPoint(x: x(s - 2), y: leg + 2))
            doors.addLine(to: CGPoint(x: x(leg + 2), y: leg + 2))
            doors.addLine(to: CGPoint(x: x(leg + 2), y: s - 2))
            ctx.stroke(doors, with: .color(.accentColor), lineWidth: 2)
        }
        .accessibilityHidden(true)
    }
}

/// The front of a blind corner: blind panel, filler and door, the blind side shaded.
struct BlindThumbnail: View {
    let side: String
    let blind: Double

    var body: some View {
        Canvas { ctx, size in
            let w = size.width, h = size.height
            let bw = w * min(max(blind, 0.2), 0.8)
            let filler = w * 0.06
            let left = side == "left"
            let panel = CGRect(x: left ? 0 : w - bw + filler, y: 0, width: bw - filler, height: h)
            let strip = CGRect(x: left ? bw - filler : w - bw, y: 0, width: filler, height: h)
            let door = CGRect(x: left ? bw + 2 : 0, y: 0, width: w - bw - 2, height: h)
            ctx.fill(Path(panel), with: .color(.primary.opacity(0.12)))
            ctx.stroke(Path(panel), with: .color(.primary.opacity(0.5)), lineWidth: 1)
            ctx.fill(Path(strip), with: .color(.primary.opacity(0.3)))
            ctx.fill(Path(door), with: .color(.accentColor.opacity(0.3)))
            ctx.stroke(Path(door), with: .color(.accentColor), lineWidth: 1.4)
            // Hinge side marks on the door's far edge.
            let hx = left ? door.maxX - 3 : door.minX + 3
            for y in [h * 0.2, h * 0.8] {
                ctx.fill(Path(ellipseIn: CGRect(x: hx - 2, y: y - 2, width: 4, height: 4)), with: .color(.accentColor))
            }
        }
        .accessibilityHidden(true)
    }
}

struct FrontsSection: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    private let layouts: [(id: String, title: String)] = [
        ("door", "Door"), ("door_pair", "Pair"), ("drawers", "Drawers"), ("lift", "Lift"), ("bi_fold_lift", "Bi-fold"), ("open", "Open"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("Fronts")
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 3), spacing: 8) {
                ForEach(layouts, id: \.id) { layout in
                    let selected = cab.frontType == layout.id
                    Button {
                        withAnimation(.snappy) { var c = cab; c.frontType = layout.id; project.selected = c }
                    } label: {
                        VStack(spacing: 6) {
                            FrontThumbnail(layout: layout.id, hinge: cab.hingeSide, ratios: [1, 1, 1])
                                .frame(width: 44, height: 52)
                            Text(layout.title).font(.caption)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                    }
                    .buttonStyle(TileStyle(selected: selected))
                }
            }
            if cab.frontType == "door" {
                Picker("Hinge", selection: Binding(get: { cab.hingeSide }, set: { v in
                    var c = cab; c.hingeSide = v; project.selected = c
                })) {
                    Label("Left", systemImage: "arrow.left.to.line").tag("left")
                    Label("Right", systemImage: "arrow.right.to.line").tag("right")
                }
                .pickerStyle(.segmented)
            }
            if cab.frontType == "drawers" {
                DrawerStackPicker(cab: cab)
            }
            if cab.frontType == "lift" {
                LiftKindPicker(cab: cab)
            }
        }
    }
}

/// How a lifting front moves, drawn from the side: each picks its AVENTOS family.
struct LiftKindPicker: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    private let kinds: [(id: String, title: String, help: String)] = [
        ("stay_lift", "Stay lift", "Swings up about its top edge and stays open (AVENTOS HK top)"),
        ("up_and_over", "Up and over", "Rises and tips over the top, for large fronts (AVENTOS HS top)"),
        ("lift_up", "Lift up", "Rises parallel to itself, for small fronts (AVENTOS HL top)"),
    ]

    var body: some View {
        HStack(spacing: 8) {
            ForEach(kinds, id: \.id) { kind in
                Button {
                    withAnimation(.snappy) { var c = cab; c.liftKind = kind.id; project.selected = c }
                } label: {
                    VStack(spacing: 6) {
                        LiftKindThumbnail(kind: kind.id).frame(width: 52, height: 44)
                        Text(kind.title).font(.caption).lineLimit(1).minimumScaleFactor(0.8)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                }
                .buttonStyle(TileStyle(selected: cab.liftKind == kind.id))
                .help(kind.help)
                .accessibilityLabel("Lift: \(kind.title)")
                .accessibilityHint(kind.help)
            }
        }
    }
}

/// A wall cabinet from the side with its front closed (faint) and open (accent).
struct LiftKindThumbnail: View {
    let kind: String

    var body: some View {
        Canvas { ctx, size in
            let w = size.width, h = size.height
            let body = CGRect(x: w * 0.35, y: h * 0.35, width: w * 0.55, height: h * 0.6)
            ctx.stroke(Path(body), with: .color(.primary.opacity(0.6)), lineWidth: 1)
            var closed = Path()
            closed.move(to: CGPoint(x: body.minX - 2, y: body.minY))
            closed.addLine(to: CGPoint(x: body.minX - 2, y: body.maxY))
            ctx.stroke(closed, with: .color(.primary.opacity(0.3)), lineWidth: 2)
            var open = Path()
            switch kind {
            case "up_and_over":
                open.move(to: CGPoint(x: body.minX + 4, y: body.minY - 3))
                open.addLine(to: CGPoint(x: body.maxX - 2, y: body.minY - 3))
            case "lift_up":
                open.move(to: CGPoint(x: body.minX - 2, y: 2))
                open.addLine(to: CGPoint(x: body.minX - 2, y: body.minY - 2))
            default:
                open.move(to: CGPoint(x: body.minX - 2, y: body.minY))
                open.addLine(to: CGPoint(x: 2, y: body.minY - h * 0.25))
            }
            ctx.stroke(open, with: .color(.accentColor), lineWidth: 2.2)
        }
        .accessibilityHidden(true)
    }
}

/// Drawer stacks as pictures: pick a stack instead of typing ratios.
struct DrawerStackPicker: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    private let stacks: [[Double]] = [[1, 1], [1, 1, 1], [1, 1, 1, 1], [1, 2], [1, 1, 2], [1, 2, 3], [1, 1, 1, 2]]

    var body: some View {
        LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 4), spacing: 8) {
            ForEach(stacks.indices, id: \.self) { i in
                let stack = stacks[i]
                Button {
                    withAnimation(.snappy) { var c = cab; c.drawerRatios = stack; project.selected = c }
                } label: {
                    FrontThumbnail(layout: "drawers", hinge: "left", ratios: stack)
                        .frame(width: 34, height: 42)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 6)
                }
                .buttonStyle(TileStyle(selected: cab.drawerRatios == stack))
                .accessibilityLabel("\(stack.count) drawers, heights \(stack.map { String(Int($0)) }.joined(separator: " to "))")
            }
        }
    }
}

struct HardwareSection: View {
    let items: [String]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionTitle("Hardware")
            let counts = Dictionary(grouping: items, by: { $0 }).mapValues(\.count)
            ForEach(counts.keys.sorted(), id: \.self) { item in
                HStack(alignment: .firstTextBaseline) {
                    Text("\(counts[item]!)×").monospacedDigit().foregroundStyle(.secondary)
                    Text(item).font(.callout)
                }
            }
        }
    }
}

struct SectionTitle: View {
    let text: String
    init(_ text: String) { self.text = text }

    var body: some View {
        Text(text.uppercased())
            .font(.caption.weight(.semibold))
            .tracking(0.8)
            .foregroundStyle(.secondary)
    }
}

struct ChipStyle: ButtonStyle {
    let selected: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .accessibilityAddTraits(selected ? .isSelected : [])
            .font(.caption.monospacedDigit())
            .padding(.horizontal, 9)
            .padding(.vertical, 4)
            .background(
                Capsule().fill(selected ? Color.accentColor : Color.primary.opacity(configuration.isPressed ? 0.16 : 0.07))
            )
            .foregroundStyle(selected ? Color.white : Color.primary)
    }
}

struct TileStyle: ButtonStyle {
    let selected: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .accessibilityAddTraits(selected ? .isSelected : [])
            .background(
                RoundedRectangle(cornerRadius: 10)
                    .fill(Color.primary.opacity(configuration.isPressed ? 0.12 : 0.04))
            )
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(selected ? Color.accentColor : Color.primary.opacity(0.12), lineWidth: selected ? 2 : 1)
            )
            .contentShape(RoundedRectangle(cornerRadius: 10))
            .scaleEffect(configuration.isPressed ? 0.97 : 1)
            .animation(.snappy(duration: 0.15), value: configuration.isPressed)
    }
}
