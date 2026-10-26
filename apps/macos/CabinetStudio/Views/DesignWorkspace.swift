import RealityKit
import SwiftUI

struct DesignWorkspace: View {
    @Environment(ProjectModel.self) private var project
    @State private var look = ViewportLook()
    @State private var hovered: HoveredPart?

    var body: some View {
        ZStack(alignment: .topLeading) {
            CabinetViewport(look: look, hovered: $hovered)
            VStack(alignment: .leading, spacing: 8) {
                ViewportControls(look: $look)
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
        .background(
            LinearGradient(colors: [Color(white: 0.20), Color(white: 0.11)], startPoint: .top, endPoint: .bottom)
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

struct ViewportControls: View {
    @Binding var look: ViewportLook

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 6) {
                Toggle(isOn: $look.showFronts.animation(.snappy)) {
                    Label("Fronts", systemImage: "door.left.hand.closed")
                }
                .help("Show doors and drawer fronts")
                Toggle(isOn: $look.open) {
                    Label("Open", systemImage: "door.left.hand.open")
                }
                .disabled(!look.showFronts)
                .help("Swing the doors open and pull the drawers out")
                Toggle(isOn: $look.section) {
                    Label("Section", systemImage: "square.split.diagonal")
                }
                .help("Cut the run through the middle of the selected cabinet")
                Toggle(isOn: $look.showHardware.animation(.snappy)) {
                    Label("Hardware", systemImage: "gearshape.2")
                }
                .help("Show hinges, runners and legs (Blum's own models where the supplier CAD is prepared)")
            }
            .toggleStyle(.button)
            .labelStyle(.iconOnly)
            HStack(spacing: 6) {
                Image(systemName: "arrow.up.left.and.arrow.down.right")
                    .foregroundStyle(.secondary)
                    .help("Explode: pull the panels apart")
                Slider(value: $look.explode, in: 0...1)
                    .frame(width: 110)
                    .accessibilityLabel("Explode")
            }
        }
        .controlSize(.small)
        .padding(8)
        .glassBackground()
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
            $0.placed != true && !["end_panel", "benchtop", "cornice", "light_rail"].contains($0.kind)
        }
    }

    private func width(_ c: CabinetDoc) -> Double {
        resizing?.id == c.id ? resizing!.width : c.width
    }

    var body: some View {
        GeometryReader { geo in
            let total = max(items.reduce(0) { $0 + width($1) }, 1)
            let scale = (geo.size.width - CGFloat(max(items.count - 1, 0)) * 2) / total
            HStack(spacing: 2) {
                ForEach(items) { c in
                    block(c, width: CGFloat(width(c)) * scale, scale: scale)
                }
            }
        }
        .frame(height: 34)
        .padding(6)
        .glassBackground()
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Run")
    }

    private func block(_ c: CabinetDoc, width: CGFloat, scale: CGFloat) -> some View {
        let selected = c.id == project.selection
        return ZStack(alignment: .trailing) {
            RoundedRectangle(cornerRadius: 5)
                .fill(selected ? Color.accentColor.opacity(0.45) : Color.primary.opacity(c.isPiece ? 0.06 : 0.12))
                .overlay(RoundedRectangle(cornerRadius: 5).strokeBorder(selected ? Color.accentColor : .primary.opacity(0.25)))
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
                    reorder(c, by: Double(v.translation.width / scale))
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
    private func reorder(_ c: CabinetDoc, by dx: Double) {
        let run = items
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
        .glassBackground()
        .fixedSize()
    }
}

/// Counts for the selected cabinet, floating over the viewport.
struct BuildBadge: View {
    let summary: BuildSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Label("\(summary.panels.count) panels", systemImage: "square.3.layers.3d")
            Label("\(summary.biscuits) biscuits · \(summary.screws) screws", systemImage: "link")
            Label("\(summary.operations) machining ops", systemImage: "circle.grid.cross")
            if !summary.warnings.isEmpty {
                Label(summary.warnings.count == 1 ? "1 note" : "\(summary.warnings.count) notes",
                      systemImage: "exclamationmark.bubble")
                    .foregroundStyle(.orange)
                    .help(summary.warnings.joined(separator: "\n"))
            }
        }
        .font(.callout)
        .padding(10)
        .glassBackground()
    }
}

extension View {
    /// Liquid Glass where the system has it, a material elsewhere.
    @ViewBuilder func glassBackground() -> some View {
        if #available(macOS 26.0, *) {
            self.glassEffect(.regular, in: .rect(cornerRadius: 12))
        } else {
            self.background(.regularMaterial, in: .rect(cornerRadius: 12))
        }
    }
}

