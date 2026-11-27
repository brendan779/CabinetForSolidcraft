import SwiftUI

/// The room the Blender scene is made in: the walls' paint, and the floor. Kept between scenes.
struct SceneOptions: Codable, Equatable {
    enum Floor: String, Codable, CaseIterable { case boards, concrete, tiles, product }
    var wallColour = "#EBE8E1"
    var floor = Floor.boards
    /// The stone product the tiles are cut from (a board id).
    var tileStone = ""
    var tileLength = 600.0
    var tileWidth = 600.0
    var grout = "#8A8782"
    /// A tile product's floor (catalog/floors), by id.
    var floorProduct = ""

    init() {}

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let d = SceneOptions()
        wallColour = try c.decodeIfPresent(String.self, forKey: .wallColour) ?? d.wallColour
        floor = (try? c.decodeIfPresent(Floor.self, forKey: .floor)) ?? d.floor
        tileStone = try c.decodeIfPresent(String.self, forKey: .tileStone) ?? d.tileStone
        tileLength = try c.decodeIfPresent(Double.self, forKey: .tileLength) ?? d.tileLength
        tileWidth = try c.decodeIfPresent(Double.self, forKey: .tileWidth) ?? d.tileWidth
        grout = try c.decodeIfPresent(String.self, forKey: .grout) ?? d.grout
        floorProduct = try c.decodeIfPresent(String.self, forKey: .floorProduct) ?? d.floorProduct
    }

    static let key = "sceneOptions"
    static var saved: SceneOptions {
        get {
            UserDefaults.standard.data(forKey: key).flatMap { try? JSONDecoder().decode(SceneOptions.self, from: $0) }
                ?? SceneOptions()
        }
        set { UserDefaults.standard.set(try? JSONEncoder().encode(newValue), forKey: key) }
    }

    static let tileSizes: [TileSize] = [(300, 300), (600, 300), (600, 600), (1200, 600), (1200, 1200)].map(TileSize.init)
    struct TileSize {
        let length, width: Double
        init(_ s: (Double, Double)) { (length, width) = s }
        var label: String { "\(String(Int(length))) × \(String(Int(width)))" }
    }
    static let grouts: [(String, String)] = [("White", "#E8E6E0"), ("Grey", "#8A8782"), ("Charcoal", "#3A3936")]

    /// What build_scene.py reads (`--room`): a tiled floor names its photo, the slab size the
    /// photo covers, and the stone's sheen.
    @MainActor func roomJSON(_ boards: [String: BoardRecord]) -> Data? {
        var floor: [String: Any] = ["kind": floor.rawValue]
        if self.floor == .tiles, let board = boards[tileStone], let surface = SlabLibrary.surface(board) {
            floor["image"] = surface.imageURL.path
            floor["covers"] = [Double(surface.length) * 1000, Double(surface.width) * 1000]
            floor["roughness"] = Double(surface.roughness)
            floor["tile"] = [tileLength, tileWidth]
            floor["grout"] = grout
            floor["grout_mm"] = 3
        }
        if self.floor == .product, let product = FloorProduct.all.first(where: { $0.id == floorProduct }) {
            floor["colour"] = product.url(product.colour, "jpg").path
            floor["height"] = product.url(product.height, "png").path
            floor["grout"] = product.url(product.grout, "png").path
            floor["covers"] = product.covers
            floor["roughness"] = product.roughness
            floor["bump"] = product.bump
            floor["bump_mm"] = product.bumpMM
            floor["grout_colour"] = grout
        }
        return try? JSONSerialization.data(withJSONObject: ["wall_colour": wallColour, "floor": floor], options: [.sortedKeys])
    }
}

/// Asked before each Blender scene: the walls' colour and the floor, as pictures.
struct SceneOptionsCard: View {
    @Environment(ProjectModel.self) private var project
    @State private var options = SceneOptions.saved
    @State private var search = ""

