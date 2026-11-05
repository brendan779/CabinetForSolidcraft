import SwiftUI

/// The inspector's levels of detail.
enum InspectorTab: String, CaseIterable, Identifiable {
    case cabinet = "Cabinet"
    case fronts = "Fronts"
    case fittings = "Fittings"
    case build = "Build"
    case benchtop = "Benchtop"

    var id: String { rawValue }

    /// The core's option groups under this tab. A new group falls under Build until it is given
    /// a home.
    func holds(_ field: String) -> Bool {
        let fronts: Set = [
            "opening", "handle_range", "door_style", "handle_size", "handle_finish", "handle_place",
            "sharknose_angle", "sharknose_door_edge", "drawer_frame", "front_grain", "overlay",
        ]
        let cabinet: Set = ["toe_kick", "leg_fixing", "wall_hanging"]
        let fittings: Set = ["drawer_system", "drawer_sides", "insert_design", "hinge_boss"]
        switch self {
        case .cabinet: return cabinet.contains(field)
        case .fronts: return fronts.contains(field)
        case .fittings: return fittings.contains(field)
        case .build: return !cabinet.contains(field) && !fronts.contains(field) && !fittings.contains(field)
        case .benchtop: return false
        }
    }
}

struct InspectorTabBar: View {
    @Binding var tab: InspectorTab

    var body: some View {
        HStack(spacing: 0) {
            ForEach(InspectorTab.allCases) { t in
                Button {
                    withAnimation(.snappy) { tab = t }
                } label: {
                    Text(t.rawValue)
                        .font(.caption.weight(t == tab ? .semibold : .regular))
                        .lineLimit(1)
                        .minimumScaleFactor(0.8)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 10)
                        .foregroundStyle(t == tab ? Theme.text : Theme.textDim)
                        .overlay(alignment: .bottom) {
                            Rectangle().fill(t == tab ? Theme.accent : .clear).frame(height: 2)
                        }
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(t == tab ? .isSelected : [])
            }
        }
        .padding(.horizontal, 6)
        .background(Theme.strip)
    }
}

/// What a choice changes, always in view: the whole job, or only this cabinet.
struct ScopeSwitch: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    var body: some View {
        HStack(spacing: 2) {
            ForEach([false, true], id: \.self) { only in
                Button(only ? "Only \(cab.id)" : "Whole job") {
                    withAnimation(.snappy) { project.onlyThisCabinet = only }
                }
                .buttonStyle(.plain)
                .font(.caption2.weight(project.onlyThisCabinet == only ? .semibold : .regular))
                .lineLimit(1)
                .padding(.horizontal, 7)
                .padding(.vertical, 4)
                .background(project.onlyThisCabinet == only ? Theme.control : .clear, in: .rect(cornerRadius: 4))
                .foregroundStyle(project.onlyThisCabinet == only ? Theme.text : Theme.textDim)
                .help(only ? "Choices change only \(cab.id); the rest of the job keeps the job's standard."
                           : "Choices change every cabinet in the job, except where a cabinet has its own choice.")
                .accessibilityLabel(only ? "Change only \(cab.id)" : "Change the whole job")
                .accessibilityAddTraits(project.onlyThisCabinet == only ? .isSelected : [])
            }
        }
        .padding(2)
        .background(Theme.strip, in: .rect(cornerRadius: 6))
        .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(Theme.hairline))
        .fixedSize()
    }
}

/// How high a wall cabinet hangs.
struct HangSection: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    let cab: CabinetDoc

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("On the wall")
            ChipRow("Height off the floor") {
                Button("Usual") { update { $0.hangHeight = 0 } }
                    .buttonStyle(ChipStyle(selected: cab.hangHeight == 0))
                    .help("Above the tallest floor cabinet, a 30 mm top and a 600 mm splashback")
                ForEach([1350.0, 1400.0, 1450.0, 1500.0, 1600.0], id: \.self) { z in
                    Button(unit.format(z, suffix: false)) { update { $0.hangHeight = z } }
                        .buttonStyle(ChipStyle(selected: abs(cab.hangHeight - z) < 0.5))
                }
            }
        }
    }

    private func update(_ change: (inout CabinetDoc) -> Void) {
        var c = cab
        change(&c)
        withAnimation(.snappy) { project.selected = c }
    }
}

/// The job's benchtop, wherever the selection is: there is one for the job, not one per cabinet.
struct BenchtopTab: View {
    @Environment(ProjectModel.self) private var project

    var body: some View {
        if let top = project.cabinets.first(where: { $0.kind == "benchtop" }) {
            PieceSection(cab: top)
        } else {
            VStack(alignment: .leading, spacing: 10) {
                SectionTitle("Benchtop")
                Text("This job has no benchtop yet.")
                    .foregroundStyle(Theme.textDim)
                Button("Add a benchtop") { project.addPiece("benchtop") }
                    .buttonStyle(ChipStyle(selected: false))
            }
        }
    }
}

struct InspectorView: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage("inspectorTab") private var tab: InspectorTab = .cabinet

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
            VStack(spacing: 0) {
                HStack(alignment: .center) {
                    header(cab)
                    Spacer(minLength: 8)
                    ScopeSwitch(cab: cab)
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 10)
                InspectorTabBar(tab: $tab)
                ScrollView {
                    VStack(alignment: .leading, spacing: 22) {
                        switch tab {
                        case .cabinet: cabinetTab(cab)
                        case .fronts: frontsTab(cab)
                        case .fittings: fittingsTab(cab)
                        case .build: buildTab(cab)
                        case .benchtop: BenchtopTab()
                        }
                    }
                    .padding(14)
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        } else {
            ContentUnavailableView("No cabinet selected", systemImage: "cube.transparent",
                                   description: Text("Pick a cabinet in the sidebar or the viewport."))
        }
    }

    @ViewBuilder private func cabinetTab(_ cab: CabinetDoc) -> some View {
        SizeSection(cab: cab)
        if cab.isUpper { HangSection(cab: cab) }
        if cab.kind == "blind_corner" { BlindSection(cab: cab) }
        if cab.kind == "sink" { SinkSection(cab: cab) }
        if cab.kind == "wardrobe" { WardrobeSection(cab: cab) }
        if cab.kind == "corner_base" || cab.kind == "corner_wall" {
            CornerSection(cab: cab)
        } else if cab.kind == "rangehood" {
            InsertSection(cab: cab)
        }
        options(cab, in: .cabinet)
        CladdingSection(cab: cab)
    }

    @ViewBuilder private func frontsTab(_ cab: CabinetDoc) -> some View {
        if cab.kind != "corner_base", cab.kind != "corner_wall", cab.kind != "rangehood" {
            FrontsSection(cab: cab)
        }
        options(cab, in: .fronts)
    }

    @ViewBuilder private func fittingsTab(_ cab: CabinetDoc) -> some View {
        options(cab, in: .fittings)
        // Lighting is on hold (LIGHT-HOLD): a job saved with strips can still take them off.
        if !cab.lights.isEmpty {
            Button("Remove this cabinet's lights") {
                withAnimation(.snappy) { var c = cab; c.lights = []; project.selected = c }
            }
        }
        if let b = project.build(cab), !b.hardware.isEmpty {
            HardwareSection(items: b.hardware)
        }
    }

    @ViewBuilder private func buildTab(_ cab: CabinetDoc) -> some View {
        ConstructionScope(cab: cab)
        BoardsSection(cab: cab)
        options(cab, in: .build)
    }

    /// The core's pickers for this tab: only those that mean something for the choices made (the
    /// core says which).
    @ViewBuilder private func options(_ cab: CabinetDoc, in tab: InspectorTab) -> some View {
        let overrides = project.overrides(of: cab)
        let groups = optionGroups().filter {
            tab.holds($0.field) && optionApplies(cabinetJson: cab.json, field: $0.field)
                // A job-wide choice, but only wall cabinets hang.
                && ($0.field != "wall_hanging" || cab.kind.contains("wall"))
                // Nothing that hangs stands on a base.
                && (!["toe_kick", "leg_fixing"].contains($0.field) || !cab.isUpper)
        }
        ForEach(groups, id: \.field) { group in
            VisualPicker(group: group, selected: project.option(group.field, of: cab),
                         overridden: overrides.contains(group.field),
                         onReset: { project.resetToProject([group.field]) }) { choice in
                withAnimation(.snappy) { project.setOption(group.field, choice) }
            }
            StandardSizes(after: group.field, cab: cab)
            // ABI-3: the style picker sits under the range switch once ABI is chosen, replacing
            // the generic size and finish tiles the core hides.
            if group.field == "handle_range",
               project.option("handle_range", of: cab) == "abi",
               ["corner", "centre"].contains(project.option("handle_place", of: cab)) {
                AbiHandleSection(cab: cab)
            }
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
                    .foregroundStyle(Theme.warning)
                    .help(err)
            }
        }
    }
}

