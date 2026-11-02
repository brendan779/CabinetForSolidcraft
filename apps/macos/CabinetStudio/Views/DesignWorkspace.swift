import RealityKit
import SwiftUI

struct DesignWorkspace: View {
    @Environment(ProjectModel.self) private var project
    @State private var look = ViewportLook()
    @State private var hovered: HoveredPart?
    @State private var tool = ViewTool.orbit
    @State private var scene = ViewportScene()

    var body: some View {
        ZStack(alignment: .topLeading) {
            CabinetViewport(look: look, tool: tool, scene: scene, hovered: $hovered)
            VStack(alignment: .leading, spacing: 8) {
                HStack(alignment: .top, spacing: 8) {
                    ViewToolbar(tool: $tool, scene: scene)
                    VStack(alignment: .leading, spacing: 8) {
                        ViewportControls(look: $look)
                        ViewPresets(scene: scene)
                        ViewHint()
                    }
                }
                if let cab = project.selected, let b = project.build(cab) {
                    BuildBadge(summary: b)
                }
            }
            .padding(14)
            VStack {
                Spacer()
                RunStrip()
                    .padding(.horizontal, 14)
                    .padding(.bottom, 12)
            }
            if let hovered {
                PartTooltip(part: hovered.panel)
                    .position(x: hovered.at.x + 90, y: hovered.at.y + 34)
                    .allowsHitTesting(false)
                    .transition(.opacity)
            }
        }
        .animation(.snappy(duration: 0.15), value: hovered?.panel.name)
        // Arrows step through the cabinets and ⌫ deletes, while the viewport has focus (it takes
        // it on a click).
        .focusable()
        .focusEffectDisabled()
        .onKeyPress(keys: [.leftArrow, .upArrow, .rightArrow, .downArrow]) { press in
            project.selectNeighbour(press.key == .leftArrow || press.key == .upArrow ? -1 : 1)
            return .handled
        }
        .onKeyPress(keys: [.delete, .deleteForward]) { _ in
            project.deleteSelection()
            return .handled
        }
        .onKeyPress(characters: CharacterSet(charactersIn: "fF")) { _ in
            scene.fit()
            return .handled
        }
        .background(
            LinearGradient(colors: [Theme.viewportTop, Theme.viewportBottom], startPoint: .top, endPoint: .bottom)
        )
    }
}

/// How the viewport shows the job: fronts on and open, exploded, cut in section.
struct ViewportLook: Equatable {
    var showFronts = true
    var open = false
    /// 0 assembled, 1 every panel pulled 250 mm away from its cabinet's centre.
    var explode = 0.0
    /// A vertical cut across the run at the selected cabinet's middle: what is left of it is cut
    /// away.
    var section = false
    /// Hinges, runners, legs: the supplier's own models where prepared, stand-ins otherwise.
    var showHardware = true
}

struct HoveredPart: Equatable {
    var panel: PanelBox
    var at: CGPoint

    static func == (a: Self, b: Self) -> Bool { a.panel.name == b.panel.name && a.at == b.at }
}

/// What dragging in the viewport does. Middle-drag or Shift-drag always pans.
enum ViewTool: String, CaseIterable, Identifiable {
    case orbit = "Orbit"
    case pan = "Pan"
    case zoom = "Zoom"

    var id: String { rawValue }

    var symbol: String {
        switch self {
        case .orbit: "rotate.3d"
        case .pan: "hand.draw"
        case .zoom: "plus.magnifyingglass"
        }
    }
}

struct ViewToolbar: View {
    @Binding var tool: ViewTool
    let scene: ViewportScene

    var body: some View {
        VStack(spacing: 2) {
            ForEach(ViewTool.allCases) { t in
                Button {
                    tool = t
                } label: {
                    Image(systemName: t.symbol).frame(width: 24, height: 20)
                }
                .buttonStyle(ChipStyle(selected: tool == t))
                .help("\(t.rawValue): drag in the view. Middle-drag or Shift-drag pans, scroll zooms, F fits.")
                .accessibilityLabel(t.rawValue)
            }
            Button {
                scene.fit()
            } label: {
                Image(systemName: "arrow.up.left.and.down.right.magnifyingglass").frame(width: 24, height: 20)
            }
            .buttonStyle(ChipStyle(selected: false))
            .help("Fit the job in the view (F)")
            .accessibilityLabel("Fit")
        }
        .padding(4)
        .floatingPanel()
        .fixedSize()
    }
}

/// Standard views.
struct ViewPresets: View {
    let scene: ViewportScene

    var body: some View {
        HStack(spacing: 2) {
            ForEach(ViewPreset.allCases) { p in
                Button(p.rawValue) { scene.show(p) }
                    .buttonStyle(ChipStyle(selected: false))
                    .help("Look at the job from \(p.rawValue.lowercased())")
            }
        }
        .padding(4)
        .floatingPanel()
        .fixedSize()
    }
}

/// How to move the view, in sight: nobody should have to find Shift-drag by trial and error.
struct ViewHint: View {
    var body: some View {
        Text("Shift-drag or middle-drag pans · scroll zooms · F fits")
            .font(.caption2)
            .foregroundStyle(Theme.textDim)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .floatingPanel()
            .fixedSize()
    }
}

enum ViewPreset: String, CaseIterable, Identifiable {
    case iso = "3D"
    case front = "Front"
    case left = "Left"
    case right = "Right"
    case top = "Top"

    var id: String { rawValue }

    /// (yaw, pitch) in radians. Yaw 0 looks at the fronts.
    var angles: (Float, Float) {
        switch self {
        case .iso: (-0.46, 0.29)
        case .front: (0, 0.03)
        case .left: (-.pi / 2, 0.03)
        case .right: (.pi / 2, 0.03)
        case .top: (0, ViewportScene.maxPitch)
        }
    }
}

struct ViewportControls: View {
    @Binding var look: ViewportLook

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 4) {
                Button("Fronts") { withAnimation(.snappy) { look.showFronts.toggle() } }
                    .buttonStyle(ChipStyle(selected: look.showFronts))
                    .help("Show doors and drawer fronts")
                Button("Open") { look.open.toggle() }
                    .buttonStyle(ChipStyle(selected: look.open))
                    .disabled(!look.showFronts)
                    .help("Swing the doors open and pull the drawers out")
                Button("Section") { look.section.toggle() }
                    .buttonStyle(ChipStyle(selected: look.section))
                    .help("Cut the run through the middle of the selected cabinet")
                Button("Hardware") { withAnimation(.snappy) { look.showHardware.toggle() } }
                    .buttonStyle(ChipStyle(selected: look.showHardware))
                    .help("Show hinges, runners and legs (Blum's own models where the supplier CAD is prepared)")
            }
            HStack(spacing: 6) {
                Image(systemName: "arrow.up.left.and.arrow.down.right")
                    .foregroundStyle(Theme.textDim)
                    .help("Explode: pull the panels apart")
                Slider(value: $look.explode, in: 0...1)
                    .frame(width: 110)
                    .accessibilityLabel("Explode")
            }
            .controlSize(.small)
        }
        .padding(8)
        .floatingPanel()
        .fixedSize()
    }
}

