import AppKit

/// Local full-slab photographs and physically scaled, smooth surface looks, for every benchtop
/// product: stone slabs and laminate tops. Each brand's `textures/<brand>.json` names its images in
/// the decors folder (`scripts/import-*.py`, `scripts/fetch_board_images.py`).
@MainActor enum SlabLibrary {
    struct Surface {
        let imageURL: URL
        /// The image's name in the decors folder, for `DecorThumbnails`.
        let imageName: String
        let roughness: Float
        let length: Float
        let width: Float
        /// A sheet photographed standing up: its length runs up the image.
        let portrait: Bool
    }
    /// By brand file, then decor: two brands may share a decor's name.
    private static var cached: [String: [String: Surface]] = [:]
    private static var loaded: String?

    /// The file a brand's images are listed in, as the Blender export finds it too.
    private static func file(_ brand: String) -> String {
        brand.lowercased().replacingOccurrences(of: " ", with: "-")
    }

    static func surface(_ board: BoardRecord) -> Surface? {
        guard ["sintered_stone", "laminate_top"].contains(board.kind), let catalog = Preferences.catalogDir else { return nil }
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
        // A board decor's photograph is of its largest cabinet sheet, as the Blender export takes it.
        var sheets: [String: [Double]] = [:]
        let files = (try? FileManager.default.contentsOfDirectory(at: catalog.appending(path: "boards"), includingPropertiesForKeys: nil)) ?? []
        for url in files where url.pathExtension == "json" {
            guard let data = try? Data(contentsOf: url),
                  let root = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any],
                  (root["brand"] as? String).map(file) == brand,
                  let boards = root["boards"] as? [[String: Any]] else { continue }
            for b in boards where b["kind"] as? String != "laminate_top" {
                guard let decor = b["decor"] as? String, let list = b["sheets"] as? [[String: Double]] else { continue }
                for s in list {
                    guard let l = s["length"], let w = s["width"] else { continue }
                    if l > sheets[decor]?[0] ?? 0 { sheets[decor] = [l, w] }
                }
            }
        }
        var out: [String: Surface] = [:]
        for (decor, image) in images {
            guard let name = image["name"] as? String,
                  let covers = image["covers"] as? [Double] ?? sheets[decor], covers.count == 2 else { continue }
            let look = image["look"] as? [String: Double] ?? [:]
            let pixels = image["pixels"] as? [Double] ?? []
            out[decor] = Surface(imageURL: decors.appending(path: "\(name).jpg"), imageName: name,
                roughness: Float(look["roughness"] ?? 0.32), length: Float(covers[0]/1000), width: Float(covers[1]/1000),
                portrait: pixels.count == 2 && pixels[1] > pixels[0])
        }
        return out
    }
}