/// A title and a menu: on one line where they fit, the menu under the title where its choice's
/// name is long.
struct MenuRow<Content: View>: View {
    let title: String
    let content: Content

    init(_ title: String, @ViewBuilder content: () -> Content) {
        self.title = title
        self.content = content()
    }

    var body: some View {
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 6) {
                Text(title).foregroundStyle(.secondary).lineLimit(1).fixedSize()
                Spacer(minLength: 4)
                content
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(title).foregroundStyle(.secondary).lineLimit(1)
                content
            }
        }
    }
}

/// A title and its row of chips: on one line where they fit, the chips under the title where the
/// inspector is narrow (full screen), so neither the title nor a chip's number wraps.
struct ChipRow<Chips: View>: View {
    let title: String
    let reset: (() -> Void)?
    let chips: Chips

    init(_ title: String, reset: (() -> Void)? = nil, @ViewBuilder chips: () -> Chips) {
        self.title = title
        self.reset = reset
        self.chips = chips()
    }

    var body: some View {
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 6) {
                label
                Spacer(minLength: 4)
                HStack(spacing: 6) { chips }.fixedSize()
            }
            VStack(alignment: .leading, spacing: 4) {
                label
                HStack(spacing: 6) { chips }.fixedSize()
            }
        }
    }

    private var label: some View {
        HStack(spacing: 4) {
            Text(title).foregroundStyle(.secondary).lineLimit(1)
            if let reset {
                Button(action: reset) { Image(systemName: "arrow.uturn.backward.circle") }
                    .buttonStyle(.borderless)
                    .help("Only this cabinet differs from the job; click to follow the job again")
            }
        }
    }
}

/// The sizes that go with a construction choice, as rows of usual values: the plinth under Base,
/// the frame under a shaker or glass door style.
struct StandardSizes: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    let after: String
    let cab: CabinetDoc

    var body: some View {
        switch after {
        case "toe_kick" where project.option("toe_kick", of: cab) != "none":
            rows([("Plinth height", "toe_kick_height", [100, 125, 150, 180, 200]),
                  ("Plinth setback", "toe_kick_setback", [25, 50, 75, 100])])
        case "overlay" where project.option("overlay", of: cab) != "inset":
            // Blum's configurator's gaps: between fronts (half at each side), top and bottom.
            rows([("Front gap", "front_gap", [2, 3, 4]),
                  ("Top gap", "top_gap", [0, 2, 3]),
                  ("Bottom gap", "bottom_gap", [0, 2])])
        case "handle_place" where project.option("handle_place", of: cab) == "edge_pull":
            rows([("Clearance over pull", "edge_pull_clearance", [2, 3, 5])])
        case "handle_place" where project.option("handle_place", of: cab) == "finger_pull":
            rows([("Notch height", "finger_pull_height", [15, 20, 25, 30, 40]),
                  ("Notch depth", "finger_pull_depth", [6, 8, 10, 12]),
                  ("Finger gap", "finger_pull_gap", [0, 5, 10, 15, 20])])
        case "sharknose_angle":
            rows([("Finger gap", "sharknose_gap", [10, 15, 20, 25, 30]),
                  ("Rail thickness", "drawer_frame_thickness", [16, 18]),
                  ("Rail behind front", "drawer_frame_overlap", [0, 10, 15, 25, 40])])
        case "drawer_frame" where project.option("drawer_frame", of: cab) == "board"
            && project.option("handle_place", of: cab) != "sharknose":
            rows([("Thickness", "drawer_frame_thickness", [16, 18]),
                  ("Rail behind front", "drawer_frame_overlap", [0, 10, 15, 25, 40])])
        case "door_style" where ["shaker", "glass_frame"].contains(project.option("door_style", of: cab)):
            rows([("Frame", "shaker_frame_width", [50, 60, 70, 80, 90]),
                  ("Panel", "shaker_panel_thickness", [6, 9, 12]),
                  ("Groove", "shaker_groove_depth", [8, 10, 12]),
                  ("Slab below", "shaker_slab_below", [150, 200, 250, 300])])
        case "wardrobe":
            // Each row only where the wardrobe has the part it sizes.
            rows([("Hat shelf drop", "wardrobe_hat_shelf_drop", [250, 300, 350, 400]),
                  ("Rail drop", "wardrobe_rail_drop", [50, 60, 70, 80])]
                 + (cab.hanging == "double" ? [("Lower shelf", "wardrobe_double_hang_shelf", [950, 1050, 1150])] : [])
                 + (cab.innerDrawers > 0 ? [("Drawer height", "wardrobe_drawer_pitch", [150, 200, 250])] : []))
        default:
            EmptyView()
        }
    }

    private func rows(_ items: [(String, String, [Double])]) -> some View {
        let overrides = project.overrides(of: cab)
        return VStack(alignment: .leading, spacing: 8) {
            ForEach(items, id: \.1) { title, field, presets in
                let value = project.standardNumber(field, of: cab)
                ChipRow(title, reset: overrides.contains(field) ? { project.resetToProject([field]) } : nil) {
                    ForEach(presets, id: \.self) { p in
                        Button(unit.format(p, suffix: false)) {
                            withAnimation(.snappy) { project.setStandardNumber(field, p) }
                        }
                        .buttonStyle(ChipStyle(selected: abs(p - value) < 0.5))
                        .accessibilityLabel("\(title) \(unit.format(p))")
                    }
                }
            }
        }
        .padding(.top, -6)
    }
}

