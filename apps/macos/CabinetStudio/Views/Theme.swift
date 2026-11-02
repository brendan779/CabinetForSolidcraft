import AppKit
import SwiftUI

/// SolidCraft's Clay Dark colours (assets/styles/ui/themes/clay-dark.json, Apache-2.0, Tangency
/// Labs), named for what they do here, with colours of our own for cabinet parts.
enum Theme {
    // Surfaces, darkest first.
    static let appBar = Color(hex: 0x1B1814)
    static let panel = Color(hex: 0x221E1A)
    static let strip = Color(hex: 0x29241F)
    static let viewportTop = Color(hex: 0x312B25)
    static let viewportBottom = Color(hex: 0x29241F)
    static let control = Color(hex: 0x3C352D)
    static let hover = Color(hex: 0x4B4238)
    static let overlay = Color(hex: 0x221E1A).opacity(0.94)
    static let selected = Color(hex: 0x4D3329)

    // Lines.
    static let border = Color(hex: 0x4A433B)
    static let hairline = Color(hex: 0x332F2A)

    // Ink.
    static let text = Color(hex: 0xECE6D9)
    static let textDim = Color(hex: 0x9D9484)
    static let accent = Color(hex: 0xE8845E)
    static let onAccent = Color(hex: 0x1D1A17)

    // Signals.
    static let ok = Color(hex: 0x89CF8E)
    static let warning = Color(hex: 0xE6AF2B)
    static let error = Color(hex: 0xF85052)

    // The viewport's parts: ours, from the same warm hues.
    static let carcass = NSColor(hex: 0xEFE9DC)
    static let carcassBack = NSColor(hex: 0xB9B1A2)
    static let front = NSColor(hex: 0xD9C7A4)
    static let floor = NSColor(hex: 0x3C352D)
    static let wall = NSColor(hex: 0xE4E1DD, alpha: 0.3)
}

extension Color {
    init(hex: UInt32) {
        self.init(
            .sRGB,
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255
        )
    }
}

extension NSColor {
    /// `#RRGGBB`, for SolidCraft.
    var hexString: String {
        let c = usingColorSpace(.sRGB) ?? self
        return String(format: "#%02X%02X%02X", Int(c.redComponent * 255), Int(c.greenComponent * 255), Int(c.blueComponent * 255))
    }

    convenience init(hex: UInt32, alpha: CGFloat = 1) {
        self.init(
            srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
            green: CGFloat((hex >> 8) & 0xFF) / 255,
            blue: CGFloat(hex & 0xFF) / 255,
            alpha: alpha
        )
    }
}
