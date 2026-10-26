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
                    .accessibilityLabel("\(group.title): \(choice.label)")
                    .accessibilityHint(choice.description)
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
            case "drawer_system": pen.drawerSystem(choice)
            case "drawer_sides": pen.drawerSides(choice)
            case "front_grain": pen.frontGrain(choice)
            case "insert_design": pen.insertDesign(choice)
            case "door_style": pen.doorStyle(choice)
            case "handle_size": pen.handleSize(choice)
            case "handle_place": pen.handlePlace(choice)
            case "handle_finish": pen.handleFinish(choice)
            case "overlay": pen.overlayPlan(choice)
            case "fixings": pen.fixing(choice)
            case "biscuit": pen.biscuit(choice)
            case "domino": pen.domino(choice)
            case "dowel": pen.dowel(choice)
            case "dowel_spacing": pen.dowelSpacing(choice)
            case "fixing_density": pen.density(choice)
            default: pen.unknown()
            }
        }
        .accessibilityHidden(true)
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

    // MARK: Drawer system, a drawer box in section from the front

    func drawerSystem(_ choice: String) {
        // The cabinet's sides, and the drawer box between them on its runners.
        board(8, 10, 6, 62)
        board(86, 10, 6, 62)
        switch choice {
        case "legrabox", "merivobox", "tandembox":
            // Steel sides with the runner hidden under them; a cut base and the back.
            board(19, 30, 4, 34, hot: true)
            board(77, 30, 4, 34, hot: true)
            board(23, 58, 54, 5)
            board(25, 36, 50, 22)
            line(p(14, 66), p(23, 66), colour: accent, width: 2)
            line(p(77, 66), p(86, 66), colour: accent, width: 2)
        default:
            // Wooden sides standing on runners under the base.
            board(18, 28, 8, 34, hot: true)
            board(74, 28, 8, 34, hot: true)
            board(26, 28, 48, 30)
            board(18, 58, 64, 5)
            board(14, 64, 12, 4, hot: true)
            board(74, 64, 12, 4, hot: true)
        }
    }

    /// A door from the front: one board, or a frame round a recessed panel.
    func doorStyle(_ choice: String) {
        board(28, 6, 44, 68, hot: choice == "slab")
        guard choice != "slab" else { return }
        board(28, 6, 9, 68, hot: true)        // stiles
        board(63, 6, 9, 68, hot: true)
        board(37, 6, 26, 9, hot: true)        // rails
        board(37, 65, 26, 9, hot: true)
        if choice == "glass_frame" {
            // Glass: pale, with a glint.
            ctx.fill(Path(r(37, 15, 26, 50)), with: .color(.cyan.opacity(0.15)))
            line(p(42, 22), p(52, 34), colour: .white.opacity(0.9), width: 1.2)
        } else {
            ctx.fill(Path(r(38, 16, 24, 48)), with: .color(.primary.opacity(0.18)))
        }
    }

    /// A drawer front with the handle to scale against it (a 600 mm front is 80 units).
    func handleSize(_ choice: String) {
        board(10, 26, 80, 30)
        guard let cc = Double(choice.dropFirst(2)) else {
            ctx.fill(Path(ellipseIn: r(45, 36, 10, 10)), with: .color(accent.opacity(0.35)))
            ctx.stroke(Path(ellipseIn: r(45, 36, 10, 10)), with: .color(accent), lineWidth: 1.4)
            return
        }
        let w = CGFloat(cc) * 80 / 600
        board(50 - w / 2 - 3, 39, w + 6, 4, hot: true)
        for x in [50 - w / 2, 50 + w / 2] {
            ctx.fill(Path(ellipseIn: r(x - 1, 40, 2, 2)), with: .color(accent))
        }
    }

    /// A handle bar in the finish's colour on a door.
    func handleFinish(_ choice: String) {
        board(24, 8, 52, 64)
        let colour: Color = switch choice {
        case "black": Color(white: 0.1)
        case "brushed_brass": Color(red: 0.76, green: 0.6, blue: 0.33)
        case "brushed_nickel": Color(red: 0.72, green: 0.71, blue: 0.67)
        default: Color(white: 0.75)
        }
        ctx.fill(Path(roundedRect: r(64, 14, 4, 26), cornerRadius: 2 * s), with: .color(colour))
        ctx.stroke(Path(roundedRect: r(64, 14, 4, 26), cornerRadius: 2 * s), with: .color(.primary.opacity(0.6)), lineWidth: 0.8)
    }

    /// A base door hinged on the left and a drawer, with the handles where the choice puts them.
    func handlePlace(_ choice: String) {
        board(12, 8, 36, 64)       // door
        board(56, 8, 32, 26)       // drawer fronts
        board(56, 38, 32, 34)
        switch choice {
        case "edge_pull":
            board(30, 5, 16, 3, hot: true)
            board(64, 5, 16, 3, hot: true)
            board(64, 35, 16, 3, hot: true)
        case "finger_pull":
            // The grip along each front's top edge, shaded.
            board(12, 8, 36, 4, hot: true)
            board(56, 8, 32, 4, hot: true)
            board(56, 38, 32, 4, hot: true)
        case "centre":
            board(41, 32, 2.5, 16, hot: true)
            board(64, 20, 16, 2.5, hot: true)
            board(64, 54, 16, 2.5, hot: true)
        default:
            board(41, 12, 2.5, 16, hot: true)
            board(64, 20, 16, 2.5, hot: true)
            board(64, 42, 16, 2.5, hot: true)
        }
    }

    /// A deep drawer from the side: the tall front on the left, the box's steel side behind it.
    func drawerSides(_ choice: String) {
        board(12, 10, 6, 62)                          // front
        board(18, 64, 70, 5)                          // base
        let top: CGFloat = choice == "high_fronted" ? 14 : 44
        board(18, top, 66, 64 - top, hot: true)       // steel side
        line(p(18, 72), p(88, 72), colour: accent, width: 2)
    }

    /// A drawer insert from above: a frame with two dividers, in steel lines or wood boards.
    func insertDesign(_ choice: String) {
        let (x0, y0, w, h): (CGFloat, CGFloat, CGFloat, CGFloat) = (14, 12, 72, 56)
        if choice == "wood" {
            board(x0, y0, w, 5, hot: true)
            board(x0, y0 + h - 5, w, 5, hot: true)
            board(x0, y0, 5, h, hot: true)
            board(x0 + w - 5, y0, 5, h, hot: true)
            board(x0 + 26, y0 + 5, 4, h - 10, hot: true)
            board(x0 + 46, y0 + 5, 4, h - 10, hot: true)
        } else {
            ctx.stroke(Path(r(x0, y0, w, h)), with: .color(accent), lineWidth: 1.5)
            line(p(x0 + 28, y0), p(x0 + 28, y0 + h), colour: accent, width: 1.5)
            line(p(x0 + 48, y0), p(x0 + 48, y0 + h), colour: accent, width: 1.5)
        }
    }

    /// A door over two drawer fronts, with their grain: upright and running on through all three,
    /// or upright on the door and across the drawers.
    func frontGrain(_ choice: String) {
        let fronts: [(CGFloat, CGFloat)] = [(6, 40), (48, 12), (62, 12)]
        for (y, h) in fronts { board(30, y, 40, h) }
        let matched = choice == "matched"
        for x in stride(from: CGFloat(36), through: 64, by: 7) {
            for (i, (y, h)) in fronts.enumerated() where matched || i == 0 {
                line(p(x, y + 2), p(x, y + h - 2), colour: matched ? accent : nil, width: 1)
            }
        }
        if !matched {
            for (y, h) in fronts.dropFirst() {
                for dy in stride(from: CGFloat(3), to: h - 1, by: 3) {
                    line(p(34, y + dy), p(66, y + dy), width: 1)
                }
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
        case "domino":
            // A flat tenon across the joint, shallow in the side and deeper in the bottom.
            let tenon = Path(roundedRect: r(21, midY - 3, 31, 6), cornerRadius: 3 * s)
            ctx.fill(tenon, with: .color(accent.opacity(0.55)))
            ctx.stroke(tenon, with: .color(accent), lineWidth: 1)
        default: break
        }
    }

    /// A fluted dowel to scale (diameter × length from the choice id, `d8x35`), with chamfered
    /// ends and its flutes.
    func dowel(_ choice: String) {
        let dims = choice.dropFirst().split(separator: "x").compactMap { Double($0) }
        guard dims.count == 2 else { return unknown() }
        let (dia, long) = (CGFloat(dims[0]), CGFloat(dims[1]))
        let k: CGFloat = 1.5
        let w = long * k, h = dia * k * 1.4
        let body = Path(roundedRect: r(50 - w / 2, 40 - h / 2, w, h), cornerRadius: 2 * s)
        ctx.fill(body, with: .color(accent.opacity(0.35)))
        ctx.stroke(body, with: .color(accent), lineWidth: 1.2)
        for i in 1..<4 {
            let y = 40 - h / 2 + h * CGFloat(i) / 4
            line(p(50 - w / 2 + 3, y), p(50 + w / 2 - 3, y), colour: accent.opacity(0.5), width: 0.7)
        }
    }

    /// A joint seen from above with its dowels: on the 32 mm grid (ticks) or spread evenly.
    func dowelSpacing(_ choice: String) {
        board(6, 34, 88, 12)
        let xs: [CGFloat] = choice == "jig" ? [14, 38, 62, 86] : [14, 30, 62, 78]
        if choice != "jig" {
            for x in stride(from: CGFloat(14), through: 86, by: 8) {
                line(p(x, 28), p(x, 32), colour: .primary.opacity(0.4), width: 0.7)
            }
        }
        for x in xs {
            let e = Path(ellipseIn: r(x - 3, 37, 6, 6))
            ctx.fill(e, with: .color(accent.opacity(0.6)))
            ctx.stroke(e, with: .color(accent), lineWidth: 1)
        }
    }

    /// A DOMINO tenon to scale (thickness × length from the choice id, `d8x40`), seen on its
    /// flat, with the glue grooves along it.
    func domino(_ choice: String) {
        let dims = choice.dropFirst().split(separator: "x").compactMap { Double($0) }
        guard dims.count == 2 else { return unknown() }
        let (thick, long) = (CGFloat(dims[0]), CGFloat(dims[1]))
        let k = min(84 / long, 3.2)
        let w = long * k, h = max(thick * k * 1.6, 6)
        let body = Path(roundedRect: r(50 - w / 2, 40 - h / 2, w, h), cornerRadius: h / 2 * s)
        ctx.fill(body, with: .color(accent.opacity(0.35)))
        ctx.stroke(body, with: .color(accent), lineWidth: 1.2)
        for dy in [-h / 5, h / 5] {
            line(p(50 - w / 2 + h / 2, 40 + dy), p(50 + w / 2 - h / 2, 40 + dy),
                 colour: accent.opacity(0.5), width: 0.7)
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
            case "lift":
                // One front swinging up: an arc from its bottom edge.
                var arc = Path()
                arc.addArc(center: CGPoint(x: r.midX, y: r.minY + 4), radius: r.height * 0.55,
                           startAngle: .degrees(100), endAngle: .degrees(160), clockwise: false)
                ctx.stroke(arc, with: .color(.accentColor), lineWidth: 1.4)
                handle(r.midX, r.maxY - 6, vertical: false)
            case "bi_fold_lift":
                // Two fronts folding up at the middle: the fold line and an arrow up.
                hLine(r.midY)
                var arrow = Path()
                arrow.move(to: CGPoint(x: r.midX, y: r.midY - 4))
                arrow.addLine(to: CGPoint(x: r.midX, y: r.minY + 6))
                arrow.move(to: CGPoint(x: r.midX - 4, y: r.minY + 10))
                arrow.addLine(to: CGPoint(x: r.midX, y: r.minY + 6))
                arrow.addLine(to: CGPoint(x: r.midX + 4, y: r.minY + 10))
                ctx.stroke(arrow, with: .color(.accentColor), lineWidth: 1.4)
                handle(r.midX, r.maxY - 6, vertical: false)
            case "drawers":
                let total = ratios.reduce(0, +)
                var y = r.minY
                for (i, ratio) in ratios.enumerated() {
                    let h = r.height * ratio / max(total, 0.001)
                    handle(r.midX, y + min(h / 2, 8), vertical: false)
                    y += h
                    if i < ratios.count - 1 { hLine(y) }
                }
            case "oven_tower":
                // A door at the top, the oven's dark glass, two drawers.
                hLine(r.minY + r.height * 0.4)
                hLine(r.minY + r.height * 0.7)
                hLine(r.minY + r.height * 0.85)
                ctx.fill(Path(CGRect(x: r.minX + 3, y: r.minY + r.height * 0.43, width: r.width - 6, height: r.height * 0.24)),
                         with: .color(.primary.opacity(0.55)))
                handle(r.maxX - 6, r.minY + r.height * 0.32, vertical: true)
            case "fridge":
                hLine(r.minY + r.height * 0.2)
                ctx.fill(Path(CGRect(x: r.minX + 3, y: r.minY + r.height * 0.24, width: r.width - 6, height: r.height * 0.72)),
                         with: .color(.primary.opacity(0.18)))
                handle(r.maxX - 6, r.minY + r.height * 0.12, vertical: false)
            default:
                for i in 1..<3 { hLine(r.minY + r.height * CGFloat(i) / 3) }
            }
        }
        .accessibilityHidden(true)
    }
}