/// The run along the bottom of the viewport, to scale: drag a cabinet to move it along the run,
/// drag its right edge to change its width (snapping to the usual sizes). A drag is one edit.
struct RunStrip: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    /// The item being resized and its width so far.
    @State private var resizing: (id: String, width: Double)?
    /// The item being moved and how far.
    @State private var moving: (id: String, dx: CGFloat)?

    private static let cabinetWidths: [Double] = [150, 200, 300, 400, 450, 500, 600, 700, 800, 900, 1000, 1200]
    private static let fillerWidths: [Double] = [25, 38, 50, 75, 100, 150, 200]

    private var items: [CabinetDoc] {
        project.cabinets.filter {
            $0.placed != true && !["end_panel", "benchtop", "cornice", "light_rail", "walls", "splashback"].contains($0.kind)
        }
    }

    /// The floor line and the line hung above it, as they stand along the wall.
    private var floorLine: [CabinetDoc] { items.filter { !$0.isUpper } }
    private var upperLine: [CabinetDoc] { items.filter(\.isUpper) }

    private func width(_ c: CabinetDoc) -> Double {
        resizing?.id == c.id ? resizing!.width : c.width
    }

    private func length(_ line: [CabinetDoc]) -> Double {
        line.reduce(0) { $0 + width($1) }
    }

    var body: some View {
        let upper = upperLine, floor = floorLine
        GeometryReader { geo in
            let total = max(length(upper), length(floor), 1)
            let scale = (geo.size.width - CGFloat(max(max(upper.count, floor.count) - 1, 0)) * 2) / total
            VStack(spacing: 2) {
                if !upper.isEmpty { row(upper, scale: scale) }
                if !floor.isEmpty { row(floor, scale: scale) }
            }
        }
        .frame(height: upper.isEmpty || floor.isEmpty ? 34 : 70)
        .padding(6)
        .floatingPanel()
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Run")
    }

    private func row(_ line: [CabinetDoc], scale: CGFloat) -> some View {
        HStack(spacing: 2) {
            ForEach(line) { c in
                block(c, width: CGFloat(width(c)) * scale, scale: scale, line: line)
            }
            Spacer(minLength: 0)
        }
        .frame(height: 34)
    }

    private func block(_ c: CabinetDoc, width: CGFloat, scale: CGFloat, line: [CabinetDoc]) -> some View {
        let selected = c.id == project.selection
        return ZStack(alignment: .trailing) {
            RoundedRectangle(cornerRadius: 5)
                .fill(selected ? Theme.selected : (c.isPiece ? Theme.strip : Theme.control))
                .overlay(
                    RoundedRectangle(cornerRadius: 5).strokeBorder(
                        selected ? Theme.accent : Theme.border,
                        // A gap is empty wall: dashed, so it does not read as a cabinet.
                        style: StrokeStyle(lineWidth: 1, dash: c.kind == "gap" ? [4, 3] : [])
                    )
                )
                .overlay {
                    if width > 46 {
                        VStack(spacing: 0) {
                            Text(c.id).font(.caption2.weight(.semibold)).monospaced()
                            Text(unit.format(self.width(c), suffix: false)).font(.caption2.monospacedDigit())
                                .foregroundStyle(.secondary)
                        }
                        .lineLimit(1)
                    }
                }
            // The width handle.
            Rectangle()
                .fill(Color.clear)
                .frame(width: 8)
                .contentShape(Rectangle())
                .overlay(Capsule().fill(selected ? Color.accentColor : .secondary).frame(width: 3, height: 16))
                .pointerStyle(.frameResize(position: .trailing))
                .gesture(
                    DragGesture(minimumDistance: 1)
                        .onChanged { v in
                            resizing = (c.id, snap(c, c.width + Double(v.translation.width / scale)))
                        }
                        .onEnded { _ in
                            if let r = resizing, abs(r.width - c.width) > 0.5,
                               let i = project.cabinets.firstIndex(where: { $0.id == c.id }) {
                                withAnimation(.snappy) {
                                    project.cabinets[i].width = r.width
                                    project.selection = c.id
                                }
                                project.changed()
                            }
                            resizing = nil
                        }
                )
                .accessibilityLabel("\(c.id) width")
                .accessibilityValue(unit.format(c.width))
                .accessibilityAdjustableAction { direction in
                    let list = c.kind == "filler" ? Self.fillerWidths : Self.cabinetWidths
                    let next = direction == .increment ? list.first { $0 > c.width } : list.last { $0 < c.width }
                    if let next, let i = project.cabinets.firstIndex(where: { $0.id == c.id }) {
                        project.cabinets[i].width = next
                        project.changed()
                    }
                }
        }
        .frame(width: max(width, 6))
        .offset(x: moving?.id == c.id ? moving!.dx : 0)
        .zIndex(moving?.id == c.id ? 1 : 0)
        .onTapGesture { withAnimation(.snappy) { project.selection = c.id } }
        .gesture(
            DragGesture(minimumDistance: 4)
                .onChanged { v in moving = (c.id, v.translation.width) }
                .onEnded { v in
                    reorder(c, by: Double(v.translation.width / scale), in: line)
                    moving = nil
                }
        )
        .help("\(c.id): drag to move along the run, drag the right edge to change the width")
    }

    /// The nearest usual width: cabinet sizes for cabinets, filler sizes for fillers.
    private func snap(_ c: CabinetDoc, _ w: Double) -> Double {
        let list = c.kind == "filler" ? Self.fillerWidths : Self.cabinetWidths
        return list.min { abs($0 - w) < abs($1 - w) } ?? w
    }

    /// Moves `c` to where its middle lands after a drag of `dx` mm along the run.
    private func reorder(_ c: CabinetDoc, by dx: Double, in run: [CabinetDoc]) {
        guard let from = run.firstIndex(where: { $0.id == c.id }) else { return }
        var x = 0.0
        var centres: [Double] = []
        for item in run {
            centres.append(x + item.width / 2)
            x += item.width
        }
        let target = centres[from] + dx
        let others = run.indices.filter { $0 != from }
        let slot = others.filter { centres[$0] < target }.count
        guard slot != from else { return }
        // Indices in the whole job: before the run item now at `slot`, or after the last one.
        let all = project.cabinets
        guard let source = all.firstIndex(where: { $0.id == c.id }) else { return }
        let remaining = others.map { run[$0].id }
        let dest: Int = slot < remaining.count
            ? all.firstIndex { $0.id == remaining[slot] }!
            : all.firstIndex { $0.id == remaining.last! }! + 1
        withAnimation(.snappy) {
            project.move(from: IndexSet(integer: source), to: dest)
            project.selection = c.id
        }
    }
}

/// The part under the pointer: its name and finished size.
struct PartTooltip: View {
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    let part: PanelBox

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(part.name).font(.callout.weight(.semibold)).monospaced()
            Text(part.role).font(.caption).foregroundStyle(.secondary)
            Text([part.width, part.height, part.depth].map { unit.format($0, suffix: false) }.joined(separator: " × "))
                .font(.caption.monospacedDigit())
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .floatingPanel()
        .fixedSize()
    }
}

/// Counts for the selected cabinet, floating over the viewport.
struct BuildBadge: View {
    let summary: BuildSummary
    @State private var showingNotes = false

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Label("\(summary.panels.count) panels", systemImage: "square.3.layers.3d")
            Label("\(summary.biscuits) biscuits · \(summary.screws) screws", systemImage: "link")
            Label("\(summary.operations) machining ops", systemImage: "circle.grid.cross")
            if !summary.warnings.isEmpty {
                Button { showingNotes.toggle() } label: {
                    Label(summary.warnings.count == 1 ? "1 note" : "\(summary.warnings.count) notes",
                          systemImage: "exclamationmark.bubble")
                        .foregroundStyle(Theme.warning)
                }
                .buttonStyle(.plain)
                .help("Show the notes")
                .popover(isPresented: $showingNotes, arrowEdge: .trailing) {
                    ScrollView {
                        VStack(alignment: .leading, spacing: 8) {
                            ForEach(summary.warnings, id: \.self) { note in
                                Text(note).fixedSize(horizontal: false, vertical: true)
                            }
                        }
                        .padding(12)
                    }
                    .frame(width: 340)
                    .frame(maxHeight: 300)
                }
            }
        }
        .font(.callout)
        .padding(10)
        .floatingPanel()
    }
}

extension View {
    /// A panel floating over the viewport.
    func floatingPanel() -> some View {
        background(Theme.overlay, in: .rect(cornerRadius: 9))
            .overlay(RoundedRectangle(cornerRadius: 9).strokeBorder(Theme.border))
    }
}

/// Pointer input RealityView's own gestures do not cover: the wheel, pinch and the middle button.
/// Local monitors, since SwiftUI has no gesture for them; only acts in this view's window while
/// the pointer is over the viewport.
@MainActor
final class ViewportEvents {
    weak var window: NSWindow?
    var inside = false
    private var monitor: Any?

    func start(scene: ViewportScene) {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(
            matching: [.scrollWheel, .magnify, .otherMouseDown, .otherMouseDragged]
        ) { [weak self, weak scene] event in
            guard let self, let scene, event.window === self.window, self.inside else { return event }
            switch event.type {
            case .scrollWheel:
                // A mouse wheel's notches are coarse next to a trackpad's points.
                let step: CGFloat = event.hasPreciseScrollingDeltas ? 1 : 10
                if event.modifierFlags.contains(.shift) {
                    scene.pan(dx: event.scrollingDeltaX * step, dy: event.scrollingDeltaY * step)
                } else {
                    scene.zoom(by: exp(Float(-event.scrollingDeltaY * step) * 0.006))
                }
            case .magnify:
                scene.zoom(by: 1 / max(0.2, 1 + Float(event.magnification)))
            case .otherMouseDragged:
                scene.pan(dx: event.deltaX, dy: event.deltaY)
            default:
                break
            }
            return nil
        }
    }

    func stop() {
        if let monitor { NSEvent.removeMonitor(monitor) }
        monitor = nil
    }
}

/// Hands back the window its view is in.
private struct WindowReader: NSViewRepresentable {
    let found: (NSWindow?) -> Void

