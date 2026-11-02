import SwiftUI

/// One ABI family size: a model (article) and the finishes it comes in.
struct AbiHandleSize: Identifiable, Equatable {
    let article: String
    /// ABI's named overall length, which is not where the screws go.
    let length: Double
    let centres: Double
    let knob: Bool
    let finishes: [HandleRecord]

    var id: String { article }
    var label: String { knob ? "Knob \(Int(length.rounded()))" : "\(Int(length.rounded()))" }

    /// This size in the finish a job already has, or its first finish.
    func best(for finish: String?) -> HandleRecord? {
        finishes.first { $0.variant == finish } ?? finishes.first
    }
    static func == (l: AbiHandleSize, r: AbiHandleSize) -> Bool { l.article == r.article }
}

/// One ABI family: its sizes, smallest first, each with its finishes.
struct AbiHandleFamily: Identifiable, Equatable {
    let name: String
    let sizes: [AbiHandleSize]

    var id: String { name }
    /// The family's own name without the brand prefix ("Tezra" from "ABI Tezra").
    var shortName: String { name.hasPrefix("ABI ") ? String(name.dropFirst(4)) : name }
    static func == (l: AbiHandleFamily, r: AbiHandleFamily) -> Bool { l.name == r.name }
}

/// The ABI records grouped the picker works in: family tiles → size chips → finish swatches.
enum AbiHandleCatalog {
    static func families(_ records: [HandleRecord]) -> [AbiHandleFamily] {
        var byFamily: [String: [HandleRecord]] = [:]
        for r in records { byFamily[r.family, default: []].append(r) }
        return byFamily.map { family, rows in
            var byArticle: [String: [HandleRecord]] = [:]
            for r in rows { byArticle[r.article, default: []].append(r) }
            let sizes = byArticle.map { article, finishes -> AbiHandleSize in
                let first = finishes[0]
                return AbiHandleSize(article: article, length: first.length, centres: first.centres, knob: first.knob,
                                     finishes: finishes.sorted { $0.variant < $1.variant })
            }
            .sorted { a, b in
                if a.knob != b.knob { return !a.knob }
                return a.length < b.length
            }
            return AbiHandleFamily(name: family, sizes: sizes)
        }
        .sorted { $0.name < $1.name }
    }

    /// Where a chosen record id sits in the picker: its family and size.
    static func locate(_ id: String, in families: [AbiHandleFamily]) -> (family: AbiHandleFamily, size: AbiHandleSize)? {
        for f in families {
            if let s = f.sizes.first(where: { $0.finishes.contains { $0.id == id } }) {
                return (f, s)
            }
        }
        return nil
    }
}

/// The chosen ABI handle as the inspector shows it: a swatch, the style and its price; the click
/// opens family tiles, size chips and finish swatches.
struct AbiHandleSection: View {
    @Environment(ProjectModel.self) private var project
    let cab: CabinetDoc
    @State private var picking = false

    private var records: [HandleRecord] { project.handleRecords }
    private var families: [AbiHandleFamily] { AbiHandleCatalog.families(records) }
    /// The chosen record, and where it sits in the picker.
    private var chosen: (record: HandleRecord, place: (family: AbiHandleFamily, size: AbiHandleSize))? {
        guard let id = project.handleStyleId(of: cab) else { return nil }
        guard let record = records.first(where: { $0.id == id }) else { return nil }
        return (record, AbiHandleCatalog.locate(id, in: families) ?? fallbackPlace)
    }
    /// The first place to sit on before any choice: the first family's mid size.
    private var fallbackPlace: (family: AbiHandleFamily, size: AbiHandleSize) {
        let f = families.first ?? AbiHandleFamily(name: "", sizes: [])
        let s = f.sizes.first(where: { !$0.knob }) ?? f.sizes.first
        return (f, s ?? AbiHandleSize(article: "", length: 0, centres: 0, knob: false, finishes: []))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionTitle("ABI handle")
            Button {
                picking = true
            } label: {
                HStack(spacing: 10) {
                    HandlePhoto(record: chosen?.record, others: chosen?.place.size.finishes ?? [])
                        .frame(width: 44, height: 44)
                    VStack(alignment: .leading, spacing: 1) {
                        Text(title).lineLimit(1)
                        Text(subtitle).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    }
                    Spacer()
                    Image(systemName: "chevron.right").foregroundStyle(.tertiary)
                }
                .padding(6)
                .contentShape(Rectangle())
            }
            .buttonStyle(TileStyle(selected: picking))
            .accessibilityLabel(chosen.map { "Handle: \($0.record.family), \($0.record.variant)" }
                                ?? "Choose an ABI handle")
            .popover(isPresented: $picking, arrowEdge: .leading) {
                AbiHandlePicker(families: families, chosenId: chosen?.record.id,
                                place: chosen?.place ?? fallbackPlace) { finish in
                    project.useHandle(finish)
                }
                .frame(width: 560, height: 560)
            }
        }
    }

    private var title: String {
        chosen.map { "\($0.record.family) \($0.record.sizeName)" } ?? "Choose a handle"
    }

    private var subtitle: String {
        chosen.map { c in
            [c.record.variant, c.record.fixingName,
             c.record.priceAud > 0 ? c.record.priceAud.formatted(.currency(code: "AUD")) : ""]
                .filter { !$0.isEmpty }.joined(separator: " · ")
        } ?? "Every family, size and finish"
    }
}

