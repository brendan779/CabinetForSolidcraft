import AppKit
import SwiftUI

extension BoardRecord {
    /// The board's colour on screen: its catalog `preview_colour`, or when the catalog has none, a
    /// rough guess from the decor's name (Oak, Walnut, Grey...). Either way an approximation: the
    /// brands' swatches are not ours to copy.
    var displayColour: NSColor {
        NSColor(hex: previewColour) ?? Self.guess(decor: decor, grained: grained)
    }

    /// Whether `displayColour` is guessed from the name rather than given in the catalog.
    var colourIsGuessed: Bool { NSColor(hex: previewColour) == nil }

    /// Word in the decor name → sRGB. The first word found wins; species before colours, so
    /// "White Oak" is an oak, lightened below.
    private static let named: [(String, UInt32)] = [
        ("walnut", 0x6b4a33), ("wenge", 0x3e2c22), ("teak", 0x9a6a3c), ("cherry", 0x9c5a3c),
        ("oak", 0xc4a07a), ("ash", 0xcdb89a), ("beech", 0xd9b48a), ("maple", 0xe2c9a4),
        ("elm", 0xb08a64), ("birch", 0xe3cfab), ("pine", 0xddbb87), ("ply", 0xdcc29a),
        ("rattan", 0xc9a46e), ("black", 0x1f1f1f), ("onyx", 0x26262a), ("charcoal", 0x3a3b3d),
        ("char", 0x3a3734), ("graphite", 0x47484b), ("grey", 0x8e8f8c), ("gray", 0x8e8f8c),
        ("cinder", 0x6f6e6a), ("cement", 0xa9a6a0), ("concrete", 0xa3a19b), ("stone", 0xb5aea3),
        ("greige", 0xb7afa3), ("taupe", 0x9b8f84), ("navy", 0x2c3a4f), ("blue", 0x5d7590),
        ("green", 0x6f7f63), ("sage", 0x9aa48c), ("olive", 0x75744f), ("pink", 0xd8b8b0),
        ("terracotta", 0xb4674a), ("ochre", 0xc39a4e), ("gold", 0xb99a5b), ("bronze", 0x8a6a45),
        ("copper", 0xa8673f), ("cream", 0xeee6d3), ("ecru", 0xe8e0cc), ("ivory", 0xefe9da),
        ("beige", 0xd9ccb5), ("linen", 0xe6dfd0), ("cotton", 0xefece4), ("white", 0xf1f0eb),
    ]

    static func guess(decor: String, grained: Bool) -> NSColor {
        let words = decor.lowercased().split { !$0.isLetter }.map(String.init)
        let rgb = named.first { entry in words.contains(entry.0) }?.1 ?? (grained ? 0xc4a07a : 0xd8d6d0)
        var colour = NSColor(srgbRed: CGFloat((rgb >> 16) & 0xff) / 255, green: CGFloat((rgb >> 8) & 0xff) / 255,
                             blue: CGFloat(rgb & 0xff) / 255, alpha: 1)
        let lighter = ["bleached", "blonde", "white", "natural", "nordic", "coastal", "drifted", "polar"]
        let darker = ["smoked", "blackened", "dark", "char", "burnt", "espresso", "havana"]
        let isWood = grained && !["white", "black", "grey", "gray"].contains { words.first == $0 }
        if isWood, words.contains(where: lighter.contains) {
            colour = colour.blended(withFraction: 0.35, of: .white) ?? colour
        }
        if words.contains(where: darker.contains) {
            colour = colour.blended(withFraction: 0.35, of: .black) ?? colour
        }
        return colour
    }
}
