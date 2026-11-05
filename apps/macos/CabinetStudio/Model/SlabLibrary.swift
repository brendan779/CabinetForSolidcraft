import AppKit

/// Local full-slab photographs and physically scaled, smooth surface looks.
@MainActor enum SlabLibrary {
    struct Surface {
        let imageURL: URL
        let roughness: Float
        let length: Float
        let width: Float
    }
    private static var cached: [String: Surface] = [:]
    private static var loaded: String?
    static func surface(_ board: BoardRecord) -> Surface? {
        guard board.kind == "sintered_stone", let catalog = Preferences.catalogDir else { return nil }
        if loaded != catalog.path {
            loaded = catalog.path; cached = [:]
            let url = catalog.appending(path: "textures/smartstone.json")
            if let data = try? Data(contentsOf: url),
               let root = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any],
               let images = root["images"] as? [String: [String: Any]] {
                for (decor, image) in images {
                    guard let file = image["file"] as? String, let covers = image["covers"] as? [Double], covers.count == 2 else { continue }
                    let look = image["look"] as? [String: Double] ?? [:]
                    cached[decor] = Surface(imageURL: catalog.appending(path: "textures/\(file)"),
                        roughness: Float(look["roughness"] ?? 0.32), length: Float(covers[0]/1000), width: Float(covers[1]/1000))
                }
            }
        }
        return cached[board.decor]
    }
}
