import SwiftUI
import UniformTypeIdentifiers
import WebKit

/// The Cut List workspace: the parts to cut, or the hardware to order.
struct CutListView: View {
    @State private var showing = "Parts"

    // Workspace controls live in the view, not the window toolbar: the toolbar is already full,
    // and adding items to it made AppKit's title bar layout unsatisfiable and crash.
    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Picker("Show", selection: $showing) {
                    Text("Parts").tag("Parts")
                    Text("Hardware").tag("Hardware")
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .fixedSize()
                .help("Parts to cut, or hardware to order")
                Spacer()
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(.bar)
            if showing == "Parts" { PartsListView() } else { HardwareListView() }
        }
    }
}

/// Every hardware item of the job, counted, with its supplier article where the catalog has it.
struct HardwareListView: View {
    @Environment(ProjectModel.self) private var project
    @State private var export: (doc: DataDocument, type: UTType, name: String)?

    private var rows: [BomRecord] {
        guard let dir = Preferences.catalogDir else { return [] }
        return (try? hardwareBom(items: project.jobItems, catalogDir: dir.path)) ?? []
    }

    var body: some View {
        let rows = self.rows
        VStack(spacing: 0) {
            Table(rows) {
                TableColumn("Qty") { Text("\($0.quantity)").monospacedDigit() }.width(40)
                TableColumn("Description", value: \.description)
                TableColumn("Brand", value: \.brand).width(80)
                TableColumn("Article") { r in
                    Text(r.article).foregroundStyle(r.brand.isEmpty ? .secondary : .primary)
                }
                .width(110)
                TableColumn("Item") { Text($0.item).monospaced().foregroundStyle(.secondary) }
                TableColumn("Cabinets", value: \.cabinets).width(120)
            }
            HStack {
                let missing = rows.filter { $0.brand.isEmpty }.count
                Text("\(rows.reduce(0) { $0 + Int($1.quantity) }) items in \(rows.count) lines"
                     + (missing > 0 ? " · \(missing) without an article number yet" : ""))
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Export CSV…", systemImage: "square.and.arrow.up") { make(pdf: false) }
                Button("Export PDF…", systemImage: "doc.richtext") { make(pdf: true) }
            }
            .padding(10)
            .background(.bar)
        }
        .fileExporter(isPresented: Binding(get: { export != nil }, set: { if !$0 { export = nil } }),
                      document: export?.doc, contentType: export?.type ?? .data,
                      defaultFilename: export?.name ?? "") { _ in }
    }

    private func make(pdf: Bool) {
        guard let dir = Preferences.catalogDir?.path else { return }
        do {
            let name = "\(project.jobName) hardware"
            if pdf {
                let data = try hardwareBomPdf(items: project.jobItems, catalogDir: dir, job: project.jobName)
                export = (DataDocument(data: data), .pdf, name)
            } else {
                let csv = try hardwareBomCsv(items: project.jobItems, catalogDir: dir)
                export = (DataDocument(data: Data(csv.utf8)), .commaSeparatedText, name)
            }
        } catch {
            project.lastError = "Could not export the hardware: \(error)"
        }
    }
}

extension BomRecord: Identifiable {
    public var id: String { item + "|" + description }
}

struct PartsListView: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    @State private var sort = [KeyPathComparator(\CutRowRecord.part)]
    @State private var export: (doc: DataDocument, type: UTType, name: String)?

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
                Button("Nesting…", systemImage: "rectangle.split.3x3") { makeNesting() }
                    .help("The parts laid out on stock sheets for the panel saw (4.4 mm kerf, 10 mm trim), with the offcuts, as a PDF")
                Button("Export CSV…", systemImage: "square.and.arrow.up") {
                    export = (DataDocument(data: Data(csv.utf8)), .commaSeparatedText, "\(project.jobName) cut list")
                }
            }
            .padding(10)
            .background(.bar)
        }
        .fileExporter(isPresented: Binding(get: { export != nil }, set: { if !$0 { export = nil } }),
                      document: export?.doc, contentType: export?.type ?? .data,
                      defaultFilename: export?.name ?? "") { result in
            if case .success(let url) = result { project.status = "Saved \(url.lastPathComponent)" }
        }
    }

    private func makeNesting() {
        guard let dir = Preferences.catalogDir?.path else { return }
        do {
            let data = try nestingPdf(items: project.jobItems, catalogDir: dir, job: project.jobName,
                                      kerf: 4.4, trim: 10)
            export = (DataDocument(data: data), .pdf, "\(project.jobName) nesting")
        } catch {
            project.lastError = "Could not nest the parts: \(error)"
        }
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

/// Bytes to save through a file exporter, typed by the exporter's content type.
struct DataDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.data] }
    var data: Data
    init(data: Data) { self.data = data }
    init(configuration: ReadConfiguration) throws {
        data = configuration.file.regularFileContents ?? Data()
    }
    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: data)
    }
}

struct CutSheetsView: View {
    @Environment(ProjectModel.self) private var project
    @State private var part: String?
    @State private var pdf: (doc: DataDocument, name: String)?

    var body: some View {
        // A plain HStack, not HSplitView: a nested NSSplitView beside the inspector sent AppKit into
        // an endless constraint-update loop in full screen, which crashed the app.
        HStack(spacing: 0) {
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
            .frame(width: 220)
            Divider()
            VStack(spacing: 0) {
                actions
                Group {
                    if let part, let svg = try? cutSheetSvg(items: project.jobItems, partName: part, job: project.jobName) {
                        SVGView(svg: svg)
                    } else {
                        ContentUnavailableView("Pick a part", systemImage: "doc.richtext",
                                               description: Text("Every part gets a dimensioned sheet with its bores, slots and edge tape."))
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .fileExporter(isPresented: Binding(get: { pdf != nil }, set: { if !$0 { pdf = nil } }),
                      document: pdf?.doc, contentType: .pdf,
                      defaultFilename: pdf?.name ?? "") { result in
            if case .success(let url) = result { project.status = "Saved \(url.lastPathComponent)" }
        }
    }

    /// Export buttons in a bar over the sheet (above the status bar's reach). They used to be window toolbar items, which crashed
    /// the app: the toolbar had no room left and its layout became unsatisfiable.
    private var actions: some View {
        HStack {
            Spacer()
                Button("Labels…", systemImage: "qrcode") {
                    do {
                        pdf = (DataDocument(data: try partLabelsPdf(items: project.jobItems, job: project.jobName)),
                               "\(project.jobName) labels")
                    } catch {
                        project.lastError = "Could not make the labels: \(error)"
                    }
                }
                .help("A 62 × 40 mm label per part with a QR code of its name, for a 62 mm label printer")
                Button("Assembly…", systemImage: "square.3.layers.3d.down.right") {
                    do {
                        pdf = (DataDocument(data: try assemblyPdf(items: project.jobItems, job: project.jobName)),
                               "\(project.jobName) assembly")
                    } catch {
                        project.lastError = "Could not make the assembly sheets: \(error)"
                    }
                }
                .help("Each cabinet exploded, with its parts list, one page per cabinet")
                Button("Export PDF…", systemImage: "doc.richtext") {
                    do {
                        pdf = (DataDocument(data: try cutSheetsPdf(items: project.jobItems, job: project.jobName)),
                               "\(project.jobName) cut sheets")
                    } catch {
                        project.lastError = "Could not make the PDF: \(error)"
                    }
                }
                .help("Every part's sheet in one PDF, after a cover and an index")
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(.bar)
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