/// A filler's width or an end panel's reach, and the scribe left on to fit the wall.
struct PieceSection: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    let cab: CabinetDoc

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            if cab.kind == "splashback" {
                SplashbackSection(cab: cab)
            } else if cab.kind == "gap" {
                SectionTitle("Gap")
                HStack(spacing: 6) {
                    ForEach([true, false], id: \.self) { upper in
                        Button(upper ? "Wall cabinets" : "Floor cabinets") {
                            withAnimation(.snappy) { var c = cab; c.gapIsUpper = upper; project.selected = c }
                        }
                        .buttonStyle(ChipStyle(selected: cab.gapIsUpper == upper))
                    }
                }
                chips("Width", value: cab.width, presets: [150, 300, 450, 600, 900, 1200]) { c, v in c.width = v }
                wallField("Exact width", cab.width) { $0.width = $1 }
                Text("Empty space between cabinets in that line, for a window, a rangehood or an appliance. It builds nothing and is not on the cut list. Move it in the sidebar. A benchtop runs on over a gap among the floor cabinets.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            } else if cab.kind == "walls" {
                SectionTitle("Walls at the run's ends")
                HStack(spacing: 8) {
                    ForEach(WallPlan.choices, id: \.title) { choice in
                        Button {
                            withAnimation(.snappy) {
                                var c = cab
                                c.wallLeft = choice.left
                                c.wallRight = choice.right
                                project.selected = c
                            }
                        } label: {
                            VStack(spacing: 6) {
                                Canvas { ctx, size in WallPlan.draw(ctx, size, left: choice.left, right: choice.right) }
                                    .frame(width: 44, height: 40)
                                    .accessibilityHidden(true)
                                Text(choice.title).font(.caption)
                            }
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 8)
                        }
                        .buttonStyle(TileStyle(selected: cab.wallLeft == choice.left && cab.wallRight == choice.right))
                        .help(choice.help)
                    }
                }
                wallField(cab.wallLeft && cab.wallRight ? "Between the walls" : "Back wall run", cab.wallLength) { $0.wallLength = $1 }
                wallField("Wall height", cab.wallHeight) { $0.wallHeight = $1 }
                if cab.wallLeft { wallField("Left wall comes out", cab.wallLeftRun) { $0.wallLeftRun = $1 } }
                if cab.wallRight { wallField("Right wall comes out", cab.wallRightRun) { $0.wallRightRun = $1 } }
                if !cab.wallLeft && !cab.wallRight {
                    wallField("Run starts along the wall", cab.wallOffset) { $0.wallOffset = $1 }
                }
                Text("Walls stay as measured, and the cabinets stand on the back wall. A return wall hides the side of a cabinet that reaches it, stops the benchtop and any cornice there, and needs a filler so fronts can open.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                ForEach(project.build(cab)?.warnings ?? [], id: \.self) { note in
                    Label(note, systemImage: "exclamationmark.triangle")
                        .font(.caption)
                        .foregroundStyle(Theme.warning)
                        .fixedSize(horizontal: false, vertical: true)
                }
            } else if ["cornice", "light_rail"].contains(cab.kind) {
                let cornice = cab.kind == "cornice"
                SectionTitle(cornice ? "Cornice" : "Light rail")
                chips("Height", value: cab.mouldingHeight, presets: [40, 60, 80, 100]) { c, v in c.mouldingHeight = v }
                HStack(spacing: 6) {
                    Text("Ends").foregroundStyle(.secondary)
                    Spacer()
                    ForEach([true, false], id: \.self) { on in
                        Button(on ? "Returned" : "Square") {
                            withAnimation(.snappy) { var c = cab; c.returns = on; project.selected = c }
                        }
                        .buttonStyle(ChipStyle(selected: cab.returns == on))
                        .help(on ? "A return piece back to the wall at each open end" : "The face stops square at the ends, its end edges taped")
                    }
                }
                Text(cornice
                     ? "A board in the plane of the door faces on top of each stretch of wall, tall, wardrobe and rangehood cabinets with the same top, in the front board."
                     : "A board in the plane of the door faces under each stretch of wall cabinets, hiding their lights, in the front board.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            } else if cab.kind == "floating_shelf" {
                SectionTitle("Floating shelf")
                chips("Width", value: cab.width, presets: [450, 600, 900, 1200]) { c, v in c.width = v }
                chips("Depth", value: cab.shelfDepth, presets: [200, 250, 300]) { c, v in c.shelfDepth = v }
                chips("Thick", value: cab.topThickness, presets: [30, 40, 50]) { c, v in c.topThickness = v }
                chips("Height", value: cab.shelfHeight, presets: [1300, 1400, 1500, 1600]) { c, v in c.shelfHeight = v }
                // Lighting is on hold (LIGHT-HOLD).
                if cab.shelfLight > 0 {
                    Button("Remove this shelf's light") {
                        withAnimation(.snappy) { var c = cab; c.shelfLight = 0; project.selected = c }
                    }
                }
                ShelfBoardButton(cab: cab)
                Text("A shelf on concealed brackets in the run, against the wall; its LED strip is 24 V.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            } else if cab.kind == "benchtop" {
                SectionTitle("Benchtop")
                chips("Stone", value: cab.topThickness, presets: project.boardsById[cab.stoneProductID].map { [$0.thickness] } ?? [12, 20, 30, 40]) { c, v in
                    c.topThickness = v
                    if c.edgeProfile == "mitred_apron", c.apronDepth < v { c.apronDepth = v }
                }
                chips("Front", value: cab.frontOverhang, presets: [0, 10, 20, 30, 40]) { c, v in c.frontOverhang = v }
                chips("Ends", value: cab.sideOverhang, presets: [0, 10, 20, 40]) { c, v in c.sideOverhang = v }
                chips("Splashback", value: cab.splashback, presets: [0, 100, 300, 600, 650]) { c, v in c.splashback = v }
                chips("Seating", value: cab.backOverhang, presets: [0, 300, 400, 600]) { c, v in c.backOverhang = v }
                chips("Corners", value: cab.endRadius, presets: [0, 25, 100, 300]) { c, v in c.endRadius = v }
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 3), spacing: 8) {
                    ForEach([("square", "Square"), ("pencil_round", "Pencil"), ("bullnose", "Bullnose"),
                             ("bevel", "Bevel"), ("mitred_apron", "Built-up")], id: \.0) { id, title in
                        Button {
                            withAnimation(.snappy) { var c = cab; c.edgeProfile = id; project.selected = c }
                        } label: {
                            VStack(spacing: 6) {
                                EdgeProfileThumbnail(profile: id).frame(width: 44, height: 30)
                                Text(title).font(.caption)
                            }
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 6)
                        }
                        .buttonStyle(TileStyle(selected: cab.edgeProfile == id))
                        .help(id == "mitred_apron" ? "Mitred stone returns over cabinet-board supports, with an adjustable shadow line"
                                                   : "The front edge's profile, for the benchtop maker")
                        .accessibilityLabel("Edge: \(title)")
                    }
                }
                if cab.edgeProfile == "mitred_apron" {
                    BenchtopBuildUpControls(cab: cab)
                }
                HStack(spacing: 6) {
                    Text("Waterfall").foregroundStyle(.secondary)
                    Spacer()
                    ForEach(["left", "right"], id: \.self) { side in
                        let on = cab.waterfall(side)
                        Button {
                            withAnimation(.snappy) { var c = cab; c.setWaterfall(side, !on); project.selected = c }
                        } label: {
                            Label(side.capitalized, systemImage: side == "left" ? "arrow.down.left" : "arrow.down.right")
                        }
                        .buttonStyle(ChipStyle(selected: on))
                        .help("The top turns down to the floor at the \(side) end of each stretch, mitred at the corner")
                    }
                }
                Text("One piece over each stretch of base cabinets in the run, past the door faces by the front overhang and past each end of a stretch by the end overhang.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                MaterialSection(cab: cab)
                SectionTitle("Cut-outs")
                let bases = project.cabinets.filter {
                    ["base", "drawer_bank", "sink", "blind_corner", "corner_base"].contains($0.kind)
                }
                ForEach(bases, id: \.id) { base in
                    HStack(spacing: 6) {
                        Text(base.id).monospaced()
                        Spacer()
                        let kind = cab.cutout(over: base.id)
                        Menu {
                            Button("None") {
                                withAnimation(.snappy) { var c = cab; c.setCutout(over: base.id, template: nil); project.selected = c }
                            }
                            Divider()
                            let library = project.appliances
                            ForEach(library.indices, id: \.self) { i in
                                let t = library[i]
                                Button {
                                    withAnimation(.snappy) { var c = cab; c.setCutout(over: base.id, template: t); project.selected = c }
                                } label: {
                                    Label(t["name"] as? String ?? "Appliance",
                                          systemImage: t["kind"] as? String == "sink" ? "drop" : "flame")
                                }
                            }
                        } label: {
                            Label(cab.cutoutName(over: base.id) ?? "None",
                                  systemImage: kind == nil ? "circle.dashed" : kind == "sink" ? "drop" : "flame")
                        }
                        .menuStyle(.borderlessButton)
                        .fixedSize()
                        .help("The sink or cooktop cut-out centred over \(base.id), from the job's appliances")
                    }
                }
                AppliancesEditor()
                if cab.splashback > 0 && cab.backOverhang == 0 {
                    SectionTitle("Splashback")
                    ForEach(bases, id: \.id) { base in
                        HStack(spacing: 6) {
                            Text(base.id).monospaced()
                            Spacer()
                            ForEach([("niches", "Niche", "square.split.1x2"), ("power_points", "Power", "powerplug")],
                                    id: \.0) { field, title, symbol in
                                let on = cab.hasOpening(field, over: base.id)
                                Button {
                                    withAnimation(.snappy) {
                                        var c = cab
                                        c.setOpening(field, over: base.id, !on)
                                        project.selected = c
                                    }
                                } label: {
                                    Label(title, systemImage: symbol)
                                }
                                .buttonStyle(ChipStyle(selected: on))
                                .help(field == "niches"
                                      ? "An open niche in the splashback over \(base.id): lined in the benchtop's board, one shelf"
                                      : "A cut-out for a power point in the splashback over \(base.id), sized for the plate below")
                            }
                        }
                    }
                    let plates = powerPointPlates()
                    if cab.hasAnyPowerPoint, !plates.isEmpty {
                        let current = cab.powerPointPlate ?? plates[0].id
                        MenuRow("Plate") {
                            Menu {
                                ForEach(plates, id: \.id) { plate in
                                    Button {
                                        withAnimation(.snappy) { var c = cab; c.setPowerPointPlate(plate.id); project.selected = c }
                                    } label: {
                                        Text(plate.label)
                                        Text(plate.description)
                                    }
                                }
                            } label: {
                                Label(plates.first { $0.id == current }?.label ?? current, systemImage: "powerplug")
                            }
                            .menuStyle(.borderlessButton)
                            .fixedSize()
                            .help(plates.first { $0.id == current }?.description ?? "")
                        }
                    }
                }
            } else if cab.kind == "filler" {
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
            if !["benchtop", "floating_shelf", "cornice", "light_rail", "walls", "gap", "splashback"].contains(cab.kind) {
                chips("Scribe", value: cab.scribe, presets: [0, 10, 20, 30, 40]) { c, v in c.scribe = v }
                Text("The scribe is extra board, cut off on site to follow the wall. It is not part of the run's length.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    private func shelfProfileTile(id: String, title: String, help: String, surface: Bool, width: Double, depth: Double) -> some View {
        Button {
            withAnimation(.snappy) { var c = cab; c.shelfProfile = id; project.selected = c }
        } label: {
            VStack(spacing: 4) {
                ProfileThumbnail(surface: surface, width: width, depth: depth).frame(height: 34)
                Text(title).font(.caption2).lineLimit(1)
            }
            .padding(5)
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(TileStyle(selected: cab.shelfProfile == id))
        .help(help)
        .accessibilityLabel(id.isEmpty ? title : "Profile \(title)")
    }

    private func wallField(_ title: String, _ value: Double,
                           set: @escaping (inout CabinetDoc, Double) -> Void) -> some View {
        NumberField(title: title, value: value) { v in
            var c = cab
            set(&c, v)
            project.selected = c
        }
        .id(title)
    }

    private func chips(_ title: String, value: Double, presets: [Double],
                       set: @escaping (inout CabinetDoc, Double) -> Void) -> some View {
        ChipRow(title) {
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

/// A floating shelf's board, from the same picker as a cabinet's.
struct ShelfBoardButton: View {
    @Environment(ProjectModel.self) private var project
    @State private var picking = false
    let cab: CabinetDoc

    var body: some View {
        let board = project.boardsById[cab.shelfBoard]
        Button { picking = true } label: {
            HStack(spacing: 10) {
                RoundedRectangle(cornerRadius: 5)
                    .fill(board.map { Color(nsColor: $0.displayColour) } ?? Color.primary.opacity(0.08))
                    .overlay {
                        if board?.grained == true { GrainPattern().clipShape(RoundedRectangle(cornerRadius: 5)) }
                    }
                    .overlay(RoundedRectangle(cornerRadius: 5).strokeBorder(.primary.opacity(0.2)))
                    .frame(width: 34, height: 24)
                VStack(alignment: .leading, spacing: 1) {
                    Text("Material").font(.caption).foregroundStyle(.secondary)
                    Text(board.map { "\($0.decor) \($0.finish), \(Int($0.thickness)) mm" } ?? "Generic board").lineLimit(1)
                }
                Spacer()
                Image(systemName: "chevron.right").foregroundStyle(.tertiary)
            }
            .padding(6)
            .contentShape(Rectangle())
        }
        .buttonStyle(TileStyle(selected: picking))
        .popover(isPresented: $picking, arrowEdge: .leading) {
            BoardPicker(boards: project.boards, marked: [cab.shelfBoard], columns: 140, onPick: { b in
                var c = cab
                c.shelfBoard = b.id
                withAnimation(.snappy) { project.selected = c }
                picking = false
            }) { _ in EmptyView() }
                .frame(width: 640, height: 520)
        }
    }
}

/// A splashback's finish, each choice a picture of itself: tile sizes, the way they are laid, the
/// grout and the colours. Tile or sheet, the field fills the wall from the top to the wall cabinets.
struct SplashbackSection: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    private static let sizes: [(w: Double, h: Double, name: String)] = [
        (300, 75, "Subway"), (300, 100, "Subway"), (200, 100, "Brick"), (600, 300, "Large"),
        (100, 100, "Square"), (200, 200, "Square"), (300, 300, "Square"), (50, 50, "Mosaic"),
    ]
    private static let tileColours = ["#F2EFE8", "#FFFFFF", "#D9D4C8", "#8A8F8C", "#3B3F42", "#5E7466", "#2F4858", "#B98D6B"]
    private static let groutColours = ["#FFFFFF", "#D9D4C8", "#B9B4AA", "#8A8780", "#55524D", "#2A2826"]

    private func set(_ change: (inout CabinetDoc) -> Void) {
        var c = cab
        change(&c)
        withAnimation(.snappy) { project.selected = c }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            SectionTitle("Splashback")
            HStack(spacing: 8) {
                kindTile("Tile", selected: cab.tileWidth > 0) { $0.tileWidth = 300; $0.tileHeight = 75 }
                kindTile("One sheet", selected: cab.tileWidth <= 0) { $0.tileWidth = 0; $0.tileHeight = 0 }
            }
            if cab.tileWidth > 0 {
                SectionTitle("Tile size")
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 6), count: 4), spacing: 6) {
                    ForEach(Self.sizes.indices, id: \.self) { i in
                        let size = Self.sizes[i]
                        Button {
                            set { $0.tileWidth = size.w; $0.tileHeight = size.h }
                        } label: {
                            VStack(spacing: 4) {
                                TileThumbnail(width: size.w, height: size.h, grout: 3, brick: false,
                                              tile: cab.tileColour, joint: cab.groutColour)
                                    .frame(height: 34)
                                Text("\(Int(size.w))×\(Int(size.h))").font(.caption2).monospacedDigit()
                            }
                            .padding(5)
                        }
                        .buttonStyle(TileStyle(selected: cab.tileWidth == size.w && cab.tileHeight == size.h))
                        .help("\(size.name), \(Int(size.w)) × \(Int(size.h)) mm")
                    }
                }
                NumberField(title: "Width", value: cab.tileWidth) { v in set { $0.tileWidth = max(v, 10) } }
                    .id("tile width")
                NumberField(title: "Height", value: cab.tileHeight) { v in set { $0.tileHeight = max(v, 10) } }
                    .id("tile height")
                SectionTitle("Laid")
                HStack(spacing: 8) {
                    ForEach([("stack", "Stacked"), ("brick", "Brick bond")], id: \.0) { id, title in
                        Button {
                            set { $0.tilePattern = id }
                        } label: {
                            VStack(spacing: 4) {
                                TileThumbnail(width: 120, height: 45, grout: 3, brick: id == "brick",
                                              tile: cab.tileColour, joint: cab.groutColour)
                                    .frame(height: 40)
                                Text(title).font(.caption)
                            }
                            .padding(6)
                            .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(TileStyle(selected: cab.tilePattern == id))
                    }
                }
                ChipRow("Grout") {
                    ForEach([0.5, 1, 1.5, 2, 3, 5], id: \.self) { g in
                        Button(String(format: "%g", g)) { set { $0.grout = g } }
                            .buttonStyle(ChipStyle(selected: cab.grout == g))
                            .accessibilityLabel("Grout \(g) millimetres")
                    }
                }
                swatches("Grout colour", Self.groutColours, chosen: cab.groutColour) { $0.groutColour = $1 }
            }
            swatches(cab.tileWidth > 0 ? "Tile colour" : "Colour", Self.tileColours, chosen: cab.tileColour) { $0.tileColour = $1 }
            ChipRow("Thickness") {
                ForEach([6.0, 10, 12, 20], id: \.self) { t in
                    Button(String(format: "%g", t)) { set { $0.setNumber("thickness", t) } }
                        .buttonStyle(ChipStyle(selected: cab.number("thickness") == t))
                        .accessibilityLabel("Thickness \(Int(t)) millimetres")
                }
            }
            Text("Goes on the wall from the benchtop up to the wall cabinets, over the base cabinets. Whole rows start at the benchtop and the ends are cut evenly. The tiles, with 10 % spare, are on the hardware list.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private func kindTile(_ title: String, selected: Bool, _ change: @escaping (inout CabinetDoc) -> Void) -> some View {
        Button { set(change) } label: {
            Text(title).frame(maxWidth: .infinity).padding(.vertical, 6)
        }
        .buttonStyle(TileStyle(selected: selected))
    }

    private func swatches(_ title: String, _ colours: [String], chosen: String,
                          set change: @escaping (inout CabinetDoc, String) -> Void) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).foregroundStyle(.secondary)
            HStack(spacing: 6) {
                ForEach(colours, id: \.self) { hex in
                    Button {
                        set { change(&$0, hex) }
                    } label: {
                        RoundedRectangle(cornerRadius: 5)
                            .fill(Color(nsColor: NSColor(hex: hex) ?? .gray))
                            .frame(width: 26, height: 26)
                            .overlay(RoundedRectangle(cornerRadius: 5)
                                .strokeBorder(chosen == hex ? Theme.accent : Theme.border, lineWidth: chosen == hex ? 2 : 1))
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("\(title) \(hex)")
                }
            }
        }
    }
}

/// A few tiles of a size, to scale with each other, with their joints.
struct TileThumbnail: View {
    let width: Double
    let height: Double
    let grout: Double
    let brick: Bool
    let tile: String
    let joint: String

    var body: some View {
        Canvas { ctx, size in
            ctx.fill(Path(CGRect(origin: .zero, size: size)), with: .color(Color(nsColor: NSColor(hex: joint) ?? .gray)))
            // 600 mm of wall fits across, so sizes compare at a glance.
            let k = size.width / 600
            let (w, h, g) = (width * k, height * k, max(grout * k * 2, 1))
            let fill = Color(nsColor: NSColor(hex: tile) ?? .white)
            var y = 0.0, row = 0
            while y < size.height {
                var x = brick && row % 2 == 1 ? -(w + g) / 2 : 0
                while x < size.width {
                    ctx.fill(Path(CGRect(x: x, y: y, width: w, height: h)), with: .color(fill))
                    x += w + g
                }
                y += h + g
                row += 1
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: 3))
        .accessibilityHidden(true)
    }
}

/// A plan of the run from above: the back wall, a return wall at either end, the cabinets.
enum WallPlan {
    struct Choice {
        let title: String
        let left: Bool
        let right: Bool
        let help: String
    }

    static let choices = [
        Choice(title: "Back only", left: false, right: false, help: "Only the back wall: both ends of the run are open."),
        Choice(title: "Left", left: true, right: false, help: "A return wall at the left end: that cabinet abuts walls on its back and its side."),
        Choice(title: "Right", left: false, right: true, help: "A return wall at the right end: that cabinet abuts walls on its back and its side."),
        Choice(title: "Both", left: true, right: true, help: "Between two walls: the run has to fit the gap."),
    ]

    static func draw(_ ctx: GraphicsContext, _ size: CGSize, left: Bool, right: Bool) {
        let w = size.width, h = size.height, t = max(2, h * 0.08)
        let inset = w * 0.12
        let run = CGRect(x: inset, y: t, width: w - 2 * inset, height: h * 0.45)
        ctx.fill(Path(run), with: .color(.primary.opacity(0.12)))
        ctx.stroke(Path(run), with: .color(.primary.opacity(0.5)), lineWidth: 1)
        var walls = Path()
        walls.addRect(CGRect(x: left ? inset - t : 0, y: 0,
                             width: w - (left ? inset - t : 0) - (right ? inset - t : 0), height: t))
        if left { walls.addRect(CGRect(x: inset - t, y: 0, width: t, height: h)) }
        if right { walls.addRect(CGRect(x: w - inset, y: 0, width: t, height: h)) }
        ctx.fill(walls, with: .color(.accentColor))
    }
}

/// A length typed in, for what no preset can guess: a measured room.
struct NumberField: View {
    let title: String
    let value: Double
    let set: (Double) -> Void
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    @State private var text = ""
    @FocusState private var focused: Bool

    var body: some View {
        HStack(spacing: 6) {
            Text(title).foregroundStyle(.secondary)
            Spacer()
            TextField(title, text: $text)
                .labelsHidden()
                .multilineTextAlignment(.trailing)
                .frame(width: 80)
                .focused($focused)
                .onAppear { text = value > 0 ? String(Int(value)) : "" }
                .onChange(of: value) { if !focused { text = value > 0 ? String(Int(value)) : "" } }
                .onSubmit { commit() }
                .onChange(of: focused) { if !focused { commit() } }
            Text("mm").foregroundStyle(.secondary)
        }
    }

    private func commit() {
        let v = Double(text.trimmingCharacters(in: .whitespaces)) ?? 0
        if abs(v - value) > 0.01 { set(max(v, 0)) }
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
            // A sectioned cabinet sets its shelves per section.
            if cab.frontType != "sections" {
                HStack {
                    Text("Shelves").foregroundStyle(.secondary)
                    Spacer()
                    Stepper("\(cab.shelves)", value: Binding(get: { cab.shelves }, set: { v in
                        var c = cab; c.shelves = v; project.selected = c
                    }), in: 0...8)
                    .monospacedDigit()
                }
            }
            // A wardrobe's drawers belong to its interior; everywhere else, to a section.
            if cab.kind == "wardrobe" {
                HStack {
                    Text("Inner drawers").foregroundStyle(.secondary)
                    Spacer()
                    Stepper("\(cab.innerDrawers)", value: Binding(get: { cab.innerDrawers }, set: { v in
                        withAnimation(.snappy) { var c = cab; c.innerDrawers = v; project.selected = c }
                    }), in: 0...5)
                    .monospacedDigit()
                    .help("Blum LEGRABOX inner drawers behind the doors, sharing the opening from the bottom. They replace the shelves; doors hang on 0-protrusion hinges.")
                    .accessibilityLabel("Inner drawers")
                    .accessibilityValue("\(cab.innerDrawers)")
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
        case "tall", "wardrobe": [1800, 2100, 2250, 2400]
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

/// A rangehood canopy's insert cut-out, from the insert maker's drawing.
struct InsertSection: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    let cab: CabinetDoc

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("Rangehood insert")
            row("Width", cab.insertWidth, [480, 520, 560, 700, 800]) { c, v in c.insertWidth = v }
            row("Depth", cab.insertDepth, [240, 260, 280, 300]) { c, v in c.insertDepth = v }
            Text("The cut-out in the canopy's bottom. Take it from the insert's installation drawing.")
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private func row(_ title: String, _ value: Double, _ presets: [Double],
                     set: @escaping (inout CabinetDoc, Double) -> Void) -> some View {
        ChipRow(title) {
            ForEach(presets, id: \.self) { p in
                Button(unit.format(p, suffix: false)) {
                    withAnimation(.snappy) { var c = cab; set(&c, p); project.selected = c }
                }
                .buttonStyle(ChipStyle(selected: abs(p - value) < 0.5))
            }
        }
    }
}

/// A benchtop's material (BENCH-7), whose rules set its overhang supports and cut-out
/// clearances; each rule can be changed for this benchtop.
struct MaterialSection: View {
    @Environment(ProjectModel.self) private var project
    @State private var open = false
    let cab: CabinetDoc

    var body: some View {
        let materials = benchtopMaterials()
        let current = materials.first { $0.id == cab.material }
        VStack(alignment: .leading, spacing: 8) {
            StoneProductControls(cab: cab)
            MenuRow("Material") {
                Menu {
                    ForEach(materials, id: \.id) { m in
                        Button {
                            withAnimation(.snappy) { var c = cab; if m.id != "porcelain" { c.setStoneProduct(nil) }; c.material = m.id; project.selected = c }
                        } label: {
                            Text(m.label)
                            Text(m.description)
                        }
                    }
                } label: {
                    Text(current?.label ?? cab.material).lineLimit(1)
                }
                .menuStyle(.borderlessButton)
                .fixedSize()
                .help("Its overhang supports and cut-out clearances follow the material's rules")
            }
            let rules = (try? benchtopRules(itemJson: cab.json)) ?? []
            if rules.isEmpty {
                Text("No rules for this material in the catalog.").font(.caption).foregroundStyle(Theme.warning)
            } else {
                DisclosureGroup(isExpanded: $open) {
                    Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 4) {
                        ForEach(rules, id: \.field) { r in
                            GridRow {
                                Text(r.label).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                                // A value equal to the material's is no change: it follows the material again.
                                TextField(r.label, value: Binding(get: { r.value },
                                                                  set: { v in
                                                                      var c = cab
                                                                      c.setMaterialRule(r.field, abs(v - r.materialValue) < 1e-9 ? nil : v)
                                                                      project.selected = c
                                                                  }),
                                          format: .number.precision(.fractionLength(0...1)))
                                    .multilineTextAlignment(.trailing)
                                    .frame(width: 56)
                                    .labelsHidden()
                                Button {
                                    var c = cab; c.setMaterialRule(r.field, nil); project.selected = c
                                } label: { Image(systemName: "arrow.uturn.backward.circle") }
                                    .buttonStyle(.borderless)
                                    .opacity(r.overridden ? 1 : 0)
                                    .disabled(!r.overridden)
                                    .help("Back to the material's value")
                            }
                        }
                    }
                    .padding(.top, 4)
                } label: {
                    Text("Material rules").foregroundStyle(.secondary)
                }
            }
        }
    }
}

/// The job's sink and cooktop templates (BENCH-9): each cut-out made from one follows its edits.
struct AppliancesEditor: View {
    @Environment(ProjectModel.self) private var project
    @State private var open = false

    var body: some View {
        let library = project.appliances
        DisclosureGroup(isExpanded: $open) {
            VStack(alignment: .leading, spacing: 10) {
                ForEach(library.indices, id: \.self) { i in
                    card(i, library[i])
                }
                HStack {
                    Button { project.addAppliance(kind: "sink") } label: { Label("Sink", systemImage: "plus") }
                    Button { project.addAppliance(kind: "cooktop") } label: { Label("Cooktop", systemImage: "plus") }
                }
                .buttonStyle(.borderless)
                Text("Copy each appliance's cut-out from its installation template. A cut-out keeps the template's name; editing the template re-cuts it.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .padding(.top, 6)
        } label: {
            Text("Appliances (\(library.count))").foregroundStyle(.secondary)
        }
    }

    private func card(_ i: Int, _ t: [String: Any]) -> some View {
        let sink = t["kind"] as? String == "sink"
        return VStack(alignment: .leading, spacing: 6) {
            HStack {
                Image(systemName: sink ? "drop" : "flame").foregroundStyle(.secondary)
                TextField("Name", text: Binding(get: { t["name"] as? String ?? "" },
                                                 set: { project.setAppliance(i, "name", $0) }))
                    .textFieldStyle(.plain)
                    .font(.body.weight(.medium))
                Button { project.removeAppliance(i) } label: { Image(systemName: "trash") }
                    .buttonStyle(.borderless)
                    .help("Remove this template; cut-outs made from it stay")
            }
            Grid(alignment: .leading, horizontalSpacing: 8, verticalSpacing: 4) {
                GridRow {
                    number(i, "Width", "width", t)
                    number(i, "Depth", "depth", t)
                }
                GridRow {
                    number(i, "Corners R", "corner_radius", t)
                    number(i, "Setback", "setback", t)
                }
                if sink {
                    GridRow {
                        number(i, "Tap hole Ø", "tap_hole", t, ours: 35)
                        number(i, "Tap behind", "tap_hole_behind", t)
                    }
                }
            }
            if sink {
                Picker("Mount", selection: Binding(get: { t["mount"] as? String ?? "undermount" },
                                                   set: { project.setAppliance(i, "mount", $0) })) {
                    Text("Undermount").tag("undermount")
                    Text("Top mount").tag("top_mount")
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .help("Undermount: the cut-out's edge shows and is finished. Top mount: the sink's rim covers it.")
            }
        }
        .padding(8)
        .background(.quaternary.opacity(0.5), in: RoundedRectangle(cornerRadius: 8))
    }

    /// A millimetre field; `ours` is shown when the template leaves the value to our default.
    private func number(_ i: Int, _ title: String, _ field: String, _ t: [String: Any], ours: Double = 0) -> some View {
        HStack(spacing: 4) {
            Text(title).font(.caption).foregroundStyle(.secondary).lineLimit(1)
            Spacer(minLength: 2)
            TextField(title, value: Binding(get: { (t[field] as? NSNumber)?.doubleValue ?? ours },
                                            set: { project.setAppliance(i, field, $0) }),
                      format: .number.precision(.fractionLength(0...1)))
                .multilineTextAlignment(.trailing)
                .frame(width: 52)
                .labelsHidden()
        }
    }
}

/// A wardrobe's hanging, drawn: one long rail under the hat shelf, or two rails one above the other.
struct WardrobeSection: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("Hanging")
            HStack(spacing: 8) {
                ForEach([("long", "Long"), ("double", "Double")], id: \.0) { id, title in
                    Button {
                        withAnimation(.snappy) { var c = cab; c.hanging = id; project.selected = c }
                    } label: {
                        VStack(spacing: 6) {
                            HangingThumbnail(double: id == "double").frame(width: 40, height: 56)
                            Text(title).font(.caption)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                    }
                    .buttonStyle(TileStyle(selected: cab.hanging == id))
                    .help(id == "long" ? "One rail under the hat shelf, for coats and dresses"
                                       : "A second shelf and rail halfway down, for shirts and trousers")
                    .accessibilityLabel("\(title) hanging")
                }
            }
            StandardSizes(after: "wardrobe", cab: cab)
                .padding(.top, 6)
        }
    }
}

struct HangingThumbnail: View {
    let double: Bool

    var body: some View {
        Canvas { ctx, size in
            let w = size.width, h = size.height
            let box = CGRect(x: 2, y: 2, width: w - 4, height: h - 4)
            ctx.stroke(Path(box), with: .color(.primary.opacity(0.6)), lineWidth: 1)
            let shelves: [CGFloat] = double ? [0.18, 0.55] : [0.18]
            for f in shelves {
                let y = box.minY + box.height * f
                ctx.fill(Path(CGRect(x: box.minX, y: y, width: box.width, height: 2)), with: .color(.primary.opacity(0.5)))
                var rail = Path()
                rail.move(to: CGPoint(x: box.minX + 3, y: y + 5))
                rail.addLine(to: CGPoint(x: box.maxX - 3, y: y + 5))
                ctx.stroke(rail, with: .color(.accentColor), lineWidth: 1.6)
                // Hangers with clothes.
                for x in stride(from: box.minX + 7, to: box.maxX - 4, by: 7) {
                    let drop = (double ? 0.3 : 0.6) * box.height
                    ctx.fill(Path(CGRect(x: x - 1.5, y: y + 6, width: 3, height: drop)), with: .color(.accentColor.opacity(0.25)))
                }
            }
        }
        .accessibilityHidden(true)
    }
}

/// A benchtop's front edge in section: the top's end on the right.
struct EdgeProfileThumbnail: View {
    let profile: String

    var body: some View {
        Canvas { ctx, size in
            let w = size.width, h = size.height
            let t: CGFloat = h * 0.3, top: CGFloat = h * 0.25
            var p = Path()
            p.move(to: CGPoint(x: 2, y: top))
            switch profile {
            case "pencil_round":
                p.addLine(to: CGPoint(x: w - 8, y: top))
                p.addQuadCurve(to: CGPoint(x: w - 4, y: top + 4), control: CGPoint(x: w - 4, y: top))
                p.addLine(to: CGPoint(x: w - 4, y: top + t - 4))
                p.addQuadCurve(to: CGPoint(x: w - 8, y: top + t), control: CGPoint(x: w - 4, y: top + t))
            case "bullnose":
                p.addLine(to: CGPoint(x: w - 4 - t / 2, y: top))
                p.addArc(center: CGPoint(x: w - 4 - t / 2, y: top + t / 2), radius: t / 2,
                         startAngle: .degrees(-90), endAngle: .degrees(90), clockwise: false)
            case "bevel":
                p.addLine(to: CGPoint(x: w - 10, y: top))
                p.addLine(to: CGPoint(x: w - 4, y: top + 6))
                p.addLine(to: CGPoint(x: w - 4, y: top + t))
            case "mitred_apron":
                p.addLine(to: CGPoint(x: w - 4, y: top))
                p.addLine(to: CGPoint(x: w - 4, y: h - 2))
                p.addLine(to: CGPoint(x: w - 4 - t * 0.6, y: h - 2))
                p.addLine(to: CGPoint(x: w - 4 - t * 0.6, y: top + t))
            default:
                p.addLine(to: CGPoint(x: w - 4, y: top))
                p.addLine(to: CGPoint(x: w - 4, y: top + t))
            }
            p.addLine(to: CGPoint(x: 2, y: top + t))
            p.closeSubpath()
            ctx.fill(p, with: .color(.primary.opacity(0.15)))
            ctx.stroke(p, with: .color(.primary.opacity(0.75)), lineWidth: 1.2)
        }
        .accessibilityHidden(true)
    }
}

/// Lining boards on the faces seen from the room (an island's back and ends), drawn from above.
struct CladdingSection: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    @State private var picking = false
    let cab: CabinetDoc

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("Seen faces")
            HStack(spacing: 8) {
                ForEach([("left", "Left"), ("back", "Back"), ("right", "Right")], id: \.0) { face, title in
                    let on = cab.seenFace(face)
                    Button {
                        withAnimation(.snappy) { var c = cab; c.setSeenFace(face, !on); project.selected = c }
                    } label: {
                        VStack(spacing: 6) {
                            CladThumbnail(face: face, on: on).frame(width: 48, height: 40)
                            Text(title).font(.caption)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                    }
                    .buttonStyle(TileStyle(selected: on))
                    .help("The \(title.lowercased()) face is seen from the room: no screws through it, and a finished board over it from the front's face to the back")
                    .accessibilityLabel("Seen \(title.lowercased())")
                    .accessibilityValue(on ? "on" : "off")
                }
            }
            if ["left", "back", "right"].contains(where: cab.seenFace) {
                // Any board: a Surround profiled lining, a plain lining, or a decor board.
                let board = project.boardsById[cab.cladBoard]
                Button { picking = true } label: {
                    HStack(spacing: 10) {
                        RoundedRectangle(cornerRadius: 5)
                            .fill(board.map { Color(nsColor: $0.displayColour) } ?? Color.primary.opacity(0.08))
                            .overlay {
                                if board?.grained == true { GrainPattern().clipShape(RoundedRectangle(cornerRadius: 5)) }
                            }
                            .overlay(RoundedRectangle(cornerRadius: 5).strokeBorder(.primary.opacity(0.2)))
                            .frame(width: 34, height: 24)
                        VStack(alignment: .leading, spacing: 1) {
                            Text(board?.profileModule ?? 0 > 0 ? "Lining, \(board!.range)" : "Finished board")
                                .font(.caption).foregroundStyle(.secondary).lineLimit(1)
                            Text(board.map { "\($0.decor) \($0.finish), \(Int($0.thickness)) mm" } ?? "Same as the fronts")
                                .lineLimit(1)
                        }
                        Spacer()
                        Image(systemName: "chevron.right").foregroundStyle(.tertiary)
                    }
                    .padding(6)
                    .contentShape(Rectangle())
                }
                .buttonStyle(TileStyle(selected: picking))
                .help("The board over the seen faces, the same as the fronts unless you choose one: a Surround profiled lining, a plain lining or any decor board")
                .popover(isPresented: $picking, arrowEdge: .leading) {
                    // Linings first: they are what cladding is usually in.
                    let boards = project.boards.filter { $0.kind == "lining" } + project.boards.filter { $0.kind != "lining" }
                    BoardPicker(boards: boards, marked: [cab.cladBoard], columns: 140, onPick: { b in
                        var c = cab
                        c.cladBoard = b.id
                        c.cladThickness = b.thickness
                        withAnimation(.snappy) { project.selected = c }
                        picking = false
                    }) { _ in EmptyView() }
                        .frame(width: 640, height: 520)
                }
                if let board, board.needsPaint {
                    PaintPicker(board: board.id, cab: cab)
                }
                if cab.seenFace("back") {
                    HStack {
                        Text("Maximum back panel").foregroundStyle(.secondary)
                        Spacer()
                        TextField("Maximum back panel", value: Binding(get: {cab.backPanelMaximum}, set: {project.setSeenBackLayout(maximum: $0)}), format: .number)
                            .textFieldStyle(.roundedBorder).multilineTextAlignment(.trailing).frame(width: 70)
                        Text("mm").font(.caption).foregroundStyle(.secondary)
                    }
                    HStack {
                        Text("Back grain").foregroundStyle(.secondary)
                        Spacer()
                        ForEach([("x","Across run"),("z","Vertical")], id: \.0) { value, title in
                            Button(title) { withAnimation(.snappy) { project.setSeenBackLayout(grain: value) } }
                                .buttonStyle(ChipStyle(selected: cab.backPanelGrain == value))
                        }
                    }
                    Button("Match seen boards across run") { project.matchSeenBoardsAcrossRun() }
                        .buttonStyle(.borderless)
                        .help("Use this cabinet's finished board, thickness, grain and maximum width on all cabinets with seen backs. Their seen sides use the same finished board.")
                    Text("Different boards, thicknesses, grain or maximum widths create separate panels. Matching seen backs form continuous panels, with joins at cabinet boundaries up to this width. Layout changes apply to matching seen backs in the job; choose a maximum that fits your stock sheets.")
                        .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                }

                if board != nil {
                    Button("Same as the fronts") {
                        var c = cab
                        c.cladBoard = ""
                        withAnimation(.snappy) { project.selected = c }
                    }
                    .buttonStyle(ChipStyle(selected: false))
                }
                if board == nil, cab.clad("back") {
                HStack(spacing: 6) {
                    Text("Back board thickness").foregroundStyle(.secondary)
                    Spacer()
                    ForEach([9.0, 12.0, 16.0], id: \.self) { t in
                        Button(unit.format(t, suffix: false)) {
                            withAnimation(.snappy) { var c = cab; c.cladThickness = t; project.selected = c }
                        }
                        .buttonStyle(ChipStyle(selected: abs(cab.cladThickness - t) < 0.5))
                    }
                }
                }
            }
        }
    }
}

/// A cabinet from above, its front at the bottom, with the clad face fluted.
struct CladThumbnail: View {
    let face: String
    let on: Bool

    var body: some View {
        Canvas { ctx, size in
            let w = size.width, h = size.height
            let body = CGRect(x: w * 0.2, y: h * 0.25, width: w * 0.6, height: h * 0.5)
            ctx.stroke(Path(body), with: .color(.primary.opacity(0.6)), lineWidth: 1)
            let strip: CGRect = switch face {
            case "left": CGRect(x: body.minX - 5, y: body.minY, width: 4, height: body.height + 4)
            case "right": CGRect(x: body.maxX + 1, y: body.minY, width: 4, height: body.height + 4)
            default: CGRect(x: body.minX, y: body.minY - 5, width: body.width, height: 4)
            }
            ctx.fill(Path(strip), with: .color(on ? Color.accentColor : Color.primary.opacity(0.2)))
        }
        .accessibilityHidden(true)
    }
}

/// LED strips: tap a zone on the drawn cabinet to light it, then the strips' voltage and power.
struct LightsSection: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    private var zones: [(id: String, title: String, help: String)] {
        var z: [(String, String, String)] = []
        if ["wall", "open_shelf", "corner_wall"].contains(cab.kind) {
            z.append(("under_cabinet", "Under", "A strip under the cabinet, lighting the bench"))
        }
        z.append(("in_cabinet", "Inside", "A strip under the top, inside the cabinet"))
        z.append(("side_rear", "Up sides", "A strip up the rear of each side, lighting across to the other: for a wardrobe"))
        if !cab.fixedShelves.isEmpty {
            z.append(("under_fixed_shelf", "Under shelf", "A strip under each fixed shelf, 50 mm from the rear"))
        }
        if ["base", "tall", "drawer_bank", "blind_corner", "corner_base", "sink", "wardrobe"].contains(cab.kind) {
            z.append(("plinth", "Plinth", "A strip in the plinth, lighting the floor"))
        }
        return z
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("Lights")
            HStack(spacing: 8) {
                ForEach(zones, id: \.id) { zone in
                    let on = cab.hasLight(zone.id)
                    Button {
                        withAnimation(.snappy) { var c = cab; c.toggleLight(zone.id); project.selected = c }
                    } label: {
                        VStack(spacing: 6) {
                            LightZoneThumbnail(zone: zone.id, on: on).frame(width: 52, height: 44)
                            Text(zone.title).font(.caption)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 8)
                    }
                    .buttonStyle(TileStyle(selected: on))
                    .help(zone.help + ". A 10 mm cable hole is drilled for it.")
                    .accessibilityLabel("\(zone.title) light")
                    .accessibilityValue(on ? "on" : "off")
                }
            }
            if !cab.lights.isEmpty {
                HStack(spacing: 6) {
                    Text("Strip").foregroundStyle(.secondary)
                    Spacer(minLength: 0)
                    ForEach(["12v", "24v"], id: \.self) { v in
                        Button(v == "12v" ? "12 V" : "24 V") {
                            withAnimation(.snappy) { var c = cab; c.lightVoltage = v; project.selected = c }
                        }
                        .buttonStyle(ChipStyle(selected: cab.lightVoltage == v))
                        .help("Strips of one voltage share a driver; 12 V and 24 V never mix")
                    }
                }
                HStack(spacing: 6) {
                    Text("Power").foregroundStyle(.secondary)
                    Spacer(minLength: 0)
                    ForEach([4.8, 9.6, 14.4], id: \.self) { w in
                        Button(String(format: "%.1f W/m", w)) {
                            withAnimation(.snappy) { var c = cab; c.lightWatts = w; project.selected = c }
                        }
                        .buttonStyle(ChipStyle(selected: abs(cab.lightWatts - w) < 0.05))
                        .help("The strip's power per metre, from its data sheet")
                    }
                }
                SectionTitle("Profile")
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 6), count: 3), spacing: 6) {
                    profileTile(id: "", title: "Bare strip", help: "No profile: the strip stuck to the panel", thumbnail: ProfileThumbnail(surface: true, width: 0, depth: 0))
                    ForEach(lightProfiles(), id: \.id) { p in
                        profileTile(id: p.id, title: p.name.replacingOccurrences(of: "Loox5 profile ", with: ""),
                                    help: "\(p.name), Häfele \(p.article). \(p.variant)",
                                    thumbnail: ProfileThumbnail(surface: p.surface, width: p.width, depth: p.depth))
                    }
                }
            }
        }
    }

    private func profileTile(id: String, title: String, help: String, thumbnail: ProfileThumbnail) -> some View {
        Button {
            withAnimation(.snappy) { var c = cab; c.lightProfile = id; project.selected = c }
        } label: {
            VStack(spacing: 4) {
                thumbnail.frame(height: 34)
                Text(title).font(.caption2).lineLimit(1)
            }
            .padding(5)
            .frame(maxWidth: .infinity)
        }
        .buttonStyle(TileStyle(selected: cab.lightProfile == id))
        .help(help)
        .accessibilityLabel(id.isEmpty ? title : "Profile \(title)")
    }
}