    func makeNSView(context: Context) -> NSView {
        let v = NSView()
        DispatchQueue.main.async { [weak v] in found(v?.window) }
        return v
    }

    func updateNSView(_ v: NSView, context: Context) {}
}

struct CabinetViewport: View {
    @Environment(ProjectModel.self) private var project
    let look: ViewportLook
    let tool: ViewTool
    let scene: ViewportScene
    @Binding var hovered: HoveredPart?
    @State private var events = ViewportEvents()
    /// The drag so far, to turn its running total into steps.
    @State private var dragged = CGSize.zero
    /// Bumped when a texture finishes loading, to update the scene.
    @State private var textures = 0

    var body: some View {
        GeometryReader { geo in
            RealityView { content in
                content.add(scene.root)
                scene.onTextureLoaded = { textures += 1 }

                let sun = DirectionalLight()
                sun.light.intensity = 3200
                sun.shadow = DirectionalLightComponent.Shadow(maximumDistance: 6, depthBias: 2)
                sun.look(at: .zero, from: [1.2, 3.0, 2.5], relativeTo: nil)
                content.add(sun)

                let fill = DirectionalLight()
                fill.light.intensity = 900
                fill.look(at: .zero, from: [-2.5, 1.5, 1.0], relativeTo: nil)
                content.add(fill)

                // A three-quarter view from front left, far enough back to see a short run.
                scene.camera.camera.fieldOfViewInDegrees = 35
                scene.applyCamera()
                content.add(scene.camera)

                let floor = ModelEntity(
                    mesh: .generatePlane(width: 12, depth: 12),
                    materials: [SimpleMaterial(color: Theme.floor, roughness: 0.9, isMetallic: false)]
                )
                floor.position.y = -0.0005
                content.add(floor)
            } update: { _ in
                _ = textures
                scene.update(project: project, look: look, hovered: hovered?.panel.name)
            }
            .simultaneousGesture(
                DragGesture(minimumDistance: 2)
                    .onChanged { v in
                        let dx = v.translation.width - dragged.width
                        let dy = v.translation.height - dragged.height
                        dragged = v.translation
                        drag(dx: dx, dy: dy)
                    }
                    .onEnded { _ in dragged = .zero }
            )
            .background(WindowReader { events.window = $0 })
            .onAppear {
                scene.viewSize = geo.size
                events.start(scene: scene)
            }
            .onDisappear { events.stop() }
            .onChange(of: geo.size) { scene.viewSize = geo.size }
            .accessibilityLabel("3D view of the job")
            .accessibilityHint("Drag to orbit, Shift-drag to pan, scroll to zoom, F to fit. Click a cabinet to select it; the arrow keys step through cabinets.")
            .gesture(
                SpatialTapGesture().targetedToAnyEntity().onEnded { value in
                    if let id = scene.cabinet(of: value.entity, in: project) {
                        withAnimation(.snappy) { project.selection = id }
                    }
                }
            )
            .onContinuousHover { phase in
                switch phase {
                case .active(let point):
                    events.inside = true
                    if let name = scene.part(at: point, size: geo.size),
                       let panel = scene.panels[name] {
                        hovered = HoveredPart(panel: panel, at: point)
                    } else {
                        hovered = nil
                    }
                case .ended:
                    events.inside = false
                    hovered = nil
                }
            }
        }
    }

    private func drag(dx: CGFloat, dy: CGFloat) {
        let panning = tool == .pan || NSEvent.modifierFlags.contains(.shift)
        if panning {
            scene.pan(dx: dx, dy: dy)
        } else if tool == .zoom {
            scene.zoom(by: exp(Float(dy) * 0.006))
        } else {
            scene.orbit(dx: dx, dy: dy)
        }
    }
}

/// The viewport's entities, kept across updates so a change moves panels instead of rebuilding
/// them (doors swing, explode slides). One entity per panel, named after the part.
@MainActor
final class ViewportScene {
    let root = Entity()
    let camera = PerspectiveCamera()
    nonisolated static let maxPitch: Float = 1.5
    /// What the camera looks at, and from where around it: yaw 0 faces the fronts, pitch 0 level.
    private var target: SIMD3<Float> = [0, 0.55, -0.3]
    private var yaw = ViewPreset.iso.angles.0
    private var pitch = ViewPreset.iso.angles.1
    private var distance: Float = 5.4
    /// The viewport's size in points, for turning a drag into a distance.
    var viewSize = CGSize(width: 800, height: 600)
    /// The run as last built, in metres: its width, height and depth.
    private var extent: SIMD3<Float> = [1.8, 0.9, 0.6]
    /// The user has orbited, panned or zoomed.
    private var cameraMoved = false
    /// What the walls were last made for.
    private var wallsKey = ""
    /// The panels shown, by part name, as last built.
    private(set) var panels: [String: PanelBox] = [:]
    /// Each panel entity's mesh size and its offset from the pivot, as last made.
    private var shapes: [String: [SIMD3<Float>]] = [:]
    /// Panels drawn with a supplier's mesh: the article, so a changed model rebuilds them.
    private var meshKeys: [String: String] = [:]
    /// The finish a supplier model was made in: its material is set when it is made.
    private var handleTints: [String: String] = [:]
    private var wasOpen = false

    func update(project: ProjectModel, look: ViewportLook, hovered: String?) {
        var seen = Set<String>()
        var minX = Float.greatestFiniteMagnitude, maxX = -Float.greatestFiniteMagnitude
        let sectionX = project.selected.flatMap { cab in
            project.build(cab).map { b in b.panels.map { $0.x + $0.width / 2 }.reduce(0, +) / Double(max(b.panels.count, 1)) }
        }
        let animate = look.open != wasOpen
        wasOpen = look.open
        for cab in project.cabinets {
            guard let b = project.build(cab) else { continue }
            let selected = cab.id == project.selection
            let centre = Self.centre(of: b.panels)
            for p in b.panels {
                // A handle goes and swings with its front.
                let isFront = p.isFront || p.board.hasPrefix("handle:")
                // The kick board stays: without it the run looks as if it floats.
                guard look.showFronts || !isFront || p.role == "Plinth" else { continue }
                var box = p
                if look.section, let cut = sectionX {
                    if p.x + p.width <= cut { continue }
                    if p.x < cut {
                        box.width -= cut - p.x
                        box.x = cut
                    }
                }
                // A rebate along an edge (a side's toe-kick notch, a front's finger pull): the two
                // boxes it leaves.
                let pieces = p.openings.isEmpty ? Self.split(box, by: p.rebate) : Self.around(box, openings: p.openings)
                let board = project.boardsById[p.board]
                for piece in pieces {
                    seen.insert(piece.name)
                    panels[piece.name] = p
                    let e = entity(for: piece)
                    e.model?.materials = [material(piece, board: board, selected: selected, hovered: p.name == hovered)]
                    let target = Self.transform(piece, centre: centre, look: look)
                    if animate && isFront {
                        e.move(to: target, relativeTo: root, duration: 0.45, timingFunction: .easeInOut)
                    } else {
                        e.transform = target
                    }
                }
                minX = min(minX, Float(p.x) / 1000)
                maxX = max(maxX, Float(p.x + p.width) / 1000)
            }
        }
        addHardware(project: project, look: look, sectionX: look.section ? sectionX : nil, seen: &seen)
        updateWalls(project: project, minX: minX, maxX: maxX, seen: &seen)
        // A copy: removing from `root.children` while walking it skips the entity after each one
        // removed (a door's handle, say).
        for e in Array(root.children) where !seen.contains(e.name) {
            e.removeFromParent()
            panels[e.name] = nil
            shapes[e.name] = nil
            meshKeys[e.name] = nil
            handleTints[e.name] = nil
        }
        // Keep the run centred on the origin the camera orbits.
        if minX < maxX {
            // Measured walls hold still, so the view centres on them rather than the run.
            let wallRun = project.cabinets.first { $0.kind == "walls" }.map { Float($0.wallLength) / 1000 } ?? 0
            let (x0, x1) = wallRun > 0 ? (Float(0), wallRun) : (minX, maxX)
            root.position.x = -(x0 + x1) / 2
            let height = panels.values.map { Float($0.z + $0.height) }.max() ?? extent.y
            let depth = panels.values.map { Float($0.y + $0.depth) }.max() ?? extent.z
            let grown: SIMD3<Float> = [x1 - x0, height / 1000, depth / 1000]
            // Until the user takes the camera, keep the whole job in view as it grows.
            if grown != extent {
                extent = grown
                if !cameraMoved { frameJob() }
            }
        }
    }

