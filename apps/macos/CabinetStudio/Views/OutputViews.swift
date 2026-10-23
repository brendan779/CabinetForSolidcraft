import SwiftUI
import UniformTypeIdentifiers
import WebKit

struct CutListView: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    @State private var sort = [KeyPathComparator(\CutRowRecord.part)]
    @State private var exporting = false

    private var rows: [CutRowRecord] {
        ((try? cutListRows(job: project.jobItems)) ?? []).sorted(using: sort)
    }

    var body: some View {
        let rows = self.rows
        VStack(spacing: 0) {
            Table(rows, sortOrder: $sort) {
                TableColumn("Part", value: \.part) { Text($0.part).monospaced() }
                TableColumn("Description", value: \.description)
                TableColumn("Board", value: \.board)
                TableColumn("T", value: \.thickness) { Text(mm($0.thickness)) }.width(40)
                TableColumn("Length", value: \.length) { Text(mm($0.length)) }.width(70)
                TableColumn("Width", value: \.width) { Text(mm($0.width)) }.width(70)
                TableColumn("Cut L", value: \.cutLength) { Text(mm($0.cutLength)).foregroundStyle(.secondary) }.width(70)
                TableColumn("Cut W", value: \.cutWidth) { Text(mm($0.cutWidth)).foregroundStyle(.secondary) }.width(70)
                TableColumn("Edges", value: \.edges).width(90)
            }
            HStack {
                let area = rows.reduce(0) { $0 + $1.length * $1.width } / 1e6
                let tape = rows.reduce(0) { $0 + $1.edgeMetres }
                Text("\(rows.count) parts · \(String(format: "%.2f", area)) m² · \(String(format: "%.1f", tape)) m edge tape")
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Export CSV…", systemImage: "square.and.arrow.up") { exporting = true }
            }
            .padding(10)
            .background(.bar)
        }
        .fileExporter(isPresented: $exporting, document: TextDocument(text: csv), contentType: .commaSeparatedText,
                      defaultFilename: "\(project.jobName) cut list") { _ in }
    }

    private var csv: String { (try? cutListCsv(job: project.jobItems)) ?? "" }

    private func mm(_ v: Double) -> String { unit.format(v, suffix: false) }
}

extension CutRowRecord: Identifiable {
    public var id: String { part }
}

struct TextDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.plainText] }
    var text: String
    init(text: String) { self.text = text }
    init(configuration: ReadConfiguration) throws {
        text = String(decoding: configuration.file.regularFileContents ?? Data(), as: UTF8.self)
    }
    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: Data(text.utf8))
    }
}

struct CutSheetsView: View {
    @Environment(ProjectModel.self) private var project
    @State private var part: String?