    private var stones: [BoardRecord] {
        // One tile per decor: a slab in two thicknesses looks the same on the floor.
        var seen = Set<String>()
        return project.boards.filter { b in
            b.kind == "sintered_stone" && seen.insert("\(b.brand)|\(b.decor)").inserted
                && (search.isEmpty || "\(b.brand) \(b.decor) \(b.finish)".localizedCaseInsensitiveContains(search))
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("Blender scene").font(.title2.weight(.semibold))
            section("Walls") {
                FlowLayout(spacing: 8) {
                    ForEach(project.paintRange.paints, id: \.name) { paint in
                        swatch(paint.colour, paint.name, selected: options.wallColour.caseInsensitiveCompare(paint.colour) == .orderedSame) {
                            options.wallColour = paint.colour
                        }
                    }
                    ColorPicker("Other", selection: Binding(
                        get: { Color(hex: options.wallColour) ?? .white },
                        set: { options.wallColour = NSColor($0).hexString }
                    ), supportsOpacity: false)
                    .labelsHidden()
                    .help("Any other wall colour")
                }
            }
            section("Floor") {
                HStack(spacing: 8) {
                    floorTile(.boards, "Oak boards") { FloorPreview.boards }
                    floorTile(.concrete, "Polished concrete") { FloorPreview.concrete }
                    floorTile(.product, "Floor tiles") { ProductThumb(product: products.first { $0.id == options.floorProduct } ?? products.first) }
                    floorTile(.tiles, "Cut from a slab") { StoneThumb(board: project.boardsById[options.tileStone]) }
                }
            }
            if options.floor == .product {
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 4), spacing: 8) {
                    ForEach(products) { product in
                        Button {
                            options.floorProduct = product.id
                        } label: {
                            VStack(alignment: .leading, spacing: 3) {
                                ProductThumb(product: product)
                                    .frame(maxWidth: .infinity)
                                    .frame(height: 54)
                                    .clipShape(RoundedRectangle(cornerRadius: 4))
                                Text(product.name).font(.caption).lineLimit(1)
                                Text("\(product.brand) · \(product.sizeLabel)").font(.caption2).foregroundStyle(.secondary).lineLimit(1)
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(4)
                        }
                        .buttonStyle(TileStyle(selected: options.floorProduct == product.id))
                        .disabled(!product.installed)
                        .help(product.installed ? "\(product.brand) \(product.name), \(product.finish)" : "Its pictures aren't on this Mac")
                    }
                }
                groutPicker
            }
            if options.floor == .tiles {
                TextField("Search stone", text: $search).textFieldStyle(.roundedBorder)
                ScrollView {
                    LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 8), count: 4), spacing: 8) {
                        ForEach(stones, id: \.id) { board in
                            Button {
                                options.tileStone = board.id
                            } label: {
                                VStack(alignment: .leading, spacing: 3) {
                                    StoneThumb(board: board)
                                        .frame(maxWidth: .infinity)
                                        .frame(height: 54)
                                        .clipShape(RoundedRectangle(cornerRadius: 4))
                                    Text(board.decor).font(.caption).lineLimit(1)
                                    Text(board.brand).font(.caption2).foregroundStyle(.secondary).lineLimit(1)
                                }
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(4)
                            }
                            .buttonStyle(TileStyle(selected: project.boardsById[options.tileStone].map { $0.brand == board.brand && $0.decor == board.decor } ?? false))
                        }
                    }
                }
                .frame(height: 200)
                section("Tile size") {
                    FlowLayout(spacing: 6) {
                        ForEach(SceneOptions.tileSizes, id: \.label) { size in
                            let on = options.tileLength == size.length && options.tileWidth == size.width
                            Button(size.label) {
                                options.tileLength = size.length
                                options.tileWidth = size.width
                            }
                            .buttonStyle(ChipStyle(selected: on))
                        }
                    }
                }
                groutPicker
            }
            HStack {
                Spacer()
                Button("Cancel") { project.showSceneOptions = false }
                    .keyboardShortcut(.cancelAction)
                Button("Make Scene") {
                    SceneOptions.saved = options
                    project.showSceneOptions = false
                    Task { await project.makeBlenderScene(options) }
                }
                .keyboardShortcut(.defaultAction)
                .disabled(options.floor == .tiles && project.boardsById[options.tileStone] == nil
                          || options.floor == .product && !products.contains { $0.id == options.floorProduct && $0.installed })
            }
        }
        .padding(24)
        .frame(width: 560)
        .animation(.snappy, value: options)
        .onAppear {
            if options.floorProduct.isEmpty, let first = products.first(where: \.installed) { options.floorProduct = first.id }
        }
    }

    private var products: [FloorProduct] { FloorProduct.all }

    private var groutPicker: some View {
        section("Grout") {
            HStack(spacing: 8) {
                ForEach(SceneOptions.grouts, id: \.0) { grout in
                    swatch(grout.1, grout.0, selected: options.grout == grout.1) { options.grout = grout.1 }
                }
            }
        }
    }

    private func section(_ title: String, @ViewBuilder _ content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title).font(.headline)
            content()
        }
    }

    private func swatch(_ hex: String, _ name: String, selected: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            RoundedRectangle(cornerRadius: 6)
                .fill(Color(hex: hex) ?? .gray)
                .frame(width: 34, height: 34)
                .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(selected ? Theme.accent : Theme.border, lineWidth: selected ? 2.5 : 1))
        }
        .buttonStyle(.plain)
        .help(name)
    }

    private func floorTile(_ kind: SceneOptions.Floor, _ title: String, @ViewBuilder picture: () -> some View) -> some View {
        Button {
            options.floor = kind
        } label: {
            VStack(spacing: 5) {
                // The picture fills the space given; a photo's own width mustn't size the tile.
                Color.clear.frame(maxWidth: .infinity).frame(height: 70)
                    .overlay { picture() }
                    .clipShape(RoundedRectangle(cornerRadius: 6))
                Text(title).font(.caption).lineLimit(1)
            }
            .frame(maxWidth: .infinity)
            .padding(6)
        }
        .buttonStyle(TileStyle(selected: options.floor == kind))
    }
}

