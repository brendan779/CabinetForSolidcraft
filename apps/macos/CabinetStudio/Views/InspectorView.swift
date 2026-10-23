import SwiftUI

struct InspectorView: View {
    @Environment(ProjectModel.self) private var project

    var body: some View {
        if let cab = project.selected {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    header(cab)
                    SizeSection(cab: cab)
                    FrontsSection(cab: cab)
                    ForEach(optionGroups(), id: \.field) { group in
                        VisualPicker(group: group, selected: project.option(group.field, of: cab)) { choice in
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

/// Size presets as chips, with a fine-tune stepper behind them. Most cabinets are a standard size,
/// so the common case is one click.
struct SizeSection: View {
    @Environment(ProjectModel.self) private var project
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
                    Text("\(Int(value)) mm").monospacedDigit().fontWeight(.medium)
                }
            }
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(presets, id: \.self) { p in
                        Button("\(Int(p))") { withAnimation(.snappy) { set(p) } }
                            .buttonStyle(ChipStyle(selected: abs(p - value) < 0.5))
                    }
                }
            }
        }
    }
}

struct FrontsSection: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc

    private let layouts: [(id: String, title: String)] = [
        ("door", "Door"), ("door_pair", "Pair"), ("drawers", "Drawers"), ("open", "Open"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("Fronts")
            HStack(spacing: 8) {
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
        }
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