/// A profile end on, to scale, under the panel it fixes to: let into it, or on its face.
struct ProfileThumbnail: View {
    let surface: Bool
    let width: Double
    let depth: Double

    var body: some View {
        Canvas { ctx, size in
            let face = size.height * 0.45
            let k = 1.1
            let panel = CGRect(x: 0, y: 0, width: size.width, height: face)
            ctx.fill(Path(panel), with: .color(Color.primary.opacity(0.18)))
            let (w, d) = (CGFloat(width) * k, CGFloat(depth) * k)
            let x = (size.width - w) / 2
            if width > 0 {
                let body = CGRect(x: x, y: surface ? face : face - d, width: w, height: d)
                ctx.fill(Path(body), with: .color(.secondary))
                let glow = CGRect(x: x, y: body.maxY - 2, width: w, height: 2)
                ctx.fill(Path(glow), with: .color(Theme.accent))
            } else {
                ctx.fill(Path(CGRect(x: size.width * 0.35, y: face, width: size.width * 0.3, height: 2)), with: .color(Theme.accent))
            }
        }
        .accessibilityHidden(true)
    }
}

/// A cabinet from the front with the zone's strip as a glowing line.
struct LightZoneThumbnail: View {
    let zone: String
    let on: Bool

    var body: some View {
        Canvas { ctx, size in
            let w = size.width, h = size.height
            let body = CGRect(x: w * 0.15, y: h * 0.12, width: w * 0.7, height: h * 0.6)
            ctx.stroke(Path(body), with: .color(.primary.opacity(0.6)), lineWidth: 1)
            if zone == "side_rear" {
                // Upright at each side, lit across to the other.
                for x in [body.minX + 5, body.maxX - 5] {
                    var strip = Path()
                    strip.move(to: CGPoint(x: x, y: body.minY + 4))
                    strip.addLine(to: CGPoint(x: x, y: body.maxY - 4))
                    if on { ctx.stroke(strip, with: .color(.yellow.opacity(0.35)), lineWidth: 6) }
                    ctx.stroke(strip, with: .color(on ? .yellow : .primary.opacity(0.25)), lineWidth: 2)
                }
                return
            }
            let y: CGFloat
            switch zone {
            case "under_cabinet": y = body.maxY + 3
            case "under_fixed_shelf":
                var shelf = Path()
                shelf.move(to: CGPoint(x: body.minX, y: body.midY))
                shelf.addLine(to: CGPoint(x: body.maxX, y: body.midY))
                ctx.stroke(shelf, with: .color(.primary.opacity(0.6)), lineWidth: 1)
                y = body.midY + 3
            case "plinth":
                ctx.stroke(Path(CGRect(x: body.minX + 4, y: body.maxY, width: body.width - 8, height: h * 0.18)),
                           with: .color(.primary.opacity(0.4)), lineWidth: 1)
                y = body.maxY + h * 0.12
            default: y = body.minY + 4
            }
            var strip = Path()
            strip.move(to: CGPoint(x: body.minX + 5, y: y))
            strip.addLine(to: CGPoint(x: body.maxX - 5, y: y))
            let colour: Color = on ? .yellow : .primary.opacity(0.25)
            if on {
                ctx.stroke(strip, with: .color(.yellow.opacity(0.35)), lineWidth: 6)
            }
            ctx.stroke(strip, with: .color(colour), lineWidth: 2)
        }
        .accessibilityHidden(true)
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
        ChipRow(title) {
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

    /// Second-leg lengths: wall corners are shallower, so their legs are shorter.
    private var legLengths: [Double] {
        cab.kind == "corner_wall" ? [600, 650, 700, 800, 900] : [800, 900, 1000, 1100, 1200]
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle(cab.kind == "corner_wall" ? "Corner wall" : "L-corner")
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
                ForEach(legLengths, id: \.self) { p in
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

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("Fronts")
            SectionsEditor(cab: cab)
            if cab.hasFront("door") {
                HStack(spacing: 6) {
                    Text("Arched top").foregroundStyle(.secondary)
                    Spacer()
                    ForEach([0.0, 40, 60, 80], id: \.self) { rise in
                        Button(rise == 0 ? "Square" : "\(Int(rise))") {
                            withAnimation(.snappy) { var c = cab; c.archedTop = rise; project.selected = c }
                        }
                        .buttonStyle(ChipStyle(selected: abs(cab.archedTop - rise) < 0.5))
                        .accessibilityLabel(rise == 0 ? "Square top" : "Arched top, \(Int(rise)) millimetres")
                    }
                }
                .help("An arched top across the doors: this many millimetres higher in the middle than at the sides (slab doors)")
            }
            if cab.hasFront("drawers") {
                Toggle("Drawer inserts", isOn: Binding(get: { cab.drawerInserts }, set: { v in
                    withAnimation(.snappy) { var c = cab; c.drawerInserts = v; project.selected = c }
                }))
                .help("Blum AMBIA-LINE in LEGRABOX and MERIVOBOX drawers (a cutlery insert on top, frames below), ORGA-LINE in TANDEMBOX (containers on top, utensil dividers below), sized for the cabinet")
                Toggle("Lock", isOn: Binding(get: { cab.lock }, set: { v in
                    withAnimation(.snappy) { var c = cab; c.lock = v; project.selected = c }
                }))
                .help("Blum CABLOXX: one locking bar locks every drawer (up to five); fit a cylinder lock, bought separately")
            }
        }
    }
}

/// How a lifting front moves, drawn from the side: each picks its AVENTOS family.
struct LiftKindPicker: View {
    let selected: String
    let onPick: (String) -> Void

    private let kinds: [(id: String, title: String, help: String)] = [
        ("stay_lift", "Stay lift", "Swings up about its top edge and stays open (AVENTOS HK top)"),
        ("up_and_over", "Up and over", "Rises and tips over the top, for large fronts (AVENTOS HS top)"),
        ("lift_up", "Lift up", "Rises parallel to itself, for small fronts (AVENTOS HL top)"),
        ("small_stay_lift", "Small stay", "A small stay lift for small wall cabinets and over the fridge (AVENTOS HK-S)"),
        ("hinged_stay_lift", "On hinges", "A stay lift on hinges along the top, for the smallest fronts (AVENTOS HK-XS)"),
        ("integrated_stay_lift", "Integrated", "A stay lift hidden in pockets in the cabinet sides (AVENTOS HKi)"),
    ]

    var body: some View {
        LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 3), spacing: 8) {
            ForEach(kinds, id: \.id) { kind in
                Button {
                    onPick(kind.id)
                } label: {
                    VStack(spacing: 6) {
                        LiftKindThumbnail(kind: kind.id).frame(width: 52, height: 44)
                        Text(kind.title).font(.caption).lineLimit(1).minimumScaleFactor(0.8)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                }
                .buttonStyle(TileStyle(selected: selected == kind.id))
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
            case "small_stay_lift":
                open.move(to: CGPoint(x: body.minX - 2, y: body.minY))
                open.addLine(to: CGPoint(x: w * 0.12, y: body.minY - h * 0.15))
            case "integrated_stay_lift":
                open.move(to: CGPoint(x: body.minX - 2, y: body.minY))
                open.addLine(to: CGPoint(x: 2, y: body.minY - h * 0.25))
                ctx.fill(Path(CGRect(x: body.minX + 2, y: body.minY + 3, width: body.width * 0.45, height: 5)),
                         with: .color(.accentColor.opacity(0.5)))
            case "hinged_stay_lift":
                open.move(to: CGPoint(x: body.minX - 2, y: body.minY))
                open.addLine(to: CGPoint(x: w * 0.12, y: body.minY - h * 0.15))
                ctx.fill(Path(ellipseIn: CGRect(x: body.minX - 4, y: body.minY - 2, width: 4, height: 4)),
                         with: .color(.accentColor))
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
    let selected: [Double]
    let onPick: ([Double]) -> Void

    private let stacks: [[Double]] = [[1], [1, 1], [1, 1, 1], [1, 1, 1, 1], [1, 2], [1, 1, 2], [1, 2, 3], [1, 1, 1, 2]]

    var body: some View {
        LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 4), spacing: 8) {
            ForEach(stacks.indices, id: \.self) { i in
                let stack = stacks[i]
                Button {
                    onPick(stack)
                } label: {
                    FrontThumbnail(layout: "drawers", hinge: "left", ratios: stack)
                        .frame(width: 34, height: 42)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 6)
                }
                .buttonStyle(TileStyle(selected: selected == stack))
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
            .font(.caption.monospacedDigit().weight(selected ? .semibold : .regular))
            .padding(.horizontal, 9)
            .padding(.vertical, 4)
            .background(
                RoundedRectangle(cornerRadius: 6)
                    .fill(selected ? Theme.accent : (configuration.isPressed ? Theme.hover : Theme.control))
            )
            .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(selected ? Theme.accent : Theme.border))
            .foregroundStyle(selected ? Theme.onAccent : Theme.text)
    }
}

struct TileStyle: ButtonStyle {
    let selected: Bool

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .accessibilityAddTraits(selected ? .isSelected : [])
            .background(
                RoundedRectangle(cornerRadius: 9)
                    .fill(selected ? Theme.selected : (configuration.isPressed ? Theme.hover : Theme.strip))
            )
            .overlay(
                RoundedRectangle(cornerRadius: 9)
                    .strokeBorder(selected ? Theme.accent : Theme.border, lineWidth: selected ? 1.5 : 1)
            )
            .contentShape(RoundedRectangle(cornerRadius: 9))
            .scaleEffect(configuration.isPressed ? 0.97 : 1)
            .animation(.snappy(duration: 0.15), value: configuration.isPressed)
    }
}
