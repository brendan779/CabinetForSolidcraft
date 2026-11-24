import AppKit
import ImageIO

/// Small pictures of decors for the board tiles, made from the full images in the decors folder
/// (`scripts/fetch_board_images.py`). A full sheet is up to 6000 pixels: each is shrunk once, kept on
/// disk in Caches, and held in memory while the app runs.
enum DecorThumbnails {
    private static let size = 320
    @MainActor private static var memory: [String: NSImage] = [:]

    private static var decors: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appending(path: "CabinetStudio/decors")
    }

    private static var cache: URL {
        FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appending(path: "CabinetStudio/thumbnails")
    }

    /// The decor's thumbnail, or nil when it has no picture on this Mac.
    @MainActor static func image(_ name: String) async -> NSImage? {
        guard !name.isEmpty else { return nil }
        if let image = memory[name] { return image }
        let made = await Task.detached(priority: .utility) { () -> NSImage? in
            let small = cache.appending(path: "\(name).jpg")
            if let image = NSImage(contentsOf: small) { return image }
            let full = decors.appending(path: "\(name).jpg")
            guard let source = CGImageSourceCreateWithURL(full as CFURL, nil),
                  let cg = CGImageSourceCreateThumbnailAtIndex(source, 0, [
                      kCGImageSourceCreateThumbnailFromImageAlways: true,
                      kCGImageSourceThumbnailMaxPixelSize: size,
                      kCGImageSourceCreateThumbnailWithTransform: true,
                  ] as CFDictionary)
            else { return nil }
            try? FileManager.default.createDirectory(at: cache, withIntermediateDirectories: true)
            if let out = CGImageDestinationCreateWithURL(small as CFURL, "public.jpeg" as CFString, 1, nil) {
                CGImageDestinationAddImage(out, cg, [kCGImageDestinationLossyCompressionQuality: 0.85] as CFDictionary)
                CGImageDestinationFinalize(out)
            }
            return NSImage(cgImage: cg, size: NSSize(width: cg.width, height: cg.height))
        }.value
        if let made { memory[name] = made }
        return made
    }
}
