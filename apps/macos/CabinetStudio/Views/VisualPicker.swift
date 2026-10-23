import SwiftUI

/// A construction option as a row of drawn tiles. The drawing is the label: you pick how the
/// panels come together by looking, not by reading.
struct VisualPicker: View {
    let group: OptionGroupRecord
    let selected: String
    /// The cabinet's choice differs from the job's.
    var overridden = false
    var onReset: (() -> Void)?
    let onPick: (String) -> Void

    @State private var hovered: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                SectionTitle(group.title)
                if overridden {
                    Button {
                        withAnimation(.snappy) { onReset?() }
                    } label: {
                        Label("Own choice", systemImage: "arrow.uturn.backward.circle.fill")
                            .font(.caption2.weight(.semibold))
                    }
                    .buttonStyle(.borderless)
                    .foregroundStyle(.orange)
                    .help("This cabinet has its own choice here. Click to follow the job's standard.")
                }
                Spacer()
                Text(group.choices.first { $0.id == (hovered ?? selected) }?.description ?? "")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .truncationMode(.tail)
                    .animation(.none, value: hovered)
            }
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 72, maximum: 110), spacing: 8)], spacing: 8) {
                ForEach(group.choices, id: \.id) { choice in
                    Button {
                        onPick(choice.id)
                    } label: {
                        VStack(spacing: 6) {
                            ConstructionThumbnail(field: group.field, choice: choice.id, highlighted: choice.id == selected)
                                .frame(height: 54)
                            Text(choice.label)
                                .font(.caption)
                                .lineLimit(1)
                                .minimumScaleFactor(0.8)
                        }
                        .padding(.vertical, 8)
                        .padding(.horizontal, 4)
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(TileStyle(selected: choice.id == selected))
                    .onHover { hovered = $0 ? choice.id : nil }
                    .help(choice.description)
                }
            }
        }
    }
}

/// Drawings of each construction choice: sections through a carcass, joint close-ups and fixings.
struct ConstructionThumbnail: View {
    let field: String
    let choice: String
    let highlighted: Bool

    var body: some View {
        Canvas { ctx, size in
            let pen = Pen(ctx: ctx, size: size, accent: highlighted ? .accentColor : .primary.opacity(0.85))
            switch field {
            case "top_joint", "bottom_joint": pen.carcassFront(field: field, choice: choice)
            case "back": pen.backSection(choice)
            case "toe_kick": pen.toeKick(choice)
            case "wall_hanging": pen.wallHanging(choice)
            case "opening": pen.opening(choice)
            case "overlay": pen.overlayPlan(choice)
            case "fixings": pen.fixing(choice)
            case "biscuit": pen.biscuit(choice)
            case "fixing_density": pen.density(choice)
            default: pen.unknown()
            }
        }
    }
}

/// Drawing helpers in a 100 × 80 design space scaled into the tile.
private struct Pen {
    let ctx: GraphicsContext
    let size: CGSize
    let accent: Color

    private var s: CGFloat { min(size.width / 100, size.height / 80) }
    private var ox: CGFloat { (size.width - 100 * s) / 2 }
    private var oy: CGFloat { (size.height - 80 * s) / 2 }

    private func r(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat) -> CGRect {
        CGRect(x: ox + x * s, y: oy + y * s, width: w * s, height: h * s)
    }

