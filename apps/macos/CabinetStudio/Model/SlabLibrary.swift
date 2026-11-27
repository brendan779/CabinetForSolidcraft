import AppKit

/// Local full-slab photographs and physically scaled, smooth surface looks, for every stone brand:
/// each brand's `textures/<brand>.json` names its images in the decors folder
/// (`scripts/import-smartstone.py`, `scripts/import-zenith.py`).
@MainActor enum SlabLibrary {
    struct Surface {
        let imageURL: URL
        /// The image's name in the decors folder, for `DecorThumbnails`.
        let imageName: String
        let roughness: Float
        let length: Float
        let width: Float
    }
    /// By brand file, then decor: two brands may share a decor's name.
    private static var cached: [String: [String: Surface]] = [:]
    private static var loaded: String?

    /// The file a brand's images are listed in, as the Blender export finds it too.
    private static func file(_ brand: String) -> String {
        brand.lowercased().replacingOccurrences(of: " ", with: "-")
    }

    static func surface(_ board: BoardRecord) -> Surface? {
        guard board.kind == "sintered_stone", let catalog = Preferences.catalogDir else { return nil }
        if loaded != catalog.path { loaded = catalog.path; cached = [:] }
        let brand = file(board.brand)
        if cached[brand] == nil { cached[brand] = load(brand, catalog: catalog) }
        return cached[brand]?[board.decor]
    }

    private static func load(_ brand: String, catalog: URL) -> [String: Surface] {
        // The brand's photographs stay on this Mac, with the other decors.
        let decors = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appending(path: "CabinetStudio/decors")
        guard let data = try? Data(contentsOf: catalog.appending(path: "textures/\(brand).json")),
              let root = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any],
              let images = root["images"] as? [String: [String: Any]] else { return [:] }
        var out: [String: Surface] = [:]
        for (decor, image) in images {
            guard let name = image["name"] as? String, let covers = image["covers"] as? [Double], covers.count == 2 else { continue }
            let look = image["look"] as? [String: Double] ?? [:]
            out[decor] = Surface(imageURL: decors.appending(path: "\(name).jpg"), imageName: name,
                roughness: Float(look["roughness"] ?? 0.32), length: Float(covers[0]/1000), width: Float(covers[1]/1000))
        }
        return out
    }
}
