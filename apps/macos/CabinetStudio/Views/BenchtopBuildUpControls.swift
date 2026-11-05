import SwiftUI

struct BenchtopBuildUpControls: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc
    @State private var pickingBoard = false
    @State private var pickingTape = false
    @State private var tapes: [ShadowTapeRecord] = []
    @State private var search = ""

    private func change(_ edit: (inout CabinetDoc) -> Void) {
        var c = cab
        edit(&c)
        withAnimation(.snappy) { project.selected = c }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SectionTitle("Mitred build-up")
            dimension("Finished edge", value: cab.apronDepth, range: cab.topThickness...200) { $0.apronDepth = $1 }
            dimension("Shadow height", value: cab.shadowHeight, range: 0...100) { $0.shadowHeight = $1 }
            dimension("Shadow recess", value: cab.shadowSetback, range: 0...150) { $0.shadowSetback = $1 }
            dimension("Support width", value: cab.supportWidth, range: 30...200) { $0.supportWidth = $1 }
            Text("Zero recess aligns the banded support with the cabinet fronts. The stone sits above cabinet-board packers; finished edge is the full visible stone depth.")
                .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            Button { pickingBoard = true } label: {
                Label(project.boardsById[cab.supportBoard].map { "Supports: \($0.decor), \(Int($0.thickness)) mm" } ?? "Supports: match carcass board", systemImage: "square.stack.3d.up")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .popover(isPresented: $pickingBoard, arrowEdge: .leading) {
                VStack {
                    Button("Match carcass board") { change { $0.resetSupportBoard() }; pickingBoard = false }
                    BoardPicker(boards: project.boards, marked: [cab.supportBoard], columns: 140, onPick: { board in
                        change { $0.setSupportBoard(board) }; pickingBoard = false
                    }) { _ in EmptyView() }
                }.padding().frame(width: 640, height: 520)
            }
            Button { pickingTape = true } label: {
                HStack {
                    if let tape = tapes.first(where: { $0.id == cab.shadowTape }) {
                        RoundedRectangle(cornerRadius: 3).fill(Color(nsColor: tape.displayColour)).frame(width: 24, height: 18)
                        Text(tape.label).lineLimit(2)
                    } else { Label("Shadow edge: match support board", systemImage: "rectangle.bottomthird.inset.filled") }
                    Spacer()
                    Image(systemName: "chevron.right")
                }
            }
            .popover(isPresented: $pickingTape, arrowEdge: .leading) {
                VStack(alignment: .leading, spacing: 10) {
                    Text("Shadow-line edgebanding").font(.headline)
                    Text("Swatches are approximate. Confirm the finish against a physical sample.").font(.caption).foregroundStyle(.secondary)
                    TextField("Search colour, finish or brand", text: $search).textFieldStyle(.roundedBorder)
                    Button("Match support board") { change { $0.clearShadowTape() }; pickingTape = false }
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 6) {
                            ForEach(tapes.filter { search.isEmpty || $0.label.localizedCaseInsensitiveContains(search) }, id: \.id) { tape in
                                Button {
                                    change { $0.setShadowTape(tape) }; pickingTape = false
                                } label: {
                                    HStack {
                                        RoundedRectangle(cornerRadius: 4).fill(Color(nsColor: tape.displayColour)).frame(width: 36, height: 24)
                                        Text(tape.label).frame(maxWidth: .infinity, alignment: .leading)
                                        if cab.shadowTape == tape.id { Image(systemName: "checkmark") }
                                    }.padding(6).contentShape(Rectangle())
                                }.buttonStyle(.plain)
                            }
                        }
                    }
                    if tapes.isEmpty { Text("No edgebanding is available in the material catalogue.").foregroundStyle(.secondary) }
                }.padding().frame(width: 480, height: 480)
            }
        }
        .task {
            if let dir = Preferences.catalogDir { tapes = (try? shadowTapes(catalogDir: dir.path)) ?? [] }
        }
    }

    private func dimension(_ title: String, value: Double, range: ClosedRange<Double>, set: @escaping (inout CabinetDoc, Double) -> Void) -> some View {
        HStack {
            Text(title).foregroundStyle(.secondary)
            Spacer()
            TextField(title, value: Binding(get: { value }, set: { v in change { set(&$0, min(range.upperBound, max(range.lowerBound, v))) } }), format: .number.precision(.fractionLength(0...1)))
                .multilineTextAlignment(.trailing).frame(width: 58).textFieldStyle(.roundedBorder)
            Text("mm").font(.caption).foregroundStyle(.secondary)
            Stepper(title, value: Binding(get: { value }, set: { v in change { set(&$0, v) } }), in: range, step: 5).labelsHidden()
        }
    }
}

extension ShadowTapeRecord {
    var displayColour: NSColor { NSColor(hex: colour) ?? BoardRecord.guess(decor: label, grained: false) }
}
