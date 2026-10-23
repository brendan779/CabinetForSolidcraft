import SwiftUI

@main
struct CabinetStudioApp: App {
    @State private var project = ProjectModel()

    var body: some Scene {
        WindowGroup("Cabinet Studio") {
            ContentView()
                .environment(project)
                .frame(minWidth: 1100, minHeight: 700)
                .task { await project.checkConnection() }
        }
        .windowToolbarStyle(.unified)
        .commands {
            CommandGroup(after: .newItem) {
                Menu("New Cabinet") {
                    ForEach(cabinetKinds(), id: \.self) { kind in
                        Button(CabinetDoc.kindTitle(kind)) { project.add(kind: kind) }
                    }
                }
            }
            CommandMenu("SolidCraft") {
                Button("Build in SolidCraft") { Task { await project.sendToSolidCraft() } }
                    .keyboardShortcut("b", modifiers: [.command, .shift])
                    .disabled(project.isSending)
                Button("Check Connection") { Task { await project.checkConnection() } }
            }
        }
    }
}