    // MARK: Hardware

    /// Loaded supplier meshes by file; nil for a file that would not load, so it is not retried
    /// on every redraw.
    private var meshes: [String: MeshResource?] = [:]

    /// Supplier models and stand-in shapes. They sit on the carcass, so they are left out of an
    /// exploded view; hinges show only with the fronts closed, drawer box sides slide out with
    /// their drawers, and nothing shows on the cut-away side of a section.
    /// The back wall and any return walls, behind and beside the run. Glassy, because the camera
    /// often looks through one; not selectable.
    private func updateWalls(project: ProjectModel, minX: Float, maxX: Float, seen: inout Set<String>) {
        guard let w = project.cabinets.first(where: { $0.kind == "walls" }) else { return }
        let run = Float(w.wallLength) / 1000
        // Measured walls stand as measured; unmeasured ones still follow the run.
        let (x0, x1) = run > 0 ? (Float(0), run) : (minX, maxX)
        guard x0 < x1 else { return }
        // The back wall's plane: the deepest cabinet's back. End panels and fillers run past it
        // by their scribes, which are cut off on site.
        let panels = project.cabinets.filter { $0.placed != true }.compactMap { project.build($0) }.flatMap(\.panels)
            .filter { $0.role != "End panel" && $0.role != "Filler" }
        let back = Float(panels.map { $0.y + $0.depth }.max() ?? 600) / 1000
        let height = Float(w.wallHeight) / 1000
        let (left, right) = (Float(w.wallLeftRun) / 1000, Float(w.wallRightRun) / 1000)
        let t: Float = 0.1
        let key = "\(w.wallLeft) \(w.wallRight) \(x0) \(x1) \(back) \(height) \(left) \(right)"
        seen.insert("walls")
        guard key != wallsKey else { return }
        wallsKey = key
        root.children.first { $0.name == "walls" }?.removeFromParent()
        let group = Entity()
        group.name = "walls"
        let paint = SimpleMaterial(color: Theme.wall, roughness: 0.9, isMetallic: false)
        func wall(_ xa: Float, _ xb: Float, _ front: Float, _ rear: Float) {
            let e = ModelEntity(mesh: .generateBox(size: [xb - xa, height, rear - front]), materials: [paint])
            e.position = [(xa + xb) / 2, height / 2, -(front + rear) / 2]
            group.addChild(e)
        }
        wall(x0 - (w.wallLeft ? t : 0), x1 + (w.wallRight ? t : 0), back, back + t)
        if w.wallLeft { wall(x0 - t, x0, back - left, back) }
        if w.wallRight { wall(x1, x1 + t, back - right, back) }
        root.addChild(group)
    }

    private func addHardware(project: ProjectModel, look: ViewportLook, sectionX: Double?, seen: inout Set<String>) {
        guard look.showHardware, look.explode < 0.001 else { return }
        let (models, shapes) = project.hardware()
        func visible(_ name: String, x: Double) -> Bool {
            if let cut = sectionX, x < cut { return false }
            if name.contains(".Hinge") { return look.showFronts && !look.open }
            // In the back of a drawer front: it goes with the front.
            if name.contains(".ServoBumper") { return look.showFronts }
            return true
        }
        func slid(_ t: Transform, _ name: String) -> Transform {
            guard look.open, name.contains(".BoxSide") || name.contains(".ServoBumper") else { return t }
            var t = t
            t.translation.z += Self.drawerTravel
            return t
        }
        let selection = project.selection.map { $0 + "." }
        func selected(_ name: String) -> Bool { selection.map { name.hasPrefix($0) } ?? false }
        for (i, m) in models.enumerated() where visible(m.name, x: m.transform[3]) {
            guard let mesh = mesh(m.mesh) else { continue }
            // Keyed by its file too, so a changed hinge or runner gets its new model.
            let key = "hw:\(m.name)#\(i):\(m.mesh)"
            let e = hardwareEntity(key) { ModelEntity(mesh: mesh) }
            e.model?.materials = [Self.metal(selected: selected(m.name))]
            e.transform = slid(Transform(matrix: Self.matrix(m.transform)), m.name)
            seen.insert(key)
        }
        for (i, sh) in shapes.enumerated() where visible(sh.name, x: sh.x) {
            // Keyed by its size too: the mesh is made once per entity.
            let key = "hw:\(sh.name)#s\(i):\(sh.cylinder ? [sh.diameter, sh.length] : [sh.width, sh.depth, sh.height])"
            let e = hardwareEntity(key) { ModelEntity(mesh: Self.standInMesh(sh)) }
            e.model?.materials = [Self.metal(selected: selected(sh.name), standIn: true)]
            e.transform = slid(Self.standInTransform(sh), sh.name)
            seen.insert(key)
        }
    }

    private func hardwareEntity(_ key: String, make: () -> ModelEntity) -> ModelEntity {
        if let e = root.findEntity(named: key) as? ModelEntity { return e }
        let e = make()
        e.name = key
        root.addChild(e)
        return e
    }

    /// A supplier mesh (STL, model mm), loaded once per file.
    private func mesh(_ path: String) -> MeshResource? {
        if let m = meshes[path] { return m }
        guard let data = try? loadMesh(path: path), !data.positions.isEmpty else {
            meshes[path] = .some(nil)
            return nil
        }
        let p = data.positions, n = data.normals
        var positions: [SIMD3<Float>] = [], normals: [SIMD3<Float>] = []
        positions.reserveCapacity(p.count / 3)
        normals.reserveCapacity(p.count / 3)
        for i in stride(from: 0, to: p.count, by: 9) {
            let a = SIMD3(p[i], p[i + 1], p[i + 2]), b = SIMD3(p[i + 3], p[i + 4], p[i + 5]), c = SIMD3(p[i + 6], p[i + 7], p[i + 8])
            var normal = SIMD3(n[i], n[i + 1], n[i + 2])
            // Some exporters leave the facet normal empty: work it out from the corners.
            if simd_length(normal) < 0.5 {
                let cross = simd_cross(b - a, c - a)
                // A sliver with no area has no direction; it draws nothing either way.
                normal = simd_length(cross) > 0 ? simd_normalize(cross) : [0, 0, 1]
            }
            positions += [a, b, c]
            normals += [normal, normal, normal]
        }
        var desc = MeshDescriptor()
        desc.positions = MeshBuffers.Positions(positions)
        desc.normals = MeshBuffers.Normals(normals)
        desc.primitives = .triangles(Array(0..<UInt32(positions.count)))
        let m = try? MeshResource.generate(from: [desc])
        meshes[path] = .some(m)
        return m
    }

    /// Model mm → cabinet mm (row-major 3 × 4) → RealityKit metres: cabinet (x, y, z) is RealityKit
    /// (x, z, −y).
    private static func matrix(_ m: [Double]) -> simd_float4x4 {
        func rk(_ a: Double, _ b: Double, _ c: Double) -> SIMD3<Float> { SIMD3(Float(a), Float(c), -Float(b)) / 1000 }
        let x = rk(m[0], m[4], m[8]), y = rk(m[1], m[5], m[9]), z = rk(m[2], m[6], m[10])
        let t = rk(m[3], m[7], m[11])
        return simd_float4x4(SIMD4(x, 0), SIMD4(y, 0), SIMD4(z, 0), SIMD4(t, 1))
    }

    private static func standInMesh(_ s: StandInShape) -> MeshResource {
        if s.cylinder {
            return .generateCylinder(height: Float(s.length) / 1000, radius: Float(s.diameter) / 2000)
        }
        return .generateBox(size: SIMD3(Float(s.width), Float(s.height), Float(s.depth)) / 1000)
    }

    private static func standInTransform(_ s: StandInShape) -> Transform {
        guard s.cylinder else {
            return Transform(translation: point(s.x + s.width / 2, s.y + s.depth / 2, s.z + s.height / 2))
        }
        // RealityKit's cylinder runs along its y, which is the cabinet's z.
        let sign = s.positive ? 1.0 : -1.0
        let half = s.length / 2 * sign
        var t = Transform()
        switch s.axis {
        case 0:
            t.translation = point(s.x + half, s.y, s.z)
            t.rotation = simd_quatf(angle: .pi / 2, axis: [0, 0, 1])
        case 1:
            t.translation = point(s.x, s.y + half, s.z)
            t.rotation = simd_quatf(angle: .pi / 2, axis: [1, 0, 0])
        default:
            t.translation = point(s.x, s.y, s.z + half)
        }
        return t
    }

