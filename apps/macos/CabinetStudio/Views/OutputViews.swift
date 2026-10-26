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
                    Text("Sheets").tag("Sheets")
                    Text("Hardware").tag("Hardware")
                    Text("Quote").tag("Quote")
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .fixedSize()
                .help("Parts to cut, the parts nested on stock sheets, hardware to order, or what the job costs")
                Spacer()
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(.bar)
            switch showing {
            case "Parts": PartsListView()
            case "Sheets": NestingView()
            case "Quote": QuoteView()
            default: HardwareListView()
            }
        }
    }
}

/// Offcuts kept from earlier jobs, as the core's JSON, in Application Support/CabinetStudio.
enum OffcutStore {
    static var url: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appending(path: "CabinetStudio/offcuts.json")
    }

    static func load() -> String { (try? String(contentsOf: url, encoding: .utf8)) ?? "" }

    static func save(_ json: String) throws {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(),
                                                withIntermediateDirectories: true)
        try json.write(to: url, atomically: true, encoding: .utf8)
    }

    static func count(_ json: String) -> Int {
        ((try? JSONSerialization.jsonObject(with: Data(json.utf8))) as? [Any])?.count ?? 0
    }
}

/// The parts nested on stock sheets for the panel saw: a summary page, then each sheet to scale.
struct NestingView: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage("nestingKerf") private var kerf = 4.4
    @AppStorage("nestingTrim") private var trim = 10.0
    @AppStorage("nestingUseOffcuts") private var useOffcuts = true
    @State private var stored = OffcutStore.load()
    @State private var page = 0
    @State private var export: DataDocument?

    private var offcuts: String { useOffcuts ? stored : "" }

    private var pages: [String] {
        guard let dir = Preferences.catalogDir?.path else { return [] }
        return (try? nestingPages(items: project.jobItems, catalogDir: dir, job: project.jobName,
                                  kerf: kerf, trim: trim, offcutsJson: offcuts)) ?? []
    }

    var body: some View {
        let pages = self.pages
        VStack(spacing: 0) {
            Group {
                if pages.isEmpty {
                    ContentUnavailableView("No parts to nest", systemImage: "rectangle.split.3x3",
                                           description: Text("Add cabinets to see their parts laid out on sheets."))
                } else {
                    SVGView(svg: pages[min(page, pages.count - 1)])
                }
            }
            .frame(minWidth: 0, maxWidth: .infinity, maxHeight: .infinity)
            VStack(spacing: 6) {
                HStack(spacing: 10) {
                    Button("Previous", systemImage: "chevron.left") { page = max(0, page - 1) }
                        .labelStyle(.iconOnly)
                        .disabled(page == 0)
                    Text(pages.isEmpty ? "" : (page == 0 ? "Summary" : "Sheet \(page) of \(pages.count - 1)"))
                        .monospacedDigit()
                    Button("Next", systemImage: "chevron.right") { page = min(pages.count - 1, page + 1) }
                        .labelStyle(.iconOnly)
                        .disabled(page >= pages.count - 1)
                    Spacer(minLength: 0)
                    Button("Export PDF…", systemImage: "doc.richtext") { makePdf() }
                        .help("The summary and every sheet as a PDF")
                }
                HStack(spacing: 10) {
                    Stepper("Kerf \(String(format: "%.1f", kerf))", value: $kerf, in: 1...8, step: 0.2)
                        .help("Saw kerf, mm")
                    Stepper("Trim \(Int(trim))", value: $trim, in: 0...25, step: 1)
                        .help("Taken off every new sheet's edges before the first cut, mm")
                    Spacer(minLength: 0)
                    Menu("Offcuts (\(OffcutStore.count(stored)))") {
                        Toggle("Nest on Kept Offcuts First", isOn: $useOffcuts)
                        Button("Keep This Job's Offcuts") { keepOffcuts() }
                        Divider()
                        Button("Forget Kept Offcuts", role: .destructive) { forget() }
                    }
                    .fixedSize()
                    .help("Offcuts kept from earlier jobs: nest on them first, and after cutting this job keep its leftovers (the kept offcuts it used are taken off)")
                }
            }
            .padding(10)
            .frame(minWidth: 0, maxWidth: .infinity)
            .background(.bar)
        }
        .onChange(of: pages.count) { page = min(page, max(0, pages.count - 1)) }
        .fileExporter(isPresented: Binding(get: { export != nil }, set: { if !$0 { export = nil } }),
                      document: export, contentType: .pdf,
                      defaultFilename: "\(project.jobName) nesting") { result in
            if case .success(let url) = result { project.status = "Saved \(url.lastPathComponent)" }
        }
    }

    private func makePdf() {
        guard let dir = Preferences.catalogDir?.path else { return }
        do {
            export = DataDocument(data: try nestingPdf(items: project.jobItems, catalogDir: dir,
                                                      job: project.jobName, kerf: kerf, trim: trim,
                                                      offcutsJson: offcuts))
        } catch {
            project.lastError = "Could not nest the parts: \(error)"
        }
    }

    /// After cutting this job: its leftovers join the store, the kept offcuts it used leave it.
    private func keepOffcuts() {
        guard let dir = Preferences.catalogDir?.path else { return }
        do {
            let kept = try nestingKeptOffcuts(items: project.jobItems, catalogDir: dir, kerf: kerf,
                                              trim: trim, offcutsJson: offcuts)
            // Offcuts not offered to this nesting stay as they were.
            let json = useOffcuts ? kept : merge(stored, kept)
            try OffcutStore.save(json)
            stored = json
            project.status = "Keeping \(OffcutStore.count(json)) offcuts"
        } catch {
            project.lastError = "Could not keep the offcuts: \(error)"
        }
    }

    private func forget() {
        try? OffcutStore.save("")
        stored = ""
    }

    private func merge(_ a: String, _ b: String) -> String {
        let parse = { (s: String) in ((try? JSONSerialization.jsonObject(with: Data(s.utf8))) as? [Any]) ?? [] }
        let all = parse(a) + parse(b)
        let data = (try? JSONSerialization.data(withJSONObject: all)) ?? Data("[]".utf8)
        return String(decoding: data, as: UTF8.self)
    }
}

