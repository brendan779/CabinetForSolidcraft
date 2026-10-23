// Draws Cabinet Studio's app icon into the asset catalog: a cabinet seen from the front, a pair of
// oak doors on a pale carcass, on a deep blue tile.
//
//   swift tools/make_app_icon.swift apps/macos/CabinetStudio/Assets.xcassets/AppIcon.appiconset
import AppKit

let out = URL(fileURLWithPath: CommandLine.arguments[1])

func rgb(_ hex: UInt32, _ a: CGFloat = 1) -> CGColor {
    CGColor(srgbRed: CGFloat((hex >> 16) & 0xff) / 255, green: CGFloat((hex >> 8) & 0xff) / 255,
            blue: CGFloat(hex & 0xff) / 255, alpha: a)
}

func draw(_ px: Int) -> Data {
    let s = CGFloat(px)
    let ctx = CGContext(data: nil, width: px, height: px, bitsPerComponent: 8, bytesPerRow: 0,
                        space: CGColorSpace(name: CGColorSpace.sRGB)!,
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.scaleBy(x: s / 1024, y: s / 1024)
    // The macOS icon grid: an 824 pt rounded tile centred on the 1024 canvas, with a soft shadow.
    let tile = CGRect(x: 100, y: 100, width: 824, height: 824)
    let tilePath = CGPath(roundedRect: tile, cornerWidth: 185, cornerHeight: 185, transform: nil)
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 24, color: rgb(0x000000, 0.35))
    ctx.addPath(tilePath)
    ctx.setFillColor(rgb(0x1f3a5c))
    ctx.fillPath()
    ctx.restoreGState()
    ctx.saveGState()
    ctx.addPath(tilePath)
    ctx.clip()
    let sky = CGGradient(colorsSpace: nil, colors: [rgb(0x2f5d8f), rgb(0x16304f)] as CFArray, locations: [0, 1])!
    ctx.drawLinearGradient(sky, start: CGPoint(x: 512, y: 924), end: CGPoint(x: 512, y: 100), options: [])
    // Floor line.
    ctx.setFillColor(rgb(0x0f2238, 0.6))
    ctx.fill(CGRect(x: 100, y: 100, width: 824, height: 170))
    ctx.restoreGState()

    // The carcass, with its toe kick.
    let carcass = CGRect(x: 250, y: 250, width: 524, height: 520)
    ctx.setFillColor(rgb(0xece9e2))
    ctx.fill(carcass)
    ctx.setFillColor(rgb(0x2a2a2a, 0.55))
    ctx.fill(CGRect(x: 280, y: 250, width: 464, height: 60))
    // Two doors with a gap between, in oak.
    let doorY: CGFloat = 322, doorH: CGFloat = 438
    let left = CGRect(x: 258, y: doorY, width: 250, height: doorH)
    let right = CGRect(x: 516, y: doorY, width: 250, height: doorH)
    let oak = CGGradient(colorsSpace: nil, colors: [rgb(0xd7b48a), rgb(0xb68d60)] as CFArray, locations: [0, 1])!
    for door in [left, right] {
        ctx.saveGState()
        ctx.addPath(CGPath(roundedRect: door, cornerWidth: 10, cornerHeight: 10, transform: nil))
        ctx.clip()
        ctx.drawLinearGradient(oak, start: CGPoint(x: door.minX, y: door.maxY), end: CGPoint(x: door.maxX, y: door.minY), options: [])
        // A few grain streaks, upright like a door's grain.
        ctx.setStrokeColor(rgb(0x6b4a2a, 0.18))
        for i in 0..<7 {
            let x = door.minX + 22 + CGFloat(i) * 32
            ctx.setLineWidth(i % 2 == 0 ? 3 : 1.5)
            ctx.move(to: CGPoint(x: x, y: door.minY))
            ctx.addCurve(to: CGPoint(x: x + 6, y: door.maxY), control1: CGPoint(x: x + 14, y: door.minY + doorH * 0.35),
                         control2: CGPoint(x: x - 10, y: door.minY + doorH * 0.7))
            ctx.strokePath()
        }
        ctx.restoreGState()
    }
    // Handles beside the gap.
    ctx.setFillColor(rgb(0x2b2b2b))
    for x in [478.0, 538.0] {
        ctx.addPath(CGPath(roundedRect: CGRect(x: x, y: 560, width: 12, height: 120), cornerWidth: 6, cornerHeight: 6, transform: nil))
        ctx.fillPath()
    }
    // A benchtop.
    ctx.setFillColor(rgb(0xf7f5f0))
    ctx.addPath(CGPath(roundedRect: CGRect(x: 226, y: 770, width: 572, height: 40), cornerWidth: 8, cornerHeight: 8, transform: nil))
    ctx.fillPath()
    let rep = NSBitmapImageRep(cgImage: ctx.makeImage()!)
    return rep.representation(using: .png, properties: [:])!
}

var images: [[String: String]] = []
for points in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let name = "icon_\(points)x\(points)\(scale == 2 ? "@2x" : "").png"
        try! draw(points * scale).write(to: out.appendingPathComponent(name))
        images.append(["idiom": "mac", "size": "\(points)x\(points)", "scale": "\(scale)x", "filename": name])
    }
}
let contents: [String: Any] = ["images": images, "info": ["version": 1, "author": "xcode"]]
try! JSONSerialization.data(withJSONObject: contents, options: [.prettyPrinted, .sortedKeys])
    .write(to: out.appendingPathComponent("Contents.json"))