    /// Nickel for supplier models, a flatter grey for stand-ins; dimmer outside the selection.
    private static func metal(selected: Bool, standIn: Bool = false) -> PhysicallyBasedMaterial {
        var m = PhysicallyBasedMaterial()
        let level: CGFloat = standIn ? 0.55 : 0.78
        m.baseColor = .init(tint: NSColor(calibratedWhite: selected ? level : level * 0.8, alpha: 1))
        m.metallic = .init(floatLiteral: standIn ? 0.3 : 0.9)
        m.roughness = .init(floatLiteral: 0.35)
        return m
    }

    /// The panel's entity, made or remade when its size or pivot changes (a door hinged on the
    /// other side keeps its size). Its mesh is centred on its hinge edge for doors (so it swings
    /// about it) and on its middle otherwise. A panel with a supplier mesh (an ABI handle) draws
    /// that model in the model's own placement instead of the box, on the same pivot.
    private func entity(for p: PanelBox) -> ModelEntity {
        let size = SIMD3<Float>(Float(p.width), Float(p.height), Float(p.depth)) / 1000
        // The geometry sits off the entity's origin so the origin is the pivot.
        let offset = Self.mid(p) - Self.pivot(p)
        let supplier = supplierHandle(p)
        // The grain and arch are in the mesh: a change of either makes a new one.
        let meshKey = supplier?.0 ?? "\(p.grain) \(p.arch)"
        if let e = root.findEntity(named: p.name) as? ModelEntity,
           shapes[p.name] == [size, offset], meshKeys[p.name] == meshKey,
           handleTints[p.name] == p.board {
            return e
        }
        root.findEntity(named: p.name)?.removeFromParent()
        let e: ModelEntity
        if let supplier {
            // The model replaces the box; the box stays as the collision shape, so the front
            // still answers the pointer where its handle is.
            e = ModelEntity()
            let child = ModelEntity(mesh: supplier.1)
            // The placement is cabinet-frame mm; the entity sits at its pivot, so the model is
            // local to it — and swings or slides with the front like the bar did.
            var local = Self.matrix(p.meshTransform)
            let pivot = Self.pivot(p)
            local.columns.3.x -= pivot.x
            local.columns.3.y -= pivot.y
            local.columns.3.z -= pivot.z
            child.transform = Transform(matrix: local)
            child.model?.materials = [Self.handleMaterial(Substring(p.board.dropFirst("handle:".count)), selected: false)]
            e.addChild(child)
        } else {
            let mesh = p.name.hasSuffix("~bevel") && p.bevel.count == 3
                ? Self.wedgeMesh(p, size: size, by: offset)
                : p.arch.count == 3
                ? Self.archedMesh(p, size: size, by: offset)
                : Self.offsetMesh(size: size, by: offset, grain: Self.grainAxis(p))
            e = ModelEntity(mesh: mesh, materials: [])
        }
        e.name = p.name
        e.components.set(InputTargetComponent())
        e.collision = CollisionComponent(shapes: [ShapeResource.generateBox(size: size).offsetBy(translation: offset)])
        root.addChild(e)
        shapes[p.name] = [size, offset]
        meshKeys[p.name] = meshKey
        handleTints[p.name] = p.board
        return e
    }

    /// A supplier handle model to draw for `p` (ABI-3): its article and loaded mesh. Nil for
    /// panels without one, or a mesh that will not load — the bar draws then.
    private func supplierHandle(_ p: PanelBox) -> (String, MeshResource)? {
        guard !p.mesh.isEmpty, p.meshTransform.count == 12,
              let path = Preferences.vendorCADDir?.appending(path: "abi/\(p.mesh).stl").path,
              let m = mesh(path) else { return nil }
        // The key holds the placement: the same pull moved (a new handle place) must be redrawn.
        return ("\(p.mesh) \(p.meshTransform)", m)
    }

    /// The panel's grain direction as a RealityKit axis index (x 0, y 1, z 2).
    private static func grainAxis(_ p: PanelBox) -> Int? {
        switch p.grain {
        case "x": 0
        case "y": 2
        case "z": 1
        default: nil
        }
    }

    /// What a sharknose keeps of the box its bevel cuts: the half away from the cut edge, a
    /// wedge along the front's width, each face flat-shaded.
    private static func wedgeMesh(_ p: PanelBox, size: SIMD3<Float>, by offset: SIMD3<Float>) -> MeshResource {
        let h = size / 2
        // The cut corner in the mesh's frame: its side of the box on each cut axis.
        let edge = point(p.bevel[0], p.bevel[1], p.bevel[2]) - mid(p)
        // The edge's mark is halfway along it.
        let along = (0..<3).min { abs(edge[$0]) < abs(edge[$1]) } ?? 0
        let (u, v) = along == 1 ? (0, 2) : (along == 0 ? 2 : 0, 1)
        func corner(_ su: Float, _ sv: Float, _ sa: Float) -> SIMD3<Float> {
            var c = SIMD3<Float>(repeating: 0)
            c[u] = su * h[u]; c[v] = sv * h[v]; c[along] = sa * h[along]
            return c + offset
        }
        let (eu, ev): (Float, Float) = (edge[u] >= 0 ? 1 : -1, edge[v] >= 0 ? 1 : -1)
        // The triangle left once the corner at (eu, ev) is gone.
        let tri: [(Float, Float)] = [(-eu, -ev), (eu, -ev), (-eu, ev)]
        var faces: [[SIMD3<Float>]] = []
        faces.append(tri.map { corner($0.0, $0.1, -1) })
        faces.append(tri.map { corner($0.0, $0.1, 1) })
        for i in 0..<3 {
            let (a, b) = (tri[i], tri[(i + 1) % 3])
            faces.append([corner(a.0, a.1, -1), corner(b.0, b.1, -1), corner(b.0, b.1, 1), corner(a.0, a.1, 1)])
        }
        let centre = (tri.reduce(SIMD3<Float>(repeating: 0)) { $0 + corner($1.0, $1.1, 0) }) / 3
        var positions: [SIMD3<Float>] = [], normals: [SIMD3<Float>] = [], uvs: [SIMD2<Float>] = []
        var indices: [UInt32] = []
        for face in faces {
            var n = simd_normalize(simd_cross(face[1] - face[0], face[2] - face[0]))
            let outward = simd_dot(n, face[0] - centre) > 0
            if !outward { n = -n }
            let base = UInt32(positions.count)
            for c in face {
                positions.append(c)
                normals.append(n)
                uvs.append([c[along], c[v]] / Self.grainRepeat)
            }
            for i in 1..<(face.count - 1) {
                let (j, k) = (UInt32(i), UInt32(i + 1))
                indices += outward ? [base, base + j, base + k] : [base, base + k, base + j]
            }
        }
        var desc = MeshDescriptor()
        desc.positions = MeshBuffers.Positions(positions)
        desc.normals = MeshBuffers.Normals(normals)
        desc.textureCoordinates = MeshBuffers.TextureCoordinates(uvs)
        desc.primitives = .triangles(indices)
        return (try? MeshResource.generate(from: [desc])) ?? .generateBox(size: size)
    }

    /// A door with an arched top: its outline across the front, thick as the door.
    private static func archedMesh(_ p: PanelBox, size: SIMD3<Float>, by offset: SIMD3<Float>) -> MeshResource {
        let h = size / 2
        let steps = 32
        // Anticlockwise from the viewer: along the bottom, then back across the arc.
        var outline: [SIMD2<Float>] = [[-h.x, -h.y], [h.x, -h.y]]
        for i in 0...steps {
            let x = h.x - size.x * Float(i) / Float(steps)
            let mm = p.x + p.width / 2 + Double(x) * 1000
            let run = mm - p.arch[0]
            let top = p.arch[1] + (p.arch[2] * p.arch[2] - run * run).squareRoot()
            outline.append([x, min(Float((top - (p.z + p.height / 2)) / 1000), h.y)])
        }
        let g = grainAxis(p)
        var positions: [SIMD3<Float>] = [], normals: [SIMD3<Float>] = [], uvs: [SIMD2<Float>] = []
        var indices: [UInt32] = []
        func add(_ face: [SIMD3<Float>], normal n: SIMD3<Float>) {
            let base = UInt32(positions.count)
            let axis = abs(n.x) > max(abs(n.y), abs(n.z)) ? 0 : abs(n.y) > abs(n.z) ? 1 : 2
            var plane = [0, 1, 2].filter { $0 != axis }
            if let g, plane.contains(g), plane[0] != g { plane.reverse() }
            for c in face {
                positions.append(c + offset)
                normals.append(n)
                let at = c + h
                uvs.append([at[plane[0]], at[plane[1]]] / grainRepeat)
            }
            let turning = simd_dot(simd_cross(face[1] - face[0], face[2] - face[0]), n) > 0
            for i in 1..<(face.count - 1) {
                let (j, k) = (UInt32(i), UInt32(i + 1))
                indices += turning ? [base, base + j, base + k] : [base, base + k, base + j]
            }
        }
        add(outline.map { [$0.x, $0.y, h.z] }, normal: [0, 0, 1])
        add(outline.reversed().map { [$0.x, $0.y, -h.z] }, normal: [0, 0, -1])
        for i in outline.indices {
            let (a, b) = (outline[i], outline[(i + 1) % outline.count])
            let edge = b - a
            guard simd_length(edge) > 1e-6 else { continue }
            let out = simd_normalize(SIMD2<Float>(edge.y, -edge.x))
            add([[a.x, a.y, h.z], [a.x, a.y, -h.z], [b.x, b.y, -h.z], [b.x, b.y, h.z]], normal: [out.x, out.y, 0])
        }
        var desc = MeshDescriptor()
        desc.positions = MeshBuffers.Positions(positions)
        desc.normals = MeshBuffers.Normals(normals)
        desc.textureCoordinates = MeshBuffers.TextureCoordinates(uvs)
        desc.primitives = .triangles(indices)
        return (try? MeshResource.generate(from: [desc])) ?? .generateBox(size: size)
    }

