import SwiftUI

struct SidebarView: View {
    @Environment(ProjectModel.self) private var project

    var body: some View {
        @Bindable var project = project
        List(selection: $project.selection) {
            Section {
                ForEach(project.cabinets) { cab in
                    CabinetRow(cab: cab)
                        .tag(cab.id)
                        .contextMenu {
                            Button("Delete", role: .destructive) { project.remove(cab.id) }
                        }
                }
                .onMove { project.move(from: $0, to: $1) }
                .onDelete { idx in idx.map { project.cabinets[$0].id }.forEach(project.remove) }
            } header: {
                HStack {
                    Text(project.jobName)
                    Spacer()
                    Menu {
                        ForEach(cabinetKinds(), id: \.self) { kind in
                            Button {
                                project.add(kind: kind)
                            } label: {
                                Label(CabinetDoc.kindTitle(kind), systemImage: CabinetDoc.kindSymbol(kind))
                            }
                        }
                    } label: {
                        Image(systemName: "plus")
                    }
                    .menuStyle(.borderlessButton)
                    .menuIndicator(.hidden)
                    .fixedSize()
                    .help("Add a cabinet")
                }
            }
        }
        .listStyle(.sidebar)
    }
}

struct CabinetRow: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    let cab: CabinetDoc

    var body: some View {
        HStack(spacing: 10) {
            CabinetGlyph(doc: cab)
                .frame(width: 26, height: 30)
            VStack(alignment: .leading, spacing: 1) {
                HStack(spacing: 6) {
                    Text(cab.id).font(.headline).monospaced()
                    Text(CabinetDoc.kindTitle(cab.kind)).foregroundStyle(.secondary)
                }
                Text([cab.width, cab.height, cab.depth].map { unit.format($0, suffix: false) }.joined(separator: " × "))
                    .font(.caption)
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if project.error(for: cab) != nil {
                Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
            }
        }
        .padding(.vertical, 2)
    }
}

/// A tiny front elevation of a cabinet, drawn from its fronts.
struct CabinetGlyph: View {
    let doc: CabinetDoc

    var body: some View {
        Canvas { ctx, size in
            let aspect = doc.width / max(doc.height, 1)
            let h = size.height
            let w = min(size.width, h * aspect)
            let r = CGRect(x: (size.width - w) / 2, y: 0, width: w, height: h)
            ctx.stroke(Path(roundedRect: r, cornerRadius: 1.5), with: .color(.primary.opacity(0.8)), lineWidth: 1)
            switch doc.frontType {
            case "door_pair":
                var p = Path()
                p.move(to: CGPoint(x: r.midX, y: r.minY))
                p.addLine(to: CGPoint(x: r.midX, y: r.maxY))
                ctx.stroke(p, with: .color(.primary.opacity(0.6)), lineWidth: 0.8)
            case "drawers":
                let ratios = doc.drawerRatios
                let total = ratios.reduce(0, +)
                var y = r.minY
                for ratio in ratios.dropLast() {
                    y += r.height * ratio / max(total, 0.001)
                    var p = Path()
                    p.move(to: CGPoint(x: r.minX, y: y))
                    p.addLine(to: CGPoint(x: r.maxX, y: y))
                    ctx.stroke(p, with: .color(.primary.opacity(0.6)), lineWidth: 0.8)
                }
            case "open":
                for i in 1..<3 {
                    let y = r.minY + r.height * CGFloat(i) / 3
                    var p = Path()
                    p.move(to: CGPoint(x: r.minX + 2, y: y))
                    p.addLine(to: CGPoint(x: r.maxX - 2, y: y))
                    ctx.stroke(p, with: .color(.primary.opacity(0.4)), lineWidth: 0.8)
                }
            default:
                break
            }
        }
    }
}
