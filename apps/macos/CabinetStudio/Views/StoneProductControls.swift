import SwiftUI

struct StoneProductControls: View {
    @Environment(ProjectModel.self) private var project
    @State private var picking = false
    @State private var search = ""
    let cab: CabinetDoc
    private var selected: BoardRecord? { project.boardsById[cab.stoneProductID] }
    var body: some View {
        Button { picking = true } label: {
            HStack {
                Text(selected.map { "\($0.brand) · \($0.decor) · \($0.finish)" } ?? "Choose stone product")
                    .lineLimit(2)
                Spacer()
                Image(systemName: "chevron.right")
            }.padding(8).contentShape(Rectangle())
        }
        .buttonStyle(TileStyle(selected: picking))
        .popover(isPresented: $picking, arrowEdge: .leading) {
            VStack(alignment: .leading, spacing: 10) {
                Text("Stone slabs").font(.headline)
                TextField("Search stone", text: $search).textFieldStyle(.roundedBorder)
                ScrollView {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 170))], spacing: 12) {
                        ForEach(project.boards.filter { $0.kind == "sintered_stone" && (search.isEmpty || "\($0.decor) \($0.finish)".localizedCaseInsensitiveContains(search)) }, id: \.id) { board in
                            Button {
                                var c = cab
                                c.setStoneProduct(board)
                                project.selected = c
                                picking = false
                            } label: {
                                VStack(alignment: .leading, spacing: 4) {
                                    if let surface = SlabLibrary.surface(board), let image = NSImage(contentsOf: surface.imageURL) {
                                        Image(nsImage: image).resizable().scaledToFit().frame(height: 85)
                                    }
                                    Text(board.decor).font(.headline)
                                    Text("\(board.finish) · 3200 × 1600 × 20 mm").font(.caption).foregroundStyle(.secondary)
                                    if board.range.contains("discontinued") { Text("Discontinued").font(.caption).foregroundStyle(Theme.warning) }
                                }.frame(maxWidth: .infinity, alignment: .leading).padding(8)
                            }.buttonStyle(TileStyle(selected: selected?.id == board.id))
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