    /// A box mesh whose texture coordinates are in metres along each face, the first one along the
    /// grain where the face has it, so a woodgrain texture runs the right way at its true scale.
    private static func offsetMesh(size: SIMD3<Float>, by offset: SIMD3<Float>, grain: Int?) -> MeshResource {
        var desc = MeshDescriptor()
        let h = size / 2
        let corners: [SIMD3<Float>] = (0..<8).map { i in
            SIMD3(i & 1 == 0 ? -h.x : h.x, i & 2 == 0 ? -h.y : h.y, i & 4 == 0 ? -h.z : h.z) + offset
        }
        // Each face as its own four vertices so it shades flat.
        let faces: [([Int], SIMD3<Float>)] = [
            ([0, 2, 6, 4], [-1, 0, 0]), ([1, 5, 7, 3], [1, 0, 0]),
            ([0, 4, 5, 1], [0, -1, 0]), ([2, 3, 7, 6], [0, 1, 0]),
            ([0, 1, 3, 2], [0, 0, -1]), ([4, 6, 7, 5], [0, 0, 1]),
        ]
        var positions: [SIMD3<Float>] = [], normals: [SIMD3<Float>] = [], uvs: [SIMD2<Float>] = []
        var indices: [UInt32] = []
        for (quad, n) in faces {
            let base = UInt32(positions.count)
            let normal = n.x != 0 ? 0 : n.y != 0 ? 1 : 2
            var plane = [0, 1, 2].filter { $0 != normal }
            if let g = grain, plane.contains(g), plane[0] != g { plane.reverse() }
            for i in quad {
                positions.append(corners[i])
                normals.append(n)
                let p = corners[i] - offset + size / 2
                uvs.append([p[plane[0]], p[plane[1]]] / Self.grainRepeat)
            }
            // Counter-clockwise seen from outside, so the faces point out.
            indices += [base, base + 2, base + 1, base, base + 3, base + 2]
        }
        desc.positions = MeshBuffers.Positions(positions)
        desc.normals = MeshBuffers.Normals(normals)
        desc.textureCoordinates = MeshBuffers.TextureCoordinates(uvs)
        desc.primitives = .triangles(indices)
        return (try? MeshResource.generate(from: [desc])) ?? .generateBox(size: size)
    }

    /// Cabinet frame (mm, Z up, Y to the back) → RealityKit (m, Y up, Z to the viewer).
    private static func point(_ x: Double, _ y: Double, _ z: Double) -> SIMD3<Float> {
        SIMD3<Float>(Float(x), Float(z), -Float(y)) / 1000
    }

    private static func mid(_ p: PanelBox) -> SIMD3<Float> {
        point(p.x + p.width / 2, p.y + p.depth / 2, p.z + p.height / 2)
    }

    /// A door (and each piece of a shaker door) turns about the door's hinge line, a sloped board
    /// about the line it's tilted on; everything else about its middle.
    private static func pivot(_ p: PanelBox) -> SIMD3<Float> {
        if p.tilt.count == 3 { return point(p.x + p.width / 2, p.tilt[1], p.tilt[2]) }
        return p.hinged.isEmpty ? mid(p) : point(p.hingeX, p.hingeY, p.z + p.height / 2)
    }

    private static func centre(of panels: [PanelBox]) -> SIMD3<Float> {
        guard !panels.isEmpty else { return .zero }
        return panels.map(mid).reduce(.zero, +) / Float(panels.count)
    }

    /// `box` less the rebate `r` (min x, y, z, max x, y, z), which runs right through it along
    /// one axis and reaches a face on each of the other two: the part beside the rebate on one of
    /// those axes, and the part beside it on the other, within the rebate's span on the first.
    static func split(_ box: PanelBox, by r: [Double]) -> [PanelBox] {
        guard r.count == 6 else { return [box] }
        let lo = [box.x, box.y, box.z], hi = [box.x + box.width, box.y + box.depth, box.z + box.height]
        let rlo = (0..<3).map { max(r[$0], lo[$0]) }, rhi = (0..<3).map { min(r[$0 + 3], hi[$0]) }
        let eps = 1e-6
        let spans = (0..<3).filter { rlo[$0] <= lo[$0] + eps && rhi[$0] >= hi[$0] - eps }
        let cut = (0..<3).filter { !spans.contains($0) }
        guard spans.count == 1, cut.count == 2 else { return [box] }
        // The range of axis `a` beside the rebate.
        func beside(_ a: Int) -> (Double, Double) {
            abs(rlo[a] - lo[a]) < eps ? (rhi[a], hi[a]) : (lo[a], rlo[a])
        }
        func piece(_ name: String, _ ranges: [Int: (Double, Double)]) -> PanelBox {
            var l = lo, h = hi
            for (a, (x0, x1)) in ranges { l[a] = x0; h[a] = x1 }
            var b = box
            b.name = name
            b.x = l[0]; b.y = l[1]; b.z = l[2]
            b.width = h[0] - l[0]; b.depth = h[1] - l[1]; b.height = h[2] - l[2]
            return b
        }
        let (a, c) = (cut[0], cut[1])
        let first = piece(box.name, [a: beside(a)])
        let second = piece(box.name + "~rebate", [a: (rlo[a], rhi[a]), c: beside(c)])
        var pieces = [first, second]
        // A bevel keeps the slope inside the rebate's box: a wedge of its own.
        if box.bevel.count == 3 {
            pieces.append(piece(box.name + "~bevel", [a: (rlo[a], rhi[a]), c: (rlo[c], rhi[c])]))
        }
        return pieces.filter { $0.width > eps && $0.depth > eps && $0.height > eps }
    }

    /// `box` around rectangular openings right through it (min x, min z, max x, max z each): the
    /// strips across it, bottom to top, each broken where an opening passes.
    static func around(_ box: PanelBox, openings o: [Double]) -> [PanelBox] {
        struct Hole { let x0, z0, x1, z1: Double }
        var holes: [Hole] = []
        for i in stride(from: 0, to: o.count - 3, by: 4) {
            holes.append(Hole(x0: o[i], z0: o[i + 1], x1: o[i + 2], z1: o[i + 3]))
        }
        let (left, right): (Double, Double) = (box.x, box.x + box.width)
        var levels: Set<Double> = [box.z, box.z + box.height]
        for h in holes {
            levels.insert(h.z0)
            levels.insert(h.z1)
        }
        let zs: [Double] = levels.sorted()
        var pieces: [PanelBox] = []
        for i in 0..<max(zs.count - 1, 0) {
            let (z0, z1): (Double, Double) = (zs[i], zs[i + 1])
            let crossing: [Hole] = holes
                .filter { (h: Hole) -> Bool in h.z0 < z1 && h.z1 > z0 }
                .sorted { (a: Hole, b: Hole) -> Bool in a.x0 < b.x0 }
            var x: Double = left
            var edges: [(Double, Double)] = crossing.map { (h: Hole) -> (Double, Double) in (h.x0, h.x1) }
            edges.append((right, right))
            for edge in edges {
                let end: Double = min(edge.0, right)
                if end > x + 1e-6 {
                    var b = box
                    b.name = pieces.isEmpty ? box.name : "\(box.name)~\(pieces.count)"
                    b.x = x
                    b.width = end - x
                    b.z = z0
                    b.height = z1 - z0
                    pieces.append(b)
                }
                x = max(x, min(edge.1, right))
            }
        }
        return pieces
    }