/// The shop's price list, as the core's JSON, in Application Support/CabinetStudio.
enum PriceStore {
    static var url: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appending(path: "CabinetStudio/prices.json")
    }

    static func load() -> [String: Any] {
        guard let data = try? Data(contentsOf: url) else { return [:] }
        return (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] ?? [:]
    }

    static func save(_ prices: [String: Any]) throws {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(),
                                                withIntermediateDirectories: true)
        try JSONSerialization.data(withJSONObject: prices, options: [.prettyPrinted, .sortedKeys]).write(to: url)
    }

    static func json(_ prices: [String: Any]) -> String {
        String(decoding: (try? JSONSerialization.data(withJSONObject: prices)) ?? Data("{}".utf8), as: UTF8.self)
    }
}

/// What the job costs: board by the sheets the nesting opens, edge tape, hardware and labour from
/// the shop's prices, which are typed in here against each line.
struct QuoteView: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage("nestingKerf") private var kerf = 4.4
    @AppStorage("nestingTrim") private var trim = 10.0
    @AppStorage("nestingUseOffcuts") private var useOffcuts = true
    @State private var prices = PriceStore.load()
    @State private var export: DataDocument?
    @State private var showingRates = false

    private var quote: QuoteRecord? {
        guard let dir = Preferences.catalogDir?.path else { return nil }
        return try? jobQuote(items: project.jobItems, catalogDir: dir, kerf: kerf, trim: trim,
                             offcutsJson: useOffcuts ? OffcutStore.load() : "",
                             pricesJson: PriceStore.json(prices))
    }

    var body: some View {
        let q = quote
        VStack(spacing: 0) {
            Table(q?.lines ?? []) {
                TableColumn("Description", value: \.description)
                TableColumn("Qty") { Text(String(format: "%g", $0.quantity)).monospacedDigit() }.width(60)
                TableColumn("Unit", value: \.unit).width(40)
                TableColumn("Price") { line in
                    TextField("Price", value: price(line), format: .number.precision(.fractionLength(0...2)),
                              prompt: Text("—"))
                        .multilineTextAlignment(.trailing)
                        .monospacedDigit()
                }
                .width(80)
                TableColumn("Total") { Text(String(format: "%.2f", $0.total)).monospacedDigit() }.width(80)
            }
            HStack(spacing: 12) {
                if let q {
                    Text("Cost \(money(q.cost)) · markup \(money(q.markup)) · tax \(money(q.tax)) · **\(q.currency) \(money(q.total))**")
                        .monospacedDigit()
                    if !q.missing.isEmpty {
                        Text("\(q.missing.count) without a price").foregroundStyle(.orange)
                    }
                }
                Spacer(minLength: 0)
                Button("Rates…", systemImage: "slider.horizontal.3") { showingRates = true }
                    .labelStyle(.iconOnly)
                    .help("Markup, tax and the labour times per part, cabinet and hardware item")
                    .popover(isPresented: $showingRates, arrowEdge: .top) { rates }
                Menu("Export", systemImage: "doc.richtext") {
                    Button("Costing PDF…") { makePdf(customer: false) }
                    Button("Customer Quote PDF…") { makePdf(customer: true) }
                }
                .fixedSize()
                .help("The costing line by line, or the customer's quote: a price per cabinet, tax included")
            }
            .padding(10)
            .frame(minWidth: 0, maxWidth: .infinity)
            .background(.bar)
        }
        .fileExporter(isPresented: Binding(get: { export != nil }, set: { if !$0 { export = nil } }),
                      document: export, contentType: .pdf,
                      defaultFilename: "\(project.jobName) quote") { result in
            if case .success(let url) = result { project.status = "Saved \(url.lastPathComponent)" }
        }
    }

    private var rates: some View {
        Form {
            Stepper("Markup \(Int((number("markup") * 100).rounded())) %", value: rate("markup"), in: 0...3, step: 0.05)
            Stepper("Tax \(Int((number("tax", or: 0.1) * 100).rounded())) %", value: rate("tax", or: 0.1), in: 0...0.3, step: 0.01)
            Stepper("Per part \(Int(number("minutes_per_part", or: 6))) min",
                    value: rate("minutes_per_part", or: 6), in: 0...60, step: 1)
            Stepper("Per cabinet \(Int(number("minutes_per_cabinet", or: 30))) min",
                    value: rate("minutes_per_cabinet", or: 30), in: 0...240, step: 5)
            Stepper("Per hardware item \(Int(number("minutes_per_hardware_item", or: 2))) min",
                    value: rate("minutes_per_hardware_item", or: 2), in: 0...30, step: 1)
        }
        .padding()
        .frame(width: 300)
    }

    private func money(_ v: Double) -> String { String(format: "%.2f", v) }

    /// A rate from the price list, or the core's default when it has none yet.
    private func number(_ key: String, or fallback: Double = 0) -> Double {
        (prices[key] as? NSNumber)?.doubleValue ?? fallback
    }

    private func rate(_ key: String, or fallback: Double = 0) -> Binding<Double> {
        Binding(get: { number(key, or: fallback) }, set: { prices[key] = $0; save() })
    }

    /// A line's price, kept where the core reads it: per m² by board, each by hardware item, or the
    /// tape and labour rates.
    private func price(_ line: QuoteLineRecord) -> Binding<Double?> {
        let (table, key): (String?, String) = switch line.section {
        case "board", "benchtop": ("boards", line.key)
        case "hardware": ("hardware", line.key)
        case "edge_tape": (nil, "edge_tape_per_m")
        default: (nil, "labour_per_hour")
        }
        return Binding(get: { line.unitPrice }, set: { v in
            if let table {
                var t = prices[table] as? [String: Any] ?? [:]
                t[key] = v
                prices[table] = t
            } else {
                prices[key] = v ?? NSNull()
            }
            save()
        })
    }

    private func save() {
        do { try PriceStore.save(prices) } catch { project.lastError = "Could not save the prices: \(error)" }
    }

    private func makePdf(customer: Bool) {
        guard let dir = Preferences.catalogDir?.path else { return }
        let offcuts = useOffcuts ? OffcutStore.load() : ""
        do {
            let data = customer
                ? try customerQuotePdf(items: project.jobItems, catalogDir: dir, job: project.jobName,
                                       kerf: kerf, trim: trim, offcutsJson: offcuts, pricesJson: PriceStore.json(prices))
                : try quotePdf(items: project.jobItems, catalogDir: dir, job: project.jobName,
                               kerf: kerf, trim: trim, offcutsJson: offcuts, pricesJson: PriceStore.json(prices))
            export = DataDocument(data: data)
        } catch {
            project.lastError = "Could not make the quote: \(error)"
        }
    }
}

