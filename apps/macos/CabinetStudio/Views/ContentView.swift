import SwiftUI

struct ContentView: View {
    @Environment(ProjectModel.self) private var project
    @State private var showInspector = true
    @AppStorage("welcomeShown") private var welcomeShown = false

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
            // Fixed size rules for both split view columns, whatever they show. When a column's
            // minimum followed its content (the Boards grid, or the inspector switching to
            // "No inspector" without a width), AppKit's split view re-measured endlessly and crashed.
            .frame(minWidth: 0, maxWidth: .infinity, minHeight: 0, maxHeight: .infinity)
            .inspector(isPresented: $showInspector) {
                Group {
                    if project.workspace == .design {
                        InspectorView()
                    } else {
                        ContentUnavailableView("No inspector", systemImage: "sidebar.right",
                                               description: Text("Select the Design workspace to edit cabinets."))
                    }
                }
                .inspectorColumnWidth(min: 300, ideal: 340, max: 420)
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
                // A toolbar menu draws only its label's icon and title, so the dot stands beside it.
                ConnectionDot()
                SolidCraftButton()

                Button {
                    showInspector.toggle()
                } label: {
                    Label("Inspector", systemImage: "sidebar.right")
                }
            }
        }
        .safeAreaInset(edge: .bottom) { StatusBar() }
        // An overlay, not .sheet: dismissing an AppKit sheet over this split view + inspector sent
        // AppKit into an endless constraint-update loop that crashed the app.
        .overlay {
            if project.showWelcome {
                ZStack {
                    Color.black.opacity(0.35)
                        .ignoresSafeArea()
                        .onTapGesture {}
                    WelcomeSheet()
                        .background(.regularMaterial, in: .rect(cornerRadius: 16))
                        .shadow(color: .black.opacity(0.3), radius: 24, y: 8)
                }
                .transition(.opacity)
            }
        }
        .animation(.snappy, value: project.showWelcome)
        .onAppear { if !welcomeShown { project.showWelcome = true } }
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
                Text("Exact: true biscuit slot shape (about 3× slower)").tag(DetailLevel.exact)
            }
            .pickerStyle(.inline)
            Divider()
            if project.connection == .offline {
                Button("Open SolidCraft", systemImage: "arrow.up.forward.app") {
                    Task { await project.openSolidCraft() }
                }
            }
            Button("Make Cabinets from Selected Boxes", systemImage: "cube.transparent") {
                Task { await project.makeCabinetsFromSelection() }
            }
            Button("Load Cabinets from Design", systemImage: "square.and.arrow.down") {
                Task { await project.loadCabinetsFromDesign() }
            }
            Button("Render to PNG", systemImage: "camera.aperture") {
                Task { await project.render() }
            }
            Button("Check Connection") { Task { await project.checkConnection() } }
        } label: {
            Label("SolidCraft", systemImage: project.isSending ? "hourglass" : "shippingbox")
                .labelStyle(.titleAndIcon)
        } primaryAction: {
            Task { await project.sendToSolidCraft() }
        }
        .disabled(project.isSending)
        .help(helpText + " Click to build; hold for options.")
    }

    private var helpText: String {
        switch project.connection {
        case .connected: "SolidCraft is connected on port \(project.port)"
        case .offline: "SolidCraft is not answering on port \(project.port). Start it with --control \(project.port)."
        case .unknown: "Checking for SolidCraft…"
        }
    }
}

/// SolidCraft's connection as a coloured dot: green connected, red not answering, grey checking.
struct ConnectionDot: View {
    @Environment(ProjectModel.self) private var project

    var body: some View {
        ZStack {
            if project.isSending {
                ProgressView().controlSize(.small)
            } else {
                Circle()
                    .fill(colour)
                    .frame(width: 9, height: 9)
                    .shadow(color: colour.opacity(0.6), radius: project.connection == .connected ? 3 : 0)
            }
        }
        .frame(width: 18, height: 18)
        .animation(.snappy, value: project.connection)
        .help(help)
        .accessibilityElement()
        .accessibilityLabel("SolidCraft")
        .accessibilityValue(help)
    }

    private var colour: Color {
        switch project.connection {
        case .connected: .green
        case .offline: .red
        case .unknown: .secondary
        }
    }

    private var help: String {
        switch project.connection {
        case .connected: "Connected"
        case .offline: "Not answering on port \(project.port)"
        case .unknown: "Checking…"
        }
    }
}

struct StatusBar: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres

    var body: some View {
        HStack(spacing: 10) {
            if let err = project.lastError {
                Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
                Text(err).lineLimit(1).truncationMode(.middle)
            } else if project.connection == .offline {
                Image(systemName: "bolt.horizontal.circle").foregroundStyle(.red)
                Text("SolidCraft is not connected")
                Button("Open SolidCraft") { Task { await project.openSolidCraft() } }
                    .buttonStyle(.link)
            } else {
                Text(project.status.isEmpty ? "\(project.cabinets.count) cabinets" : project.status)
                    .lineLimit(1)
            }
            Spacer()
            Text("Run \(unit.format(project.runLength))").monospacedDigit()
        }
        .font(.callout)
        .foregroundStyle(.secondary)
        .padding(.horizontal, 14)
        .padding(.vertical, 6)
        .background(.bar)
    }
}