    /// How far an open drawer slides out, m.
    private static let drawerTravel: Float = 0.35

    private static func transform(_ p: PanelBox, centre: SIMD3<Float>, look: ViewportLook) -> Transform {
        var t = Transform(translation: pivot(p))
        if p.tilt.count == 3 { t.rotation = simd_quatf(angle: Float(p.tilt[0]) * .pi / 180, axis: [1, 0, 0]) }
        // Exploded: away from the cabinet's centre, as far as 250 mm.
        let away = mid(p) - centre
        let len = simd_length(away)
        if len > 0.0001 { t.translation += away / len * Float(look.explode) * 0.25 }
        if look.open {
            switch (p.role, p.hinged) {
            case (_, "left"): t.rotation = simd_quatf(angle: -.pi * 0.55, axis: [0, 1, 0])
            case (_, "right"): t.rotation = simd_quatf(angle: .pi * 0.55, axis: [0, 1, 0])
            case _ where p.slides:
                t.translation.z += Self.drawerTravel
            case ("Lift front", _): t.translation.y += 0.3
            default: break
            }
        }
        return t
    }

    /// One woodgrain texture tile covers this many metres.
    private static let grainRepeat: Float = 0.6
    private var grainTextures: [String: TextureResource] = [:]

    /// The board's preview colour (generic colours without a board), with a woodgrain texture for
    /// grained decors. Cabinets other than the selected one are greyed a little.
    private func material(_ p: PanelBox, board: BoardRecord?, selected: Bool, hovered: Bool) -> PhysicallyBasedMaterial {
        let generic: NSColor = p.isFront ? Theme.front : (p.role == "Back" ? Theme.carcassBack : Theme.carcass)
        if p.board.hasPrefix("handle:") {
            return Self.handleMaterial(p.board.dropFirst("handle:".count), selected: selected)
        }
        if p.board.hasPrefix("tile:") {
            return tileMaterial(p, selected: selected, hovered: hovered)
        }
        let colour = board?.displayColour ?? generic
        var m = PhysicallyBasedMaterial()
        let tint = selected ? NSColor.white : NSColor(calibratedWhite: 0.72, alpha: 1)
        let texture: TextureResource? = if let board, board.profileModule > 0 {
            profileTexture(board, colour)
        } else if let board, board.grained {
            grainTexture(board.id, colour)
        } else {
            nil
        }
        if let texture {
            let sampler = MTLSamplerDescriptor()
            sampler.sAddressMode = .repeat
            sampler.tAddressMode = .repeat
            sampler.minFilter = .linear
            sampler.magFilter = .linear
            sampler.mipFilter = .linear
            m.baseColor = .init(tint: tint, texture: .init(texture, sampler: .init(sampler)))
        } else {
            m.baseColor = .init(tint: selected ? colour : colour.blended(withFraction: 0.35, of: .gray) ?? colour)
        }
        m.roughness = board?.finish.lowercased().contains("gloss") == true ? 0.2 : 0.65
        // The selected cabinet glows faintly in the accent colour; the panel under the pointer more.
        if selected || hovered {
            m.emissiveColor = .init(color: NSColor(hex: 0xE8845E))
            m.emissiveIntensity = hovered ? 0.6 : 0.12
        } else {
            // Said outright: a new material that leaves this unset keeps the entity's old glow.
            m.emissiveColor = .init(color: .black)
            m.emissiveIntensity = 0
        }
        return m
    }

    /// A tiled field: its whole layout as one picture, to scale, so the cuts and joints fall where
    /// the core puts them. The box's texture coordinates are in `grainRepeat`s from its corner.
    private func tileMaterial(_ p: PanelBox, selected: Bool, hovered: Bool) -> PhysicallyBasedMaterial {
        var m = PhysicallyBasedMaterial()
        let parts = p.board.split(separator: ":").map(String.init)
        let colour = parts.count > 1 ? NSColor(hex: parts[1]) : nil
        if let texture = tileTexture(p) {
            let sampler = MTLSamplerDescriptor()
            sampler.minFilter = .linear
            sampler.magFilter = .linear
            sampler.mipFilter = .linear
            m.baseColor = .init(tint: .white, texture: .init(texture, sampler: .init(sampler)))
            m.textureCoordinateTransform = .init(scale: [
                Self.grainRepeat / Float(p.width / 1000),
                Self.grainRepeat / Float(p.height / 1000),
            ])
        } else {
            m.baseColor = .init(tint: colour ?? Theme.carcass)
        }
        m.roughness = 0.35
        if selected || hovered {
            m.emissiveColor = .init(color: NSColor(hex: 0xE8845E))
            m.emissiveIntensity = hovered ? 0.6 : 0.06
        } else {
            m.emissiveColor = .init(color: .black)
            m.emissiveIntensity = 0
        }
        return m
    }

    private func tileTexture(_ p: PanelBox) -> TextureResource? {
        let key = "\(p.board)|\(p.width)|\(p.height)"
        if let t = grainTextures[key] { return t }
        guard !loading.contains(key), let image = Self.tilePicture(p) else { return nil }
        loading.insert(key)
        Task {
            let t = try? await TextureResource(image: image, options: .init(semantic: .color, mipmapsMode: .allocateAndGenerateAll))
            grainTextures[key] = t
            loading.remove(key)
            onTextureLoaded?()
        }
        return nil
    }

    /// The field drawn at up to 2 pixels a millimetre, within what a texture can hold.
    static func tilePicture(_ p: PanelBox) -> CGImage? {
        let f = p.board.split(separator: ":").map(String.init)
        guard f.count == 7, p.width > 0, p.height > 0,
              let tile = NSColor(hex: f[1])?.usingColorSpace(.sRGB),
              let grout = NSColor(hex: f[2])?.usingColorSpace(.sRGB)
        else { return nil }
        let scale = min(2, 4096 / max(p.width, p.height))
        let w = Int(p.width * scale), h = Int(p.height * scale)
        guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
        else { return nil }
        ctx.setFillColor(grout.cgColor)
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        ctx.setFillColor(tile.cgColor)
        for t in tileRects(board: p.board, width: p.width, height: p.height) {
            ctx.fill(CGRect(x: t.x * scale, y: t.z * scale, width: t.width * scale, height: t.height * scale))
        }
        return ctx.makeImage()
    }

    /// A handle's metal, by its finish's name, or by a measured `#RRGGBB` from the catalog.
    private static func handleMaterial(_ finish: Substring, selected: Bool) -> PhysicallyBasedMaterial {
        // "#RRGGBB", and " matte" for a finish that is not bare metal.
        let measured = finish.hasPrefix("#") ? nsColor(hex: finish.prefix(7)) : nil
        let matte = finish.hasSuffix(" matte")
        let named: NSColor = switch finish {
        case "matt black": NSColor(calibratedWhite: 0.08, alpha: 1)
        case "brushed brass": NSColor(calibratedRed: 0.76, green: 0.6, blue: 0.33, alpha: 1)
        case "brushed nickel": NSColor(calibratedRed: 0.72, green: 0.71, blue: 0.67, alpha: 1)
        default: NSColor(calibratedWhite: 0.8, alpha: 1)
        }
        let colour = measured ?? named
        var m = PhysicallyBasedMaterial()
        m.baseColor = .init(tint: selected ? colour : colour.blended(withFraction: 0.3, of: .gray) ?? colour)
        m.metallic = .init(floatLiteral: finish == "matt black" || matte ? 0.2 : 1)
        m.roughness = .init(floatLiteral: finish.hasPrefix("brushed") || measured != nil ? 0.45 : 0.3)
        return m
    }

    /// `#RRGGBB` as a calibrated colour. Anything else is no colour.
    private static func nsColor(hex: Substring) -> NSColor? {
        let digits = hex.dropFirst()
        guard digits.count == 6, let v = UInt32(digits, radix: 16) else { return nil }
        return NSColor(calibratedRed: CGFloat((v >> 16) & 0xff) / 255,
                        green: CGFloat((v >> 8) & 0xff) / 255,
                        blue: CGFloat(v & 0xff) / 255, alpha: 1)
    }

    private var loading: Set<String> = []
    /// Called when a texture has loaded, so the viewport updates with it.
    var onTextureLoaded: (() -> Void)?

    /// A lining's routed profile as stripes along the grain, or nil while it loads.
    private func profileTexture(_ board: BoardRecord, _ colour: NSColor) -> TextureResource? {
        let key = "profile:" + board.id
        if let t = grainTextures[key] { return t }
        guard !loading.contains(key),
              let image = Self.flutes(colour, shape: board.profileShape, module: board.profileModule,
                                      groove: board.profileGroove)
        else { return nil }
        loading.insert(key)
        Task {
            let t = try? await TextureResource(image: image, options: .init(semantic: .color, mipmapsMode: .allocateAndGenerateAll))
            grainTextures[key] = t
            loading.remove(key)
            onTextureLoaded?()
        }
        return nil
    }