extension QuoteLineRecord: Identifiable {
    public var id: String { section + "|" + key + "|" + description }
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
                    .help("The parts laid out on stock sheets for the panel saw, with the offcuts, as a PDF (kerf, trim and kept offcuts as set under Sheets)")
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
            // The Sheets view's settings.
            let d = UserDefaults.standard
            let kerf = d.object(forKey: "nestingKerf") as? Double ?? 4.4
            let trim = d.object(forKey: "nestingTrim") as? Double ?? 10
            let use = d.object(forKey: "nestingUseOffcuts") as? Bool ?? true
            let data = try nestingPdf(items: project.jobItems, catalogDir: dir, job: project.jobName,
                                      kerf: kerf, trim: trim, offcutsJson: use ? OffcutStore.load() : "")
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

    /// Writes every part's DXF into a new "<job> DXF" folder in the folder the user picks.
    private func exportDxf() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.prompt = "Export Here"
        panel.message = "Choose where the \(project.jobName) DXF folder goes"
        guard panel.runModal() == .OK, let base = panel.url else { return }
        do {
            let files = try jobDxfs(items: project.jobItems)
            let folder = base.appending(path: "\(project.jobName) DXF", directoryHint: .isDirectory)
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            for f in files {
                try Data(f.contents.utf8).write(to: folder.appending(path: "\(f.name).dxf"))
            }
            project.status = "Saved \(files.count) DXF files in \(folder.lastPathComponent)"
        } catch {
            project.lastError = "Could not export the DXF files: \(error)"
        }
    }

    /// Export buttons in a bar over the sheet (above the status bar's reach). They used to be window toolbar items, which crashed
    /// the app: the toolbar had no room left and its layout became unsatisfiable.
    private var actions: some View {
        // Four labelled buttons need ~430 pt; narrower, they show their icons only.
        ViewThatFits(in: .horizontal) {
            buttons.labelStyle(.titleAndIcon)
            buttons.labelStyle(.iconOnly)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .frame(minWidth: 0, maxWidth: .infinity)
        .background(.bar)
    }

    private var buttons: some View {
        HStack {
            Spacer(minLength: 0)
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
            if project.cabinets.contains(where: { $0.kind == "benchtop" }) {
                Button("Benchtops…", systemImage: "rectangle.tophalf.filled") {
                    do {
                        pdf = (DataDocument(data: try benchtopsPdf(items: project.jobItems, job: project.jobName)),
                               "\(project.jobName) benchtops")
                    } catch {
                        project.lastError = "Could not make the benchtop schedule: \(error)"
                    }
                }
                .help("For the benchtop maker: every piece's size, area, finished edges and cut-outs, then each piece's sheet")
            }
            Button("DXF…", systemImage: "square.and.arrow.up.on.square") { exportDxf() }
                .help("A DXF per part for CNC (outline, bores by size and depth, pockets, cut-outs on their own layers) into a folder you choose")
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