struct CabinetViewport: View {
    @Environment(ProjectModel.self) private var project
    let look: ViewportLook
    @Binding var hovered: HoveredPart?
    @State private var scene = ViewportScene()
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
                scene.camera.look(at: [0, 0.55, -0.3], from: [-2.3, 2.1, 4.3], relativeTo: nil)
                content.add(scene.camera)

                let floor = ModelEntity(
                    mesh: .generatePlane(width: 12, depth: 12),
                    materials: [SimpleMaterial(color: NSColor(white: 0.32, alpha: 1), roughness: 0.9, isMetallic: false)]
                )
                floor.position.y = -0.0005
                content.add(floor)
            } update: { _ in
                _ = textures
                scene.update(project: project, look: look, hovered: hovered?.panel.name)
            }
            .realityViewCameraControls(.orbit)
            .accessibilityLabel("3D view of the job")
            .accessibilityHint("Drag to orbit. Click a cabinet to select it; the arrow keys step through cabinets.")
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
                    if let name = scene.part(at: point, size: geo.size),
                       let panel = scene.panels[name] {
                        hovered = HoveredPart(panel: panel, at: point)
                    } else {
                        hovered = nil
                    }
                case .ended:
                    hovered = nil
                }
            }
        }
    }
}

/// The viewport's entities, kept across updates so a change moves panels instead of rebuilding
/// them (doors swing, explode slides). One entity per panel, named after the part.
@MainActor
final class ViewportScene {
    let root = Entity()
    let camera = PerspectiveCamera()
    /// The panels shown, by part name, as last built.
    private(set) var panels: [String: PanelBox] = [:]
    private var sizes: [String: SIMD3<Float>] = [:]
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
                // A side notched for the toe kick: its upper part, and the foot behind the notch.
                var pieces = [box]
                if p.notchHeight > 0 {
                    var foot = box
                    foot.name = p.name + "~foot"
                    foot.y += p.notchDepth
                    foot.depth -= p.notchDepth
                    foot.height = p.notchHeight
                    pieces[0].z += p.notchHeight
                    pieces[0].height -= p.notchHeight
                    pieces.append(foot)
                }
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
        for e in root.children where !seen.contains(e.name) {
            e.removeFromParent()
            panels[e.name] = nil
            sizes[e.name] = nil
        }
        // Keep the run centred on the origin the camera orbits.
        if minX < maxX { root.position.x = -(minX + maxX) / 2 }
    }

    // MARK: Hardware

    /// Loaded supplier meshes by file; nil for a file that would not load, so it is not retried
    /// on every redraw.
    private var meshes: [String: MeshResource?] = [:]

    /// Supplier models and stand-in shapes. They sit on the carcass, so they are left out of an
    /// exploded view; hinges show only with the fronts closed, drawer box sides slide out with
    /// their drawers, and nothing shows on the cut-away side of a section.
    private func addHardware(project: ProjectModel, look: ViewportLook, sectionX: Double?, seen: inout Set<String>) {
        guard look.showHardware, look.explode < 0.001 else { return }
        let (models, shapes) = project.hardware()
        func visible(_ name: String, x: Double) -> Bool {
            if let cut = sectionX, x < cut { return false }
            if name.contains(".Hinge") { return look.showFronts && !look.open }
            return true
        }
        func slid(_ t: Transform, _ name: String) -> Transform {
            guard look.open, name.contains(".BoxSide") else { return t }
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

    /// The panel's entity, made or remade when its size changes. Its mesh is centred on its hinge
    /// edge for doors (so it swings about it) and on its middle otherwise.
    private func entity(for p: PanelBox) -> ModelEntity {
        let size = SIMD3<Float>(Float(p.width), Float(p.height), Float(p.depth)) / 1000
        if let e = root.findEntity(named: p.name) as? ModelEntity, sizes[p.name] == size {
            return e
        }
        root.findEntity(named: p.name)?.removeFromParent()
        // The geometry sits off the entity's origin so the origin is the pivot.
        let offset = Self.mid(p) - Self.pivot(p)
        let e = ModelEntity(mesh: Self.offsetMesh(size: size, by: offset, grain: Self.grainAxis(p)), materials: [])
        e.name = p.name
        e.components.set(InputTargetComponent())
        e.collision = CollisionComponent(shapes: [ShapeResource.generateBox(size: size).offsetBy(translation: offset)])
        root.addChild(e)
        sizes[p.name] = size
        return e
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

    /// A door (and each piece of a shaker door) turns about the door's hinge line; everything else
    /// about its middle.
    private static func pivot(_ p: PanelBox) -> SIMD3<Float> {
        p.hinged.isEmpty ? mid(p) : point(p.hingeX, p.hingeY, p.z + p.height / 2)
    }

    private static func centre(of panels: [PanelBox]) -> SIMD3<Float> {
        guard !panels.isEmpty else { return .zero }
        return panels.map(mid).reduce(.zero, +) / Float(panels.count)
    }

    /// How far an open drawer slides out, m.
    private static let drawerTravel: Float = 0.35

    private static func transform(_ p: PanelBox, centre: SIMD3<Float>, look: ViewportLook) -> Transform {
        var t = Transform(translation: pivot(p))
        // Exploded: away from the cabinet's centre, as far as 250 mm.
        let away = mid(p) - centre
        let len = simd_length(away)
        if len > 0.0001 { t.translation += away / len * Float(look.explode) * 0.25 }
        if look.open {
            switch (p.role, p.hinged) {
            case (_, "left"): t.rotation = simd_quatf(angle: -.pi * 0.55, axis: [0, 1, 0])
            case (_, "right"): t.rotation = simd_quatf(angle: .pi * 0.55, axis: [0, 1, 0])
            case ("Drawer front", _), ("Drawer bottom", _), ("Drawer back", _), ("Drawer side", _), ("Drawer handle", _):
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
        let generic: NSColor = p.isFront
            ? NSColor(calibratedRed: 0.78, green: 0.65, blue: 0.48, alpha: 1)
            : NSColor(calibratedWhite: p.role == "Back" ? 0.78 : 0.93, alpha: 1)
        if p.board.hasPrefix("handle:") {
            return Self.handleMaterial(p.board.dropFirst("handle:".count), selected: selected)
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
            m.emissiveColor = .init(color: .controlAccentColor)
            m.emissiveIntensity = hovered ? 0.6 : 0.12
        }
        return m
    }

    /// A handle's metal, by its finish's name.
    private static func handleMaterial(_ finish: Substring, selected: Bool) -> PhysicallyBasedMaterial {
        let colour: NSColor = switch finish {
        case "matt black": NSColor(calibratedWhite: 0.08, alpha: 1)
        case "brushed brass": NSColor(calibratedRed: 0.76, green: 0.6, blue: 0.33, alpha: 1)
        case "brushed nickel": NSColor(calibratedRed: 0.72, green: 0.71, blue: 0.67, alpha: 1)
        default: NSColor(calibratedWhite: 0.8, alpha: 1)
        }
        var m = PhysicallyBasedMaterial()
        m.baseColor = .init(tint: selected ? colour : colour.blended(withFraction: 0.3, of: .gray) ?? colour)
        m.metallic = .init(floatLiteral: finish == "matt black" ? 0.2 : 1)
        m.roughness = .init(floatLiteral: finish.hasPrefix("brushed") ? 0.45 : 0.3)
        return m
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
