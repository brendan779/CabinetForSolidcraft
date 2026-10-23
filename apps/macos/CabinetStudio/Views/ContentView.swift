import SwiftUI

struct ContentView: View {
    @Environment(ProjectModel.self) private var project
    @State private var showInspector = true

    var body: some View {
        @Bindable var project = project
        NavigationSplitView {
            SidebarView()
                .navigationSplitViewColumnWidth(min: 220, ideal: 250, max: 320)
        } detail: {
            Group {
                switch project.workspace {
                case .design: DesignWorkspace()
                case .cutList: CutListView()
                case .sheets: CutSheetsView()
                case .boards: BoardLibraryView()
                }
            }
            .inspector(isPresented: $showInspector) {
                if project.workspace == .design {
                    InspectorView()
                        .inspectorColumnWidth(min: 300, ideal: 340, max: 420)
                } else {
                    ContentUnavailableView("No inspector", systemImage: "sidebar.right",
                                           description: Text("Select the Design workspace to edit cabinets."))
                }
            }
        }
        .toolbar {
            ToolbarItem(placement: .principal) {
                Picker("Workspace", selection: $project.workspace) {
                    ForEach(Workspace.allCases) { w in
                        Text(w.rawValue).tag(w)
                    }
                }
                .pickerStyle(.segmented)
                .fixedSize()
            }
            ToolbarItemGroup(placement: .primaryAction) {
                SolidCraftButton()

                Button {
                    showInspector.toggle()
                } label: {
                    Label("Inspector", systemImage: "sidebar.right")
                }
            }
        }
        .safeAreaInset(edge: .bottom) { StatusBar() }
    }
}

struct SolidCraftButton: View {
    @Environment(ProjectModel.self) private var project

    var body: some View {
        @Bindable var project = project
        Menu {
            Picker("Detail", selection: $project.detail) {
                Text("Design: plain panels").tag(DetailLevel.design)
                Text("Manufacturing: every bore, groove and slot").tag(DetailLevel.manufacturing)
            }
            .pickerStyle(.inline)
            Divider()
            Button("Check Connection") { Task { await project.checkConnection() } }
        } label: {
            HStack(spacing: 6) {
                Circle()
                    .fill(dotColour)
                    .frame(width: 8, height: 8)
                if project.isSending {
                    ProgressView().controlSize(.small)
                }
                Text("SolidCraft")
            }
        } primaryAction: {
            Task { await project.sendToSolidCraft() }
        }
        .disabled(project.isSending)
        .help(helpText + " Click to build; hold for options.")
    }

    private var dotColour: Color {
        switch project.connection {
        case .connected: .green
        case .offline: .red
        case .unknown: .secondary
        }
    }

    private var helpText: String {
        switch project.connection {
        case .connected: "SolidCraft is connected on port \(project.port)"
        case .offline: "SolidCraft is not answering on port \(project.port). Start it with --control \(project.port)."
        case .unknown: "Checking for SolidCraft…"
        }
    }
}

struct StatusBar: View {
    @Environment(ProjectModel.self) private var project

    var body: some View {
        HStack(spacing: 10) {
            if let err = project.lastError {
                Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
                Text(err).lineLimit(1).truncationMode(.middle)
            } else {
                Text(project.status.isEmpty ? "\(project.cabinets.count) cabinets" : project.status)
                    .lineLimit(1)
            }
            Spacer()
            let total = project.cabinets.reduce(0) { $0 + $1.width }
            Text("Run \(Int(total)) mm").monospacedDigit()
        }
        .font(.callout)
        .foregroundStyle(.secondary)
        .padding(.horizontal, 14)
        .padding(.vertical, 6)
        .background(.bar)
    }
}
