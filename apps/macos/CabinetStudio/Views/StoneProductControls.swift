import SwiftUI

struct StoneProductControls: View {
    @Environment(ProjectModel.self) private var project
    @State private var picking = false
    @State private var search = ""
    let cab: CabinetDoc
    private var selected: BoardRecord? { project.boardsById[cab.stoneProductID] }
    /// A laminate top picks from laminate tops; any other material from stone slabs.
    private var laminate: Bool { cab.material == "laminate" }
    private var kind: String { laminate ? "laminate_top" : "sintered_stone" }
    private var slabs: [BoardRecord] {
        project.boards.filter { $0.kind == kind && (search.isEmpty
            || "\($0.brand) \($0.range) \($0.decor) \($0.finish)".localizedCaseInsensitiveContains(search)) }
    }
    private var brands: [String] { Array(Set(slabs.map(\.brand))).sorted() }
    var body: some View {
        Button { picking = true } label: {
            HStack {
                Text(selected.map { "\($0.brand) · \($0.decor) · \($0.finish)" } ?? (laminate ? "Choose laminate top" : "Choose stone product"))
                    .lineLimit(2)
                Spacer()
                Image(systemName: "chevron.right")
            }.padding(8).contentShape(Rectangle())
        }
        .buttonStyle(TileStyle(selected: picking))
        .popover(isPresented: $picking, arrowEdge: .leading) {
            VStack(alignment: .leading, spacing: 10) {
                Text(laminate ? "Laminate tops" : "Stone slabs").font(.headline)
                TextField(laminate ? "Search colour, finish or profile" : "Search stone", text: $search).textFieldStyle(.roundedBorder)
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 14, pinnedViews: .sectionHeaders) {
                        ForEach(brands, id: \.self) { brand in
                            Section {
                                LazyVGrid(columns: [GridItem(.adaptive(minimum: 170))], spacing: 12) {
                                    ForEach(slabs.filter { $0.brand == brand }, id: \.id) { board in
                                        Button {
                                            var c = cab
                                            c.setStoneProduct(board)
                                            project.selected = c
                                            picking = false
                                        } label: {
                                            SlabTile(board: board)
                                        }.buttonStyle(TileStyle(selected: selected?.id == board.id))
                                    }
                                }
                            } header: {
                                Text(brand).font(.subheadline.weight(.semibold)).foregroundStyle(.secondary)
                                    .frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, 4)
                                    .background(Theme.overlay)
                            }
                        }
                    }
                }
                Button("Use generic material") {
                    var c = cab; c.setStoneProduct(nil); project.selected = c; picking = false
                }.buttonStyle(.borderless)
            }.padding(14).frame(width: 630, height: 550)
        }
    }
}

/// One slab in the picker: a small picture of its photograph, its name, finish and size.
private struct SlabTile: View {
    let board: BoardRecord
    @State private var thumbnail: NSImage?
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Group {
                if let thumbnail {
                    Image(nsImage: thumbnail).resizable().scaledToFill()
                } else {
                    Color(nsColor: NSColor(hex: board.previewColour) ?? .gray)
                }
            }.frame(height: 85).frame(maxWidth: .infinity).clipped()
            Text(board.decor).font(.headline)
            Text("\(board.finish) · \(board.sheets.first ?? "") × \(Int(board.thickness)) mm").font(.caption).foregroundStyle(.secondary)
            if board.range.contains("discontinued") { Text("Discontinued").font(.caption).foregroundStyle(Theme.warning) }
        }
        .frame(maxWidth: .infinity, alignment: .leading).padding(8)
        .task(id: board.id) {
            if let surface = SlabLibrary.surface(board) { thumbnail = await DecorThumbnails.image(surface.imageName) }
        }
    }
}
