import SwiftUI

@main
struct CabinetStudioApp: App {
    var body: some Scene {
        DocumentGroup(newDocument: { JobDocument() }) { file in
            JobWindow(document: file.document, fileURL: file.fileURL)
        }
        .windowToolbarStyle(.unified)
        .commands { JobCommands() }
        Settings {
            SettingsView()
        }
    }
}

/// Menu commands for the job in the key window.
struct JobCommands: Commands {
    @FocusedValue(\.project) private var project

    var body: some Commands {
        CommandGroup(after: .newItem) {
            Menu("New Cabinet") {
                ForEach(cabinetKinds(), id: \.self) { kind in
                    Button(CabinetDoc.kindTitle(kind)) { project?.add(kind: kind) }
                }
            }
            .disabled(project == nil)
        }
        CommandMenu("SolidCraft") {
            Button("Build in SolidCraft") { Task { await project?.sendToSolidCraft() } }
                .keyboardShortcut("b", modifiers: [.command, .shift])
                .disabled(project == nil || project?.isSending == true)
            Button("Check Connection") { Task { await project?.checkConnection() } }
                .disabled(project == nil)
        }
    }
}