    private func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: ox + x * s, y: oy + y * s) }

    /// A board in section: filled, outlined.
    func board(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat, hot: Bool = false) {
        let rect = r(x, y, w, h)
        ctx.fill(Path(rect), with: .color(hot ? accent.opacity(0.35) : Color.primary.opacity(0.10)))
        ctx.stroke(Path(rect), with: .color(hot ? accent : Color.primary.opacity(0.7)), lineWidth: hot ? 1.4 : 1)
    }

    func line(_ a: CGPoint, _ b: CGPoint, colour: Color? = nil, width: CGFloat = 1, dash: [CGFloat] = []) {
        var path = Path()
        path.move(to: a)
        path.addLine(to: b)
        ctx.stroke(path, with: .color(colour ?? .primary.opacity(0.6)), style: StrokeStyle(lineWidth: width, dash: dash))
    }

    // MARK: Carcass front section (top and bottom joints)

    func carcassFront(field: String, choice: String) {
        let t: CGFloat = 7
        let (x0, x1, y0, y1): (CGFloat, CGFloat, CGFloat, CGFloat) = (22, 78, 6, 74)
        let top = field == "top_joint" ? choice : "between_sides"
        let bottom = field == "bottom_joint" ? choice : "between_sides"
        let hotTop = field == "top_joint", hotBottom = field == "bottom_joint"
        let sideTop = top == "over_sides" ? y0 + t : y0
        let sideBottom = bottom == "under_sides" ? y1 - t : y1
        board(x0, sideTop, t, sideBottom - sideTop)
        board(x1 - t, sideTop, t, sideBottom - sideTop)
        switch top {
        case "over_sides": board(x0, y0, x1 - x0, t, hot: hotTop)
        case "rails":
            // Rails seen end-on: a front and a back rail, drawn as two short boards.
            board(x0 + t, y0, 12, t, hot: hotTop)
            board(x1 - t - 12, y0, 12, t, hot: hotTop)
            line(p(x0 + t + 12, y0 + t / 2), p(x1 - t - 12, y0 + t / 2), dash: [2, 2])
        default: board(x0 + t, y0, x1 - x0 - 2 * t, t, hot: hotTop)
        }
        if bottom == "under_sides" {
            board(x0, y1 - t, x1 - x0, t, hot: hotBottom)
        } else {
            board(x0 + t, y1 - t, x1 - x0 - 2 * t, t, hot: hotBottom)
        }
    }

    // MARK: Side section (back styles)

    func backSection(_ choice: String) {
        // Looking at the left side from inside: front on the left, back on the right.
        let (x0, x1, y0, y1): (CGFloat, CGFloat, CGFloat, CGFloat) = (12, 88, 8, 72)
        let t: CGFloat = 6
        ctx.stroke(Path(r(x0, y0, x1 - x0, y1 - y0)), with: .color(.primary.opacity(0.35)), lineWidth: 1)
        board(x0, y0, x1 - x0, t)          // top
        board(x0, y1 - t, x1 - x0, t)      // bottom
        switch choice {
        case "inset_groove":
            board(x1 - 12, y0 + t - 2, 3, y1 - y0 - 2 * t + 4, hot: true)
        case "applied":
            board(x1, y0, 3, y1 - y0, hot: true)
        case "structural":
            board(x1 - t, y0 + t, t, y1 - y0 - 2 * t, hot: true)
        default:
            line(p(x1, y0 + t), p(x1, y1 - t), colour: accent, width: 1.2, dash: [3, 3])
        }
        arrowText("front", at: p(x0 + 2, y1 + 6))
    }

    // MARK: Opening, a door and a drawer from the front

    func opening(_ choice: String) {
        board(14, 8, 34, 64)        // door
        board(54, 8, 34, 28)        // drawer fronts
        board(54, 40, 34, 32)
        switch choice {
        case "handles":
            board(42, 34, 2.5, 14, hot: true)
            board(63, 20, 16, 2.5, hot: true)
            board(63, 54, 16, 2.5, hot: true)
        case "tip_on":
            // Press here: an arrow into the door and a drawer.
            for (x, y) in [(31.0, 40.0), (71.0, 22.0), (71.0, 56.0)] as [(CGFloat, CGFloat)] {
                ctx.fill(Path(ellipseIn: r(x - 4, y - 4, 8, 8)), with: .color(accent.opacity(0.35)))
                ctx.stroke(Path(ellipseIn: r(x - 4, y - 4, 8, 8)), with: .color(accent), lineWidth: 1.2)
                ctx.fill(Path(ellipseIn: r(x - 1.2, y - 1.2, 2.4, 2.4)), with: .color(accent))
            }
        default:
            // A press point on the door; a lightning bolt on each drawer.
            ctx.stroke(Path(ellipseIn: r(27, 36, 8, 8)), with: .color(accent), lineWidth: 1.2)
            for y in [22.0, 56.0] as [CGFloat] {
                var bolt = Path()
                bolt.move(to: p(73, y - 7))
                bolt.addLine(to: p(68, y + 1))
                bolt.addLine(to: p(72, y + 1))
                bolt.addLine(to: p(69, y + 8))
                bolt.addLine(to: p(76, y - 1))
                bolt.addLine(to: p(72, y - 1))
                bolt.closeSubpath()
                ctx.fill(bolt, with: .color(accent))
            }
        }
    }

    // MARK: Wall hanging, side section against the wall

    func wallHanging(_ choice: String) {
        // The wall on the right; the cabinet's side, top and inset back, with the void behind it.
        let wall: CGFloat = 86
        line(p(wall, 4), p(wall, 76), colour: .primary.opacity(0.5), width: 1.5)
        let (x0, y0, y1): (CGFloat, CGFloat, CGFloat) = (20, 16, 66)
        ctx.stroke(Path(r(x0, y0, wall - x0, y1 - y0)), with: .color(.primary.opacity(0.35)), lineWidth: 1)
        board(x0, y0, wall - x0, 6)                  // top
        board(x0, y1 - 6, wall - x0, 6)              // bottom
        board(wall - 12, y0 + 4, 3, y1 - y0 - 8)     // inset back
        guard choice == "hangers" else {
            line(p(wall - 8, y0 + 10), p(wall - 2, y0 + 10), colour: accent, width: 1.2, dash: [2, 2])
            return
        }
        board(wall - 4, y0 - 2, 4, 6, hot: true)     // wall rail
        // The hanger in the void, its hook over the rail.
        board(wall - 9, y0 + 7, 4, 12, hot: true)
        var hook = Path()
        hook.move(to: p(wall - 7, y0 + 7))
        hook.addQuadCurve(to: p(wall - 3, y0 + 4), control: p(wall - 7, y0 + 3))
        ctx.stroke(hook, with: .color(accent), lineWidth: 1.6)
    }

    // MARK: Toe kick, side elevation

    func toeKick(_ choice: String) {
        let (x0, x1): (CGFloat, CGFloat) = (14, 86)
        let floor: CGFloat = 74, kick: CGFloat = 16, set: CGFloat = 10
        line(p(4, floor), p(96, floor), colour: .primary.opacity(0.5))
        let carcassBottom = choice == "none" || choice == "integrated_notch" ? floor : floor - kick
        board(x0, 8, x1 - x0, carcassBottom - 8)
        switch choice {
        case "legs_and_plinth":
            for x in [x0 + 14, x1 - 10] { board(x, floor - kick, 4, kick, hot: true) }
            board(x0 + set, floor - kick + 1, 3, kick - 1, hot: true)
        case "plinth_box":
            board(x0 + set, floor - kick, x1 - x0 - set, kick, hot: true)
        case "integrated_notch":
            let notch = r(x0, floor - kick, set, kick)
            ctx.fill(Path(notch), with: .color(Color(nsColor: .windowBackgroundColor)))
            ctx.stroke(Path(notch), with: .color(accent), lineWidth: 1.4)
        default:
            break
        }
    }

    // MARK: Plan view of a door against a side

    func overlayPlan(_ choice: String) {
        let t: CGFloat = 8
        let sideX: CGFloat = 30
        // Side panel running back from the front line at y = 30.
        board(sideX, 30, t, 44)
        board(sideX + t, 66, 50, 6) // part of the back
        switch choice {
        case "full": board(sideX - 1, 30 - t, 60, t, hot: true)
        case "half": board(sideX + t / 2, 30 - t, 56, t, hot: true)
        default: board(sideX + t + 1.5, 30, 54, t, hot: true)
        }
        line(p(10, 30), p(94, 30), colour: .primary.opacity(0.25), dash: [2, 3])
    }

    // MARK: Fixings, as a corner joint close-up

    func fixing(_ choice: String) {
        // Side (vertical) and bottom (horizontal) meeting at a corner, seen from the front.
        board(18, 8, 12, 64)
        board(30, 54, 62, 12)
        let midY: CGFloat = 60
        func biscuit(_ x: CGFloat) {
            let e = Path(ellipseIn: r(x - 9, midY - 3, 18, 6))
            ctx.fill(e, with: .color(accent.opacity(0.55)))
            ctx.stroke(e, with: .color(accent), lineWidth: 1)
        }
        func screw(_ y: CGFloat, thick: Bool = false) {
            line(p(14, y), p(54, y), colour: accent, width: thick ? 3 : 1.6)
            line(p(14, y - 3.5), p(14, y + 3.5), colour: accent, width: 2)
        }
        switch choice {
        case "biscuit_and_screw":
            // In the joint they alternate along its length; drawn one above the other here.
            biscuit(30)
            screw(midY - 9)
        case "biscuit_only": biscuit(30)
        case "screw_only": screw(midY)
        case "confirmat": screw(midY, thick: true)
        case "dowel":
            ctx.fill(Path(r(22, midY - 2.5, 26, 5)), with: .color(accent.opacity(0.6)))
            ctx.stroke(Path(r(22, midY - 2.5, 26, 5)), with: .color(accent), lineWidth: 1)
        case "cam_lock":
            let cam = Path(ellipseIn: r(42, midY - 7, 14, 14))
            ctx.stroke(cam, with: .color(accent), lineWidth: 1.6)
            line(p(18, midY), p(49, midY), colour: accent, width: 1.6)
        default: break
        }
    }

    func biscuit(_ choice: String) {
        let (len, wid): (CGFloat, CGFloat) = switch choice {
        case "no0": (47, 15)
        case "no10": (53, 19)
        default: (56, 23)
        }
        let k: CGFloat = 1.3
        let e = Path(ellipseIn: r(50 - len * k / 2, 40 - wid * k / 2, len * k, wid * k))
        ctx.fill(e, with: .color(accent.opacity(0.35)))
        ctx.stroke(e, with: .color(accent), lineWidth: 1.2)
        // Grain lines across the biscuit, as a pressed beech biscuit has.
        for i in stride(from: -2, through: 2, by: 1) {
            let x = 50 + CGFloat(i) * len * k / 7
            line(p(x - 4, 40 - wid * k / 3), p(x + 4, 40 + wid * k / 3), colour: accent.opacity(0.5), width: 0.7)
        }
    }

    func density(_ choice: String) {
        let gap: CGFloat = switch choice {
        case "sparse": 30
        case "dense": 14
        default: 21
        }
        board(6, 34, 88, 12)
        var x: CGFloat = 14
        while x <= 86 {
            let e = Path(ellipseIn: r(x - 5, 37, 10, 6))
            ctx.fill(e, with: .color(accent.opacity(0.55)))
            ctx.stroke(e, with: .color(accent), lineWidth: 1)
            x += gap
        }
    }

    func unknown() {
        board(30, 20, 40, 40)
    }

    private func arrowText(_ s: String, at point: CGPoint) {
        ctx.draw(Text(s).font(.system(size: 7)).foregroundStyle(.secondary), at: point, anchor: .leading)
    }
}