/// A stone's photograph, small; its colour until the picture is loaded.
private struct StoneThumb: View {
    let board: BoardRecord?
    @State private var image: NSImage?
    var body: some View {
        Group {
            if let image {
                Image(nsImage: image).resizable().scaledToFill()
            } else {
                Color(nsColor: board.flatMap { NSColor(hex: $0.previewColour) } ?? .darkGray)
            }
        }
        .clipped()
        .task(id: board?.id) {
            guard let board, let surface = SlabLibrary.surface(board) else { image = nil; return }
            image = await DecorThumbnails.image(surface.imageName)
        }
    }
}

/// A tile product's floor picture, small; its colour until the picture is loaded.
private struct ProductThumb: View {
    let product: FloorProduct?
    @State private var image: NSImage?
    var body: some View {
        Group {
            if let image {
                Image(nsImage: image).resizable().scaledToFill()
            } else {
                Color(nsColor: product.flatMap { NSColor(hex: $0.previewColour) } ?? .darkGray)
            }
        }
        .clipped()
        .task(id: product?.id) {
            image = if let product { await DecorThumbnails.image(product.colour) } else { nil }
        }
    }
}

/// A tile product's floor for the Blender scene (catalog/floors/floors.json): a seamless picture of a
/// grid of its tiles with grout, made by scripts/make_floor_tiles.py, kept with the decors.
struct FloorProduct: Identifiable, Equatable {
    let id, brand, name, finish: String
    let tile: [Double]
    let covers: [Double]
    let colour, height, grout: String
    let roughness, bump, bumpMM: Double
    let previewColour: String

    var sizeLabel: String { tile.map { String(Int($0)) }.joined(separator: " × ") }

    func url(_ name: String, _ ext: String) -> URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appending(path: "CabinetStudio/decors/\(name).\(ext)")
    }

    var installed: Bool { FileManager.default.fileExists(atPath: url(colour, "jpg").path) }

    @MainActor static var all: [FloorProduct] {
        guard let catalog = Preferences.catalogDir else { return [] }
        if loaded != catalog.path { loaded = catalog.path; cached = load(catalog.appending(path: "floors/floors.json")) }
        return cached
    }
    @MainActor private static var loaded = ""
    @MainActor private static var cached: [FloorProduct] = []

    static func load(_ file: URL) -> [FloorProduct] {
        guard let data = try? Data(contentsOf: file),
              let root = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any],
              let floors = root["floors"] as? [[String: Any]] else { return [] }
        return floors.compactMap { f in
            guard let id = f["id"] as? String, let name = f["name"] as? String,
                  let colour = f["colour"] as? String, let height = f["height"] as? String, let grout = f["grout"] as? String,
                  let tile = f["tile"] as? [Double], let covers = f["covers"] as? [Double], covers.count == 2 else { return nil }
            let look = f["look"] as? [String: Double] ?? [:]
            return FloorProduct(id: id, brand: f["brand"] as? String ?? "", name: name, finish: f["finish"] as? String ?? "",
                                tile: tile, covers: covers, colour: colour, height: height, grout: grout,
                                roughness: look["roughness"] ?? 0.6, bump: look["bump"] ?? 1, bumpMM: look["bump_mm"] ?? 2.5,
                                previewColour: f["preview_colour"] as? String ?? "#c8c4bc")
        }
    }
}

/// Small drawings of the procedural floors, as the scene makes them.
private enum FloorPreview {
    static var boards: some View {
        Canvas { ctx, size in
            let rows = 5
            let h = size.height / CGFloat(rows)
            for r in 0..<rows {
                let shift = r % 2 == 0 ? 0 : size.width * 0.35
                for i in -1..<3 {
                    let x = shift + CGFloat(i) * size.width * 0.7
                    let plank = CGRect(x: x, y: CGFloat(r) * h, width: size.width * 0.7 - 1, height: h - 1)
                    let tone = 0.34 + 0.05 * Double((r * 3 + i + 7) % 4)
                    ctx.fill(Path(plank), with: .color(Color(red: tone, green: tone * 0.72, blue: tone * 0.48)))
                }
            }
        }
    }

    static var concrete: some View {
        LinearGradient(colors: [Color(white: 0.52), Color(white: 0.6), Color(white: 0.5)], startPoint: .topLeading, endPoint: .bottomTrailing)
    }
}