extension HandleRecord {
    /// "220 mm", or "knob" for a knob.
    var sizeName: String { knob ? "knob" : "\(Int(length.rounded())) mm" }

    /// Where the screws go, which is what a maker bores: ABI's name gives the length.
    var fixingName: String {
        switch screws {
        case 0, 1: "1 screw"
        case 2: "screws \(centres.formatted(.number.precision(.fractionLength(0...1)))) apart"
        default: "\(screws) screws over \(Int(centres))"
        }
    }
}

/// ABI's own photo of a handle in a finish, from `vendor-cad/abi-web` (see
/// `scripts/fetch_abi_photos.py`). Before those are downloaded, the finish's colour.
struct HandlePhoto: View {
    let record: HandleRecord?
    /// Other finishes of the same size, for a finish whose page had no photo: the shape still
    /// shows, which is what a family tile is for.
    var others: [HandleRecord] = []

    /// Loaded photos by row id; nil for one that is not there, so it is not retried each draw.
    @MainActor private static var cache: [String: NSImage?] = [:]
    @MainActor private static func photo(_ id: String) -> NSImage? {
        if let known = cache[id] { return known }
        let path = Preferences.vendorCADDir?.appending(path: "abi-web/\(id).jpg").path
        let image = path.flatMap { NSImage(contentsOfFile: $0) }
        cache[id] = .some(image)
        return image
    }

    var body: some View {
        if let image = ([record].compactMap { $0 } + others).lazy.compactMap({ Self.photo($0.id) }).first {
            // ABI shoots on white; the tile keeps that, not the window's dark background.
            Image(nsImage: image)
                .resizable()
                .interpolation(.high)
                .scaledToFit()
                .background(.white)
                .clipShape(RoundedRectangle(cornerRadius: 6))
                .accessibilityHidden(true)
        } else {
            FinishSwatch(colour: record?.colour ?? "", size: 30)
        }
    }
}

/// Family tiles as ABI's photos, size chips and large finish samples. Every tap is the job's choice at
/// once: a new family or size keeps the finish where it comes in that finish, so what the picker
/// shows is always what the job has.
struct AbiHandlePicker: View {
    let families: [AbiHandleFamily]
    let chosenId: String?
    /// Where the picker sits before any choice: the job's, or the first family's mid size.
    let place: (family: AbiHandleFamily, size: AbiHandleSize)
    let onPick: (HandleRecord) -> Void

    @State private var family: AbiHandleFamily?
    @State private var size: AbiHandleSize?

    /// The family and size the picker shows now.
    private var current: (family: AbiHandleFamily, size: AbiHandleSize) {
        if let family, let size, family.sizes.contains(size) { return (family, size) }
        if let family, size == nil, let s = place.family.sizes.first(where: { $0 == place.size }) {
            return (family, s)
        }
        return place
    }

