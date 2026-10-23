import SwiftUI

/// The board slots a cabinet uses, as the core names them.
enum BoardSlot: String, CaseIterable, Identifiable {
    case carcass, front, back
    var id: String { rawValue }

    var title: String {
        switch self {
        case .carcass: "Carcass"
        case .front: "Fronts"
        case .back: "Back"
        }
    }

    var symbol: String {
        switch self {
        case .carcass: "square.split.bottomrightquarter"
        case .front: "door.left.hand.closed"
        case .back: "rectangle.portrait"
        }
    }
}

/// One decor in one finish from one brand; its boards are the thicknesses it comes in.
struct DecorGroup: Identifiable {
    let boards: [BoardRecord]
    var id: String { "\(first.brand)|\(first.range)|\(first.decor)|\(first.finish)" }
    var first: BoardRecord { boards[0] }
}

/// Brand, range, finish and grain filters over a grid of decor swatches. Each swatch offers the
/// thicknesses it comes in; `onPick` gets the board chosen.
struct BoardPicker<Accessory: View>: View {
    let boards: [BoardRecord]
    /// Board ids to mark as in use.
    var marked: Set<String> = []
    var columns = 170.0
    let onPick: (BoardRecord) -> Void
    @ViewBuilder var accessory: (BoardRecord) -> Accessory

    @State private var brand = ""
    @State private var range = ""
    @State private var finishes: Set<String> = []
    @State private var grain = Grain.all
    @State private var search = ""

    enum Grain: String, CaseIterable, Identifiable {
        case all = "All", wood = "Woodgrains", solid = "Colours"
        var id: String { rawValue }
    }

    private var brands: [String] { Array(Set(boards.map(\.brand))).sorted() }
    private var ranges: [String] {
        Array(Set(boards.filter { brand.isEmpty || $0.brand == brand }.map(\.range))).sorted()
    }
    private var finishList: [String] {
        let pool = boards.filter { (brand.isEmpty || $0.brand == brand) && (range.isEmpty || $0.range == range) }
        let counts = Dictionary(grouping: pool, by: \.finish).mapValues(\.count)
        return counts.keys.sorted { counts[$0]! > counts[$1]! }
    }

    private var groups: [DecorGroup] {
        let shown = boards.filter { b in
            (brand.isEmpty || b.brand == brand)
                && (range.isEmpty || b.range == range)
                && (finishes.isEmpty || finishes.contains(b.finish))
                && (grain == .all || b.grained == (grain == .wood))
                && (search.isEmpty || "\(b.decor) \(b.finish) \(b.brand)".localizedCaseInsensitiveContains(search))
        }
        let byDecor = Dictionary(grouping: shown) { "\($0.brand)|\($0.range)|\($0.decor)|\($0.finish)" }
        return byDecor.values
            .map { DecorGroup(boards: $0.sorted { $0.thickness < $1.thickness }) }
            .sorted { ($0.first.decor, $0.first.finish) < ($1.first.decor, $1.first.finish) }
    }

    var body: some View {
        VStack(spacing: 0) {
            filters
                .padding(.horizontal, 14)
                .padding(.vertical, 10)
            Divider()
            let groups = self.groups
            ScrollView {
                if groups.isEmpty {
                    ContentUnavailableView("No boards", systemImage: "square.stack.3d.up.slash",
                                           description: Text(boards.isEmpty
                                                             ? "Board data from Polytec, Laminex and others is added to catalog/boards."
                                                             : "Nothing matches these filters."))
                        .padding(.top, 60)
                }
                LazyVGrid(columns: [GridItem(.adaptive(minimum: columns), spacing: 14)], spacing: 14) {
                    ForEach(groups) { g in
                        DecorCard(group: g, marked: marked, onPick: onPick, accessory: accessory)
                    }
                }
                .padding(14)
            }
        }
    }

    private var filters: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) {
                Picker("Brand", selection: $brand) {
                    Text("All brands").tag("")
                    ForEach(brands, id: \.self) { Text($0).tag($0) }
                }
                .fixedSize()
                Picker("Range", selection: $range) {
                    Text("All ranges").tag("")
                    ForEach(ranges, id: \.self) { Text($0).tag($0) }
                }
                .fixedSize()
                Picker("Grain", selection: $grain) {
                    ForEach(Grain.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .fixedSize()
                TextField("Search decors", text: $search)
                    .textFieldStyle(.roundedBorder)
                    .frame(minWidth: 120)
            }
            .labelsHidden()
            .onChange(of: brand) { range = "" }
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(finishList, id: \.self) { f in
                        let on = finishes.contains(f)
                        Button(f) {
                            withAnimation(.snappy) {
                                if on { finishes.remove(f) } else { finishes.insert(f) }
                            }
                        }
                        .buttonStyle(ChipStyle(selected: on))
                        .accessibilityLabel("Finish \(f)")
                    }
                }
            }
        }
        .controlSize(.small)
    }
}

/// One decor: its swatch, name, finish and the thicknesses to pick from.
struct DecorCard<Accessory: View>: View {
    let group: DecorGroup
    let marked: Set<String>
    let onPick: (BoardRecord) -> Void
    @ViewBuilder var accessory: (BoardRecord) -> Accessory
    @State private var chosen: String?

