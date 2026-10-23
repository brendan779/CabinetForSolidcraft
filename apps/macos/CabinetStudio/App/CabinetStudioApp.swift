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
        CommandGroup(after: .pasteboard) {
            Button("Delete Cabinet") { project?.deleteSelection() }
                .keyboardShortcut(.delete, modifiers: .command)
                .disabled(project?.selection == nil)
        }
        CommandGroup(before: .sidebar) {
            ForEach(Array(Workspace.allCases.enumerated()), id: \.element) { i, w in
                Button(w.rawValue) { project?.workspace = w }
                    .keyboardShortcut(KeyEquivalent(Character("\(i + 1)")), modifiers: .command)
                    .disabled(project == nil)
            }
            Button("Previous Cabinet") { project?.selectNeighbour(-1) }
                .keyboardShortcut("[", modifiers: .command)
                .disabled(project == nil)
            Button("Next Cabinet") { project?.selectNeighbour(1) }
                .keyboardShortcut("]", modifiers: .command)
                .disabled(project == nil)
            Divider()
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