    var body: some View {
        let now = current
        VStack(alignment: .leading, spacing: 0) {
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    familyTiles(selected: now.family)
                    sizeChips(now.family, selected: now.size)
                    finishSwatches(now.size)
                }
                .padding(14)
            }
            Divider()
            footer(now)
        }
        .background(Theme.panel)
    }

    private func familyTiles(selected family: AbiHandleFamily) -> some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 124), spacing: 10)], spacing: 10) {
            ForEach(families) { f in
                // The tile shows the family's middle pull (not a knob or its shortest T pull).
                let pulls = f.sizes.filter { !$0.knob }
                let mid = pulls.isEmpty ? f.sizes.first : pulls[pulls.count / 2]
                Button {
                    withAnimation(.snappy) {
                        self.family = f
                        self.size = mid
                    }
                    if let record = mid?.best(for: chosenFinish) { onPick(record) }
                } label: {
                    VStack(spacing: 5) {
                        HandlePhoto(record: mid?.best(for: chosenFinish), others: mid?.finishes ?? [])
                            .frame(height: 84)
                            .frame(maxWidth: .infinity)
                        Text(f.shortName).font(.caption).lineLimit(1)
                    }
                    .padding(6)
                }
                .buttonStyle(TileStyle(selected: f == family))
                .accessibilityLabel("Handle family \(f.shortName)")
            }
        }
    }

    /// The finish the job has now, which a new family or size keeps where it can.
    private var chosenFinish: String? {
        families.lazy.flatMap(\.sizes).flatMap(\.finishes).first { $0.id == chosenId }?.variant
    }

    private func sizeChips(_ family: AbiHandleFamily, selected size: AbiHandleSize) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("\(family.shortName) size (mm long)")
                .font(.caption).foregroundStyle(.secondary)
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(family.sizes) { s in
                        Button(s.label) {
                            withAnimation(.snappy) { self.size = s }
                            if let record = s.best(for: chosenFinish) { onPick(record) }
                        }
                        .buttonStyle(ChipStyle(selected: s == size))
                        .accessibilityLabel(s.knob ? "\(family.shortName) knob"
                                             : "\(family.shortName), \(s.label) mm long")
                    }
                }
            }
        }
    }

    private func finishSwatches(_ size: AbiHandleSize) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Finish").font(.caption).foregroundStyle(.secondary)
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 92), spacing: 8)], spacing: 8) {
                ForEach(size.finishes, id: \.id) { finish in
                    Button {
                        onPick(finish)
                    } label: {
                        VStack(spacing: 4) {
                            FinishSample(record: finish)
                                .frame(width: 64, height: 64)
                            Text(finish.variant.replacingOccurrences(of: " (colour to verify)", with: ""))
                                .font(.caption2).lineLimit(1).minimumScaleFactor(0.8)
                            Text(finish.priceAud > 0 ? finish.priceAud.formatted(.currency(code: "AUD")) : "—")
                                .font(.caption2).foregroundStyle(.secondary)
                        }
                        .padding(6)
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(TileStyle(selected: finish.id == chosenId))
                    .accessibilityLabel("\(finish.variant), "
                        + (finish.priceAud > 0 ? finish.priceAud.formatted(.currency(code: "AUD")) : "price not listed")
                        + (finish.colour.isEmpty ? ", colour to verify" : ""))
                }
            }
        }
    }

    private func footer(_ now: (family: AbiHandleFamily, size: AbiHandleSize)) -> some View {
        HStack {
            if let finish = now.size.finishes.first(where: { $0.id == chosenId }) {
                Text("\(finish.family) \(finish.sizeName), \(finish.fixingName), \(finish.variant)")
                    .lineLimit(1).minimumScaleFactor(0.8)
                Spacer()
                Text("SKU \(finish.sku)"
                     + (finish.priceAud > 0 ? " · \(finish.priceAud.formatted(.currency(code: "AUD"))) each" : ""))
                    .foregroundStyle(.secondary)
            } else {
                Text("Pick a handle").foregroundStyle(.secondary)
                Spacer()
            }
        }
        .font(.callout)
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
        .background(Theme.strip)
    }
}

/// A finish as a large round sample, so its colour reads: ABI's own swatch image, from
/// `vendor-cad/abi-web/swatches`, or its colour.
struct FinishSample: View {
    let record: HandleRecord

    @MainActor private static var cache: [String: NSImage?] = [:]
    @MainActor private static func swatch(_ finish: String) -> NSImage? {
        if let known = cache[finish] { return known }
        let slug = finish.lowercased().split(whereSeparator: { !$0.isLetter && !$0.isNumber }).joined(separator: "-")
        let path = Preferences.vendorCADDir?.appending(path: "abi-web/swatches/\(slug).png").path
        let image = path.flatMap { NSImage(contentsOfFile: $0) }
        cache[finish] = .some(image)
        return image
    }

    var body: some View {
        Group {
            if let image = Self.swatch(record.variant) {
                Image(nsImage: image).resizable().interpolation(.high).scaledToFill()
            } else {
                Rectangle().fill(NSColor(hex: record.colour).map { Color(nsColor: $0) } ?? Color.primary.opacity(0.08))
            }
        }
        .clipShape(Circle())
        .overlay(Circle().strokeBorder(.primary.opacity(0.25)))
        .accessibilityHidden(true)
    }
}

/// A finish's measured colour as a filled rounded square, or a hatch where the colour is still to
/// verify.
struct FinishSwatch: View {
    let colour: String
    var size: CGFloat = 24

