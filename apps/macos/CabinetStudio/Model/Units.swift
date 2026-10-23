import AppKit
import SwiftUI

/// How lengths are shown. The model, the files and SolidCraft stay in millimetres; this only
/// changes the numbers on screen.
enum LengthUnit: String, CaseIterable, Identifiable {
    case millimetres, inches

    var id: String { rawValue }

    var title: String {
        switch self {
        case .millimetres: "Millimetres"
        case .inches: "Inches"
        }
    }

    /// The `@AppStorage` key the choice is kept under.
    static let key = "lengthUnit"

    /// `mm` as text: whole millimetres (one decimal when it has one), or inches to the nearest
    /// sixteenth as a mixed fraction (23 5/8″).
    func format(_ mm: Double, suffix: Bool = true) -> String {
        switch self {
        case .millimetres:
            let s = mm.rounded() == mm ? String(Int(mm)) : String(format: "%.1f", mm)
            return suffix ? "\(s) mm" : s
        case .inches:
            let sixteenths = Int((mm / 25.4 * 16).rounded())
            let whole = sixteenths / 16
            var num = sixteenths % 16, den = 16
            while num > 0, num % 2 == 0 { num /= 2; den /= 2 }
            let s = num == 0 ? "\(whole)" : whole == 0 ? "\(num)/\(den)" : "\(whole) \(num)/\(den)"
            return suffix ? "\(s)″" : s
        }
    }
}

/// Settings kept in user defaults, read where they are used.
enum Preferences {
    static let portKey = "solidcraftPort"
    static let catalogKey = "catalogFolder"
    static let vendorCADKey = "vendorCADFolder"
    static let defaultPort = 7878

    /// SolidCraft's control port.
    static var port: UInt16 {
        let p = UserDefaults.standard.integer(forKey: portKey)
        return UInt16(exactly: p == 0 ? defaultPort : p) ?? UInt16(defaultPort)
    }

    /// The board and hardware catalog: the folder chosen in Settings, or the one inside the app.
    static var catalogDir: URL? {
        if let path = UserDefaults.standard.string(forKey: catalogKey), !path.isEmpty,
           FileManager.default.fileExists(atPath: path) {
            return URL(fileURLWithPath: path, isDirectory: true)
        }
        return Bundle.main.url(forResource: "catalog", withExtension: nil)
    }
}

struct SettingsView: View {
    var body: some View {
        TabView {
            GeneralSettings().tabItem { Label("General", systemImage: "gearshape") }
            JobSettings().tabItem { Label("Jobs", systemImage: "doc.on.doc") }
            FolderSettings().tabItem { Label("Folders", systemImage: "folder") }
        }
        .frame(width: 480)
    }
}

/// Units and the SolidCraft connection.
struct GeneralSettings: View {
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres
    @AppStorage(Preferences.portKey) private var port = Preferences.defaultPort

    var body: some View {
        Form {
            Picker("Show lengths in", selection: $unit) {
                ForEach(LengthUnit.allCases) { u in
                    Text(u.title).tag(u)
                }
            }
            .pickerStyle(.radioGroup)
            Text("Cabinets are built, saved and sent to SolidCraft in millimetres. Cut lists exported as CSV stay in millimetres. e.g. 600 mm = \(unit.format(600))")
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            LabeledContent("SolidCraft port") {
                HStack {
                    TextField("Port", value: $port, format: .number.grouping(.never))
                        .frame(width: 70)
                        .multilineTextAlignment(.trailing)
                    Stepper("Port", value: $port, in: 1024...65535).labelsHidden()
                    if port != Preferences.defaultPort {
                        Button("Default") { port = Preferences.defaultPort }
                    }
                }
            }
            Text("SolidCraft listens here when started with --control \(String(port)). Check Connection in the SolidCraft menu after changing it.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(20)
    }
}

/// The standard new jobs start from.
struct JobSettings: View {
    @AppStorage(StandardLibrary.defaultKey) private var defaultStandard = ""
    @State private var library = StandardLibrary.shared

    var body: some View {
        Form {
            Picker("New jobs use", selection: $defaultStandard) {
                Text("The built-in standard").tag("")
                if !library.standards.isEmpty { Divider() }
                ForEach(library.standards) { s in
                    Text(s.name).tag(s.name)
                }
            }
            Text(library.standards.isEmpty
                 ? "Save a standard from a job (Construction › Save…) to start new jobs from it."
                 : "Standards are saved from a job's Construction section.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(20)
        .onAppear { library.reload() }
    }
}

/// Where the catalog and the supplier CAD files live.
struct FolderSettings: View {
    @AppStorage(Preferences.catalogKey) private var catalog = ""
    @AppStorage(Preferences.vendorCADKey) private var vendorCAD = ""

    var body: some View {
        Form {
            folder("Catalog", path: $catalog, empty: "Inside the app",
                   note: "A folder with boards/ and hardware/ like the repository's catalog. Jobs opened after a change use it.")
            folder("Supplier CAD", path: $vendorCAD, empty: "Not set",
                   note: "Blum and Häfele STEP files you downloaded (vendor-cad/). Used to replace stand-in hardware with the real models.")
        }
        .padding(20)
    }

    private func folder(_ title: String, path: Binding<String>, empty: String, note: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            LabeledContent(title) {
                HStack {
                    Text(path.wrappedValue.isEmpty ? empty : (path.wrappedValue as NSString).abbreviatingWithTildeInPath)
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .foregroundStyle(path.wrappedValue.isEmpty ? .secondary : .primary)
                    Button("Choose…") {
                        let panel = NSOpenPanel()
                        panel.canChooseDirectories = true
                        panel.canChooseFiles = false
                        panel.allowsMultipleSelection = false
                        if panel.runModal() == .OK, let url = panel.url { path.wrappedValue = url.path }
                    }
                    if !path.wrappedValue.isEmpty {
                        Button("Clear") { path.wrappedValue = "" }
                    }
                }
            }
            Text(note)
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}