    /// A profile tile: one texture tile spans `grainRepeat` metres across the board, so the
    /// module repeats a whole number of times in it (rounded, to tile without a seam). Grooves are
    /// dark bands along the grain (the image's x axis); scallops and rounds shade across each module.
    static func flutes(_ colour: NSColor, shape: String, module: Double, groove: Double) -> CGImage? {
        let w = 64, h = 512
        guard module > 0, let rgb = colour.usingColorSpace(.sRGB),
              let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
        else { return nil }
        ctx.setFillColor(rgb.cgColor)
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        let count = max(1, (Double(grainRepeat) * 1000 / module).rounded())
        let pitch = CGFloat(h) / CGFloat(count)
        let band = pitch * CGFloat(min(groove / module, 0.6))
        for i in 0..<Int(count) {
            let y0 = CGFloat(i) * pitch
            switch shape {
            case "scallop", "round":
                // Light at one edge of each module, dark at the other: a curved face in light.
                let steps = 16
                for s in 0..<steps {
                    let f = CGFloat(s) / CGFloat(steps)
                    let shade = shape == "round" ? 0.5 - 0.5 * cos(f * 2 * .pi) : f
                    ctx.setFillColor(CGColor(gray: 0, alpha: 0.18 * shade))
                    ctx.fill(CGRect(x: 0, y: y0 + f * pitch, width: CGFloat(w), height: pitch / CGFloat(steps) + 0.5))
                }
            default:
                ctx.setFillColor(CGColor(gray: 0, alpha: 0.35))
                ctx.fill(CGRect(x: 0, y: y0, width: CGFloat(w), height: max(band, 1)))
                // A light edge where the face meets the groove.
                ctx.setFillColor(CGColor(gray: 1, alpha: 0.25))
                ctx.fill(CGRect(x: 0, y: y0 + max(band, 1), width: CGFloat(w), height: 1))
            }
        }
        return ctx.makeImage()
    }

    /// The decor's woodgrain texture, or nil while it loads.
    private func grainTexture(_ key: String, _ colour: NSColor) -> TextureResource? {
        if let t = grainTextures[key] { return t }
        guard !loading.contains(key), let image = Self.woodgrain(colour) else { return nil }
        loading.insert(key)
        Task {
            let t = try? await TextureResource(image: image, options: .init(semantic: .color, mipmapsMode: .allocateAndGenerateAll))
            grainTextures[key] = t
            loading.remove(key)
            onTextureLoaded?()
        }
        return nil
    }

    /// A woodgrain tile in the board's colour: long, slightly wavy streaks of darker and lighter
    /// wood along the image's x axis, the grain direction. Seeded, so a decor always looks the same.
    static func woodgrain(_ colour: NSColor) -> CGImage? {
        let w = 512, h = 512
        guard let rgb = colour.usingColorSpace(.sRGB),
              let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
        else { return nil }
        ctx.setFillColor(rgb.cgColor)
        ctx.fill(CGRect(x: 0, y: 0, width: w, height: h))
        var seed: UInt64 = 0x9E37_79B9_7F4A_7C15
        func random() -> CGFloat {
            seed = seed &* 6_364_136_223_846_793_005 &+ 1_442_695_040_888_963_407
            return CGFloat(seed >> 33) / CGFloat(1 << 31)
        }
        // Broad soft bands first (growth rings seen flat), then fine streaks. Each is drawn again
        // one tile up and down, so the tile repeats without a seam.
        func stroke(y: CGFloat, width: CGFloat, colour: CGColor, amplitude: CGFloat, periods: CGFloat, phase: CGFloat) {
            ctx.setStrokeColor(colour)
            ctx.setLineWidth(width)
            for shift in [-CGFloat(h), 0, CGFloat(h)] {
                let path = CGMutablePath()
                for x in stride(from: 0, through: w, by: 4) {
                    let p = CGPoint(x: CGFloat(x), y: y + shift + amplitude * sin(CGFloat(x) / CGFloat(w) * 2 * .pi * periods + phase))
                    x == 0 ? path.move(to: p) : path.addLine(to: p)
                }
                ctx.addPath(path)
                ctx.strokePath()
            }
        }
        for _ in 0..<14 {
            let dark = random() < 0.5
            stroke(y: random() * CGFloat(h), width: 10 + random() * 36,
                   colour: dark ? CGColor(gray: 0, alpha: 0.03 + random() * 0.05) : CGColor(gray: 1, alpha: 0.03 + random() * 0.05),
                   amplitude: 2 + random() * 6, periods: CGFloat(Int(1 + random() * 2)), phase: random() * 2 * .pi)
        }
        for _ in 0..<110 {
            let dark = random() < 0.7
            stroke(y: random() * CGFloat(h), width: 0.5 + random() * 1.6,
                   colour: dark ? CGColor(gray: 0, alpha: 0.03 + random() * 0.08) : CGColor(gray: 1, alpha: 0.03 + random() * 0.06),
                   amplitude: 0.5 + random() * 2.5, periods: CGFloat(Int(1 + random() * 3)), phase: random() * 2 * .pi)
        }
        return ctx.makeImage()
    }

    /// The cabinet an entity in the viewport belongs to: its part names start with the id.
    func cabinet(of entity: Entity, in project: ProjectModel) -> String? {
        project.cabinets.first { entity.name.hasPrefix($0.id + ".") }?.id
    }

    // MARK: Camera

    func applyCamera() {
        let from = target + distance * SIMD3<Float>(sin(yaw) * cos(pitch), sin(pitch), cos(yaw) * cos(pitch))
        camera.look(at: target, from: from, relativeTo: nil)
    }

    func orbit(dx: CGFloat, dy: CGFloat) {
        cameraMoved = true
        yaw -= Float(dx) * 0.008
        pitch = min(Self.maxPitch, max(-0.2, pitch + Float(dy) * 0.008))
        applyCamera()
    }

    /// Moves what the camera looks at so the view follows the pointer.
    func pan(dx: CGFloat, dy: CGFloat) {
        cameraMoved = true
        let fov = camera.camera.fieldOfViewInDegrees * .pi / 180
        let perPoint = 2 * distance * tan(fov / 2) / Float(max(viewSize.height, 1))
        let m = camera.transformMatrix(relativeTo: nil)
        let right = simd_make_float3(m.columns.0)
        let up = simd_make_float3(m.columns.1)
        target += (-Float(dx) * right + Float(dy) * up) * perPoint
        applyCamera()
    }

    func zoom(by factor: Float) {
        cameraMoved = true
        distance = min(30, max(0.15, distance * factor))
        applyCamera()
    }

    /// Back to the whole job, and following it again as it grows.
    func fit() {
        cameraMoved = false
        frameJob()
    }

    /// Frames the whole run, keeping the angle.
    private func frameJob() {
        let fov = camera.camera.fieldOfViewInDegrees * .pi / 180
        let aspect = Float(max(viewSize.width, 1) / max(viewSize.height, 1))
        let span = max(extent.x / max(aspect, 0.1), extent.y)
        distance = max(0.5, span / (2 * tan(fov / 2)) * 1.5 + extent.z / 2)
        target = [0, extent.y / 2, -extent.z / 2]
        applyCamera()
    }

    func show(_ preset: ViewPreset) {
        (yaw, pitch) = preset.angles
        fit()
    }

    /// The part under a point of the view, by a ray from the camera through it.
    func part(at point: CGPoint, size: CGSize) -> String? {
        guard size.width > 0, size.height > 0, let scene = root.scene else { return nil }
        let fov = camera.camera.fieldOfViewInDegrees * .pi / 180
        let t = tan(fov / 2)
        let aspect = Float(size.width / size.height)
        let local = SIMD3<Float>(
            (2 * Float(point.x / size.width) - 1) * t * aspect,
            (1 - 2 * Float(point.y / size.height)) * t,
            -1
        )
        let m = camera.transformMatrix(relativeTo: nil)
        let origin = SIMD3<Float>(m.columns.3.x, m.columns.3.y, m.columns.3.z)
        let dir = simd_normalize(simd_make_float3(m * SIMD4<Float>(local, 0)))
        let hit = scene.raycast(origin: origin, direction: dir, length: 50, query: .nearest).first
        return hit.map(\.entity.name).flatMap { panels[$0] != nil ? $0 : nil }
    }
}