    var body: some View {
        HSplitView {
            List(selection: $part) {
                ForEach(project.cabinets) { cab in
                    Section(cab.id) {
                        ForEach(project.build(cab)?.panels ?? [], id: \.name) { p in
                            Label(p.role, systemImage: p.isFront ? "door.left.hand.closed" : "rectangle")
                                .tag(p.name)
                        }
                    }
                }
            }
            .frame(minWidth: 200, maxWidth: 280)
            Group {
                if let part, let svg = try? cutSheetSvg(items: project.jobItems, partName: part, job: project.jobName) {
                    SVGView(svg: svg)
                } else {
                    ContentUnavailableView("Pick a part", systemImage: "doc.richtext",
                                           description: Text("Every part gets a dimensioned sheet with its bores, slots and edge tape."))
                }
            }
            .frame(minWidth: 500, maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}

struct SVGView: NSViewRepresentable {
    let svg: String

    func makeNSView(context: Context) -> WKWebView {
        let v = WKWebView()
        v.setValue(false, forKey: "drawsBackground")
        return v
    }

    func updateNSView(_ v: WKWebView, context: Context) {
        let html = """
        <html><head><meta name="viewport" content="width=device-width"><style>
        body{margin:0;display:flex;align-items:center;justify-content:center;height:100vh;background:transparent}
        svg{width:96%;height:auto;max-height:96vh;box-shadow:0 8px 30px rgba(0,0,0,.25);border-radius:4px}
        </style></head><body>\(svg)</body></html>
        """
        v.loadHTMLString(html, baseURL: nil)
    }
}

struct BoardLibraryView: View {
    @Environment(ProjectModel.self) private var project
    @State private var search = ""

    var body: some View {
        let boards = project.boards.filter {
            search.isEmpty || "\($0.brand) \($0.decor) \($0.finish)".localizedCaseInsensitiveContains(search)
        }
        ScrollView {
            if boards.isEmpty {
                ContentUnavailableView("No boards", systemImage: "square.stack.3d.up.slash",
                                       description: Text("Board data from Polytec, Laminex and others is added to catalog/boards."))
                    .padding(.top, 80)
            }
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 170), spacing: 16)], spacing: 16) {
                ForEach(boards, id: \.id) { b in
                    VStack(alignment: .leading, spacing: 8) {
                        RoundedRectangle(cornerRadius: 10)
                            .fill(Color(nsColor: b.displayColour))
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
                                .foregroundStyle(.white.opacity(0.8))
                                .shadow(radius: 1)
                                .padding(6)
                            }
                            .overlay {
                                if b.grained {
                                    GrainPattern().clipShape(RoundedRectangle(cornerRadius: 10))
                                }
                            }
                            .overlay(alignment: .bottomLeading) { usedFor(b) }
                            .frame(height: 110)
                            .shadow(color: .black.opacity(0.15), radius: 4, y: 2)
                            .accessibilityLabel("\(b.decor) swatch\(b.colourIsGuessed ? ", colour approximate" : "")")
                        HStack {
                            Text(b.decor).font(.headline)
                            Spacer()
                            useMenu(b)
                        }
                        Text("\(b.brand) · \(b.finish)").font(.subheadline).foregroundStyle(.secondary)
                        Text("\(Int(b.thickness)) mm \(b.substrate.replacingOccurrences(of: "_", with: " ")) · \(b.sheets.joined(separator: ", "))")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            .padding(20)
        }
        .searchable(text: $search, prompt: "Search decors")
    }

    private static let slots: [(id: String, title: String, symbol: String)] = [
        ("carcass", "Carcass", "square.split.bottomrightquarter"),
        ("front", "Fronts", "door.left.hand.closed"),
        ("back", "Back", "rectangle.portrait"),
    ]

    /// Assigns the board to a part of the selected cabinet.
    private func useMenu(_ b: BoardRecord) -> some View {
        Menu {
            ForEach(Self.slots, id: \.id) { slot in
                Button("Use for \(slot.title)", systemImage: slot.symbol) { project.useBoard(b, for: slot.id) }
            }
        } label: {
            Image(systemName: "plus.circle")
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .disabled(project.selected == nil)
        .help(project.selected.map { "Use this board for \($0.id)" } ?? "Select a cabinet first")
    }

    /// Badges for what the selected cabinet already uses this board for.
    @ViewBuilder
    private func usedFor(_ b: BoardRecord) -> some View {
        if let cab = project.selected {
            let used = Self.slots.filter { cab.board(for: $0.id) == b.id }
            if !used.isEmpty {
                HStack(spacing: 4) {
                    ForEach(used, id: \.id) { slot in
                        Label(slot.title, systemImage: slot.symbol)
                            .font(.caption2.weight(.semibold))
                            .padding(.horizontal, 6).padding(.vertical, 3)
                            .background(.regularMaterial, in: Capsule())
                    }
                }
                .padding(6)
            }
        }
    }
}

struct GrainPattern: View {
    var body: some View {
        Canvas { ctx, size in
            var y: CGFloat = 4
            var i = 0
            while y < size.height {
                var p = Path()
                p.move(to: CGPoint(x: 0, y: y))
                p.addCurve(to: CGPoint(x: size.width, y: y + CGFloat(i % 3)),
                           control1: CGPoint(x: size.width * 0.3, y: y - 3),
                           control2: CGPoint(x: size.width * 0.7, y: y + 4))
                ctx.stroke(p, with: .color(.black.opacity(0.07)), lineWidth: 1)
                y += CGFloat(5 + (i * 7) % 6)
                i += 1
            }
        }
        .accessibilityHidden(true)
    }
}

extension NSColor {
    convenience init?(hex: String) {
        let s = hex.trimmingCharacters(in: CharacterSet(charactersIn: "#"))
        guard s.count == 6, let v = UInt32(s, radix: 16) else { return nil }
        self.init(srgbRed: CGFloat((v >> 16) & 0xff) / 255, green: CGFloat((v >> 8) & 0xff) / 255,
                  blue: CGFloat(v & 0xff) / 255, alpha: 1)
    }
}

extension Color {
    init?(hex: String) {
        let s = hex.trimmingCharacters(in: CharacterSet(charactersIn: "#"))
        guard s.count == 6, let v = UInt32(s, radix: 16) else { return nil }
        self.init(red: Double((v >> 16) & 0xff) / 255, green: Double((v >> 8) & 0xff) / 255, blue: Double(v & 0xff) / 255)
    }
}
