import RealityKit
import SwiftUI

struct DesignWorkspace: View {
    @Environment(ProjectModel.self) private var project
    @State private var showFronts = true

    var body: some View {
        ZStack(alignment: .topLeading) {
            CabinetViewport(showFronts: showFronts)
            VStack(alignment: .leading, spacing: 8) {
                Toggle(isOn: $showFronts) {
                    Label("Fronts", systemImage: "door.left.hand.closed")
                }
                .toggleStyle(.button)
                if let cab = project.selected, let b = project.build(cab) {
                    BuildBadge(summary: b)
                }
            }
            .padding(14)
        }
        .background(
            LinearGradient(colors: [Color(white: 0.20), Color(white: 0.11)], startPoint: .top, endPoint: .bottom)
        )
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
    let showFronts: Bool

    var body: some View {
        RealityView { content in
            let root = Entity()
            root.name = "root"
            content.add(root)

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
            let camera = PerspectiveCamera()
            camera.camera.fieldOfViewInDegrees = 35
            camera.look(at: [0, 0.55, -0.3], from: [-2.3, 2.1, 4.3], relativeTo: nil)
            content.add(camera)

            let floor = ModelEntity(
                mesh: .generatePlane(width: 12, depth: 12),
                materials: [SimpleMaterial(color: NSColor(white: 0.32, alpha: 1), roughness: 0.9, isMetallic: false)]
            )
            floor.position.y = -0.0005
            content.add(floor)
        } update: { content in
            guard let root = content.entities.first(where: { $0.name == "root" }) else { return }
            root.children.removeAll()
            var minX = Float.greatestFiniteMagnitude, maxX = -Float.greatestFiniteMagnitude
            for cab in project.cabinets {
                guard let b = project.build(cab) else { continue }
                let selected = cab.id == project.selection
                for p in b.panels where showFronts || !p.isFront {
                    let e = panelEntity(p, selected: selected)
                    e.name = cab.id
                    root.addChild(e)
                    minX = min(minX, Float(p.x) / 1000)
                    maxX = max(maxX, Float(p.x + p.width) / 1000)
                }
            }
            // Keep the run centred on the origin the camera orbits.
            if minX < maxX { root.position.x = -(minX + maxX) / 2 }
        }
        .realityViewCameraControls(.orbit)
        .gesture(
            SpatialTapGesture().targetedToAnyEntity().onEnded { value in
                var e: Entity? = value.entity
                while let cur = e, !project.cabinets.contains(where: { $0.id == cur.name }) { e = cur.parent }
                if let id = e?.name { project.selection = id }
            }
        )
    }

    private func panelEntity(_ p: PanelBox, selected: Bool) -> ModelEntity {
        // Cabinet frame (mm, Z up, Y to the back) → RealityKit (m, Y up, Z to the viewer).
        let size = SIMD3<Float>(Float(p.width), Float(p.height), Float(p.depth)) / 1000
        let centre = SIMD3<Float>(
            Float(p.x + p.width / 2),
            Float(p.z + p.height / 2),
            -Float(p.y + p.depth / 2)
        ) / 1000
        let colour: NSColor = p.isFront
            ? NSColor(calibratedRed: 0.78, green: 0.65, blue: 0.48, alpha: 1)
            : NSColor(calibratedWhite: p.role == "Back" ? 0.78 : 0.93, alpha: 1)
        var material = PhysicallyBasedMaterial()
        material.baseColor = .init(tint: selected ? colour : colour.blended(withFraction: 0.35, of: .gray) ?? colour)
        material.roughness = 0.65
        let e = ModelEntity(mesh: .generateBox(size: size, cornerRadius: 0.0006), materials: [material])
        e.position = centre
        e.components.set(InputTargetComponent())
        e.collision = CollisionComponent(shapes: [.generateBox(size: size)])
        return e
    }
}