    private var board: BoardRecord {
        group.boards.first { $0.id == chosen } ?? group.boards.first { marked.contains($0.id) } ?? group.boards[0]
    }

    var body: some View {
        let b = board
        VStack(alignment: .leading, spacing: 6) {
            Button {
                onPick(b)
            } label: {
                RoundedRectangle(cornerRadius: 10)
                    .fill(Color(nsColor: b.displayColour))
                    .overlay {
                        if b.grained { GrainPattern().clipShape(RoundedRectangle(cornerRadius: 10)) }
                    }
                    .overlay(alignment: .topTrailing) {
                        HStack(spacing: 4) {
                            if b.colourIsGuessed {
                                Image(systemName: "eyedropper.halffull")
                                    .help("Colour guessed from the decor's name: see the brand's swatch")
                            }
                            if !b.verified {
                                Image(systemName: "checkmark.seal")
                                    .help("Not yet checked against the supplier's document")
                            }
                        }
                        .foregroundStyle(.white.opacity(0.85))
                        .shadow(radius: 1)
                        .padding(6)
                    }
                    .overlay {
                        if group.boards.contains(where: { marked.contains($0.id) }) {
                            RoundedRectangle(cornerRadius: 10).strokeBorder(Color.accentColor, lineWidth: 3)
                        }
                    }
                    .frame(height: 90)
                    .shadow(color: .black.opacity(0.15), radius: 4, y: 2)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("\(b.decor) \(b.finish), \(Int(b.thickness)) mm\(b.colourIsGuessed ? ", colour approximate" : "")")
            HStack {
                Text(b.decor).font(.headline).lineLimit(1)
                Spacer()
                accessory(b)
            }
            Text("\(b.brand) · \(b.finish)").font(.subheadline).foregroundStyle(.secondary).lineLimit(1)
            HStack(spacing: 4) {
                ForEach(group.boards, id: \.id) { t in
                    Button("\(Int(t.thickness))") { withAnimation(.snappy) { chosen = t.id } }
                        .buttonStyle(ChipStyle(selected: t.id == b.id))
                        .help("\(Int(t.thickness)) mm \(t.substrate.replacingOccurrences(of: "_", with: " ")) · \(t.sheets.joined(separator: ", "))")
                        .accessibilityLabel("\(Int(t.thickness)) millimetres")
                }
                Text("mm").font(.caption).foregroundStyle(.secondary)
            }
        }
    }
}

/// The Boards workspace: every board, with "Use for" on each card for the selected cabinet.
struct BoardLibraryView: View {
    @Environment(ProjectModel.self) private var project

    var body: some View {
        let used = Set(BoardSlot.allCases.compactMap { slot in project.selected.map { $0.board(for: slot.rawValue) } })
        BoardPicker(boards: project.boards, marked: used, onPick: { _ in }) { b in
            Menu {
                ForEach(BoardSlot.allCases) { slot in
                    Button("Use for \(slot.title)", systemImage: slot.symbol) { project.useBoard(b, for: slot.rawValue) }
                }
            } label: {
                Image(systemName: "plus.circle")
            }
            .menuStyle(.borderlessButton)
            .fixedSize()
            .disabled(project.selected == nil)
            .help(project.selected.map { "Use this board for \($0.id)" } ?? "Select a cabinet first")
        }
    }
}

/// The selected cabinet's boards, one row per slot; a click opens the picker for that slot.
struct BoardsSection: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc
    @State private var picking: BoardSlot?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionTitle("Boards")
            ForEach(BoardSlot.allCases) { slot in
                let board = project.boardsById[cab.board(for: slot.rawValue)]
                Button {
                    picking = slot
                } label: {
                    HStack(spacing: 10) {
                        RoundedRectangle(cornerRadius: 5)
                            .fill(board.map { Color(nsColor: $0.displayColour) } ?? Color.primary.opacity(0.08))
                            .overlay {
                                if board?.grained == true { GrainPattern().clipShape(RoundedRectangle(cornerRadius: 5)) }
                            }
                            .overlay(RoundedRectangle(cornerRadius: 5).strokeBorder(.primary.opacity(0.2)))
                            .frame(width: 34, height: 24)
                        VStack(alignment: .leading, spacing: 1) {
                            Text(slot.title).font(.caption).foregroundStyle(.secondary)
                            Text(board.map { "\($0.decor) \($0.finish), \(Int($0.thickness)) mm" } ?? "Generic board")
                                .lineLimit(1)
                        }
                        Spacer()
                        Image(systemName: "chevron.right").foregroundStyle(.tertiary)
                    }
                    .padding(6)
                    .contentShape(Rectangle())
                }
                .buttonStyle(TileStyle(selected: picking == slot))
                .accessibilityLabel("\(slot.title) board: \(board.map { "\($0.decor) \($0.finish)" } ?? "generic")")
                .popover(isPresented: Binding(get: { picking == slot }, set: { if !$0 { picking = nil } }),
                         arrowEdge: .leading) {
                    BoardPicker(boards: project.boards, marked: [cab.board(for: slot.rawValue)], columns: 140,
                                onPick: { b in
                                    project.useBoard(b, for: slot.rawValue)
                                    picking = nil
                                }) { _ in EmptyView() }
                        .frame(width: 640, height: 520)
                }
            }
        }
    }
}