    var body: some View {
        RoundedRectangle(cornerRadius: 5)
            .fill(NSColor(hex: colour).map { Color(nsColor: $0) } ?? Color.primary.opacity(0.08))
            .overlay {
                if NSColor(hex: colour) == nil {
                    // A colour still to be measured: a diagonal hatch, not a guess.
                    GeometryReader { geo in
                        Path { p in
                            for i in stride(from: -geo.size.width, through: geo.size.width, by: 6) {
                                p.move(to: CGPoint(x: i, y: 0))
                                p.addLine(to: CGPoint(x: i + geo.size.height, y: geo.size.height))
                            }
                        }
                        .stroke(.primary.opacity(0.25), lineWidth: 1.5)
                        .clipShape(RoundedRectangle(cornerRadius: 5))
                    }
                }
            }
            .overlay(RoundedRectangle(cornerRadius: 5).strokeBorder(.primary.opacity(0.2)))
            .frame(width: size, height: size)
            .accessibilityHidden(true)
    }
}

/// A handle model drawn face-on from its STL: the ABI import puts the screw holes on X and the
/// handle towards −Y, so this projects every triangle onto the X–Z plane, with a soft depth cue
/// from −Y (nearest the viewer).
struct HandleThumbnail: View {
    let article: String

    var body: some View {
        let mesh = Self.mesh(article)
        Canvas { ctx, size in
            Self.draw(mesh, in: &ctx, size)
        }
        .accessibilityHidden(true)
    }

    /// Loaded meshes by article; nil for one that will not load, so it is not retried each draw.
    @MainActor private static var cache: [String: MeshData?] = [:]
    @MainActor private static func mesh(_ article: String) -> MeshData? {
        guard !article.isEmpty, let dir = Preferences.vendorCADDir else { return nil }
        let path = dir.appending(path: "abi/\(article).stl").path
        if let m = cache[article] { return m }
        let m = try? loadMesh(path: path)
        cache[article] = .some(m)
        return m
    }

    /// Face-on: X horizontal, Z up, a soft depth cue from −Y (the viewer's side).
    private static func draw(_ mesh: MeshData?, in ctx: inout GraphicsContext, _ size: CGSize) {
        guard let mesh else { return }
        let p = mesh.positions
        // Nine floats a triangle (three vertices × xyz); a malformed mesh draws nothing.
        guard p.count >= 9, p.count % 9 == 0 else { return }
        var minX = Double.greatestFiniteMagnitude, maxX = -minX, minZ = minX, maxZ = -minX
        var depth = (Double.greatestFiniteMagnitude, -Double.greatestFiniteMagnitude)
        for i in stride(from: 0, to: p.count, by: 3) {
            let (x, y, z) = (Double(p[i]), Double(p[i + 1]), Double(p[i + 2]))
            minX = min(minX, x); maxX = max(maxX, x)
            minZ = min(minZ, z); maxZ = max(maxZ, z)
            depth.0 = min(depth.0, y); depth.1 = max(depth.1, y)
        }
        guard maxX > minX, maxZ > minZ, depth.1 > depth.0 else { return }
        let pad = 4.0
        let scale = min((size.width - pad * 2) / (maxX - minX), (size.height - pad * 2) / (maxZ - minZ))
        let cx = (minX + maxX) / 2, cz = (minZ + maxZ) / 2
        let to = { (x: Double, z: Double) in
            CGPoint(x: size.width / 2 + (x - cx) * scale, y: size.height / 2 - (z - cz) * scale)
        }
        var triangles: [(Path, Double)] = []
        for i in stride(from: 0, to: p.count, by: 9) {
            var path = Path()
            let first = to(Double(p[i]), Double(p[i + 2]))
            path.move(to: first)
            for a in 1...2 {
                path.addLine(to: to(Double(p[i + a * 3]), Double(p[i + a * 3 + 2])))
            }
            path.addLine(to: first)
            // Nearer the viewer (−Y) is lighter: the front of the pull reads against its legs.
            let nearness = ((Double(p[i + 1]) + Double(p[i + 4]) + Double(p[i + 7])) / 3 - depth.0) / (depth.1 - depth.0)
            triangles.append((path, 0.45 + 0.5 * (1 - nearness)))
        }
        for (path, shade) in triangles {
            ctx.fill(path, with: .color(.primary.opacity(shade)))
        }
        // The screw holes, marked where the model's X span says they sit.
        let cc = (maxX - minX) / 2
        if cc > 6 {
            for x in [minX + cc, maxX - cc] {
                let c = to(x, cz)
                ctx.fill(Path(ellipseIn: CGRect(x: c.x - 1.5, y: c.y - 1.5, width: 3, height: 3)),
                         with: .color(.accentColor))
            }
        }
    }
}