/// Front elevations for the front layout tiles.
struct FrontThumbnail: View {
    let layout: String
    let hinge: String
    let ratios: [Double]

    var body: some View {
        Canvas { ctx, size in
            let r = CGRect(origin: .zero, size: size).insetBy(dx: 1, dy: 1)
            let frame = Path(roundedRect: r, cornerRadius: 3)
            ctx.fill(frame, with: .color(.primary.opacity(layout == "open" ? 0.03 : 0.10)))
            ctx.stroke(frame, with: .color(.primary.opacity(0.75)), lineWidth: 1.2)
            func hLine(_ y: CGFloat) {
                var p = Path(); p.move(to: CGPoint(x: r.minX, y: y)); p.addLine(to: CGPoint(x: r.maxX, y: y))
                ctx.stroke(p, with: .color(.primary.opacity(0.75)), lineWidth: 1.2)
            }
            func handle(_ x: CGFloat, _ y: CGFloat, vertical: Bool) {
                let rect = vertical ? CGRect(x: x - 1, y: y - 6, width: 2, height: 12) : CGRect(x: x - 7, y: y - 1, width: 14, height: 2)
                ctx.fill(Path(roundedRect: rect, cornerRadius: 1), with: .color(.accentColor))
            }
            switch layout {
            case "door":
                handle(hinge == "left" ? r.maxX - 6 : r.minX + 6, r.minY + 12, vertical: true)
            case "door_pair":
                var p = Path(); p.move(to: CGPoint(x: r.midX, y: r.minY)); p.addLine(to: CGPoint(x: r.midX, y: r.maxY))
                ctx.stroke(p, with: .color(.primary.opacity(0.75)), lineWidth: 1.2)
                handle(r.midX - 5, r.minY + 12, vertical: true)
                handle(r.midX + 5, r.minY + 12, vertical: true)
            case "drawers":
                let total = ratios.reduce(0, +)
                var y = r.minY
                for (i, ratio) in ratios.enumerated() {
                    let h = r.height * ratio / max(total, 0.001)
                    handle(r.midX, y + min(h / 2, 8), vertical: false)
                    y += h
                    if i < ratios.count - 1 { hLine(y) }
                }
            default:
                for i in 1..<3 { hLine(r.minY + r.height * CGFloat(i) / 3) }
            }
        }
    }
}
