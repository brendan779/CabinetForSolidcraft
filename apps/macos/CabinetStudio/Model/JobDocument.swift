import Foundation
import Synchronization
import SwiftUI
import UniformTypeIdentifiers

extension UTType {
    /// A job: the cabinets of one kitchen, laundry or wardrobe, as the core's cabinet JSON.
    static let cabinetJob = UTType(exportedAs: "io.github.brendan779.cabinetjob", conformingTo: .json)
}

/// The file behind a job window. The window's `ProjectModel` is the live state; it hands every
/// edit's JSON to `latest`, which the system reads (on any thread) when it saves or autosaves.
final class JobDocument: ReferenceFileDocument, Sendable {
    static var readableContentTypes: [UTType] { [.cabinetJob, .json] }
    static var writableContentTypes: [UTType] { [.cabinetJob] }

    /// What the file held when it was opened; `nil` for a new job.
    let opened: JobContents?
    private let latest = Mutex<Data?>(nil)

    init() {
        opened = nil
    }

    init(configuration: ReadConfiguration) throws {
        guard let data = configuration.file.regularFileContents else {
            throw CocoaError(.fileReadCorruptFile)
        }
        opened = try JobContents(data: data)
        latest.withLock { $0 = data }
    }

    func update(_ data: Data) {
        latest.withLock { $0 = data }
    }

    func snapshot(contentType: UTType) throws -> Data {
        guard let data = latest.withLock({ $0 }) else { throw CocoaError(.fileWriteUnknown) }
        return data
    }

    func fileWrapper(snapshot: Data, configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: snapshot)
    }
}

/// One job window: its model, made once from the file, and the link back to the file.
struct JobWindow: View {
    let document: JobDocument
    let fileURL: URL?
    @State private var project: ProjectModel
    @Environment(\.undoManager) private var undoManager
    @AppStorage(Preferences.vendorCADKey) private var vendorCAD = ""

    init(document: JobDocument, fileURL: URL?) {
        self.document = document
        self.fileURL = fileURL
        _project = State(initialValue: document.opened.map(ProjectModel.init(contents:)) ?? ProjectModel())
    }

    var body: some View {
        ContentView()
            .environment(project)
            .focusedSceneValue(\.project, project)
            .frame(minWidth: 1100, minHeight: 700)
            .task { await project.checkConnection() }
            .task(id: vendorCAD) { await project.prepareSupplierCAD() }
            .onReceive(NotificationCenter.default.publisher(for: Preferences.supplierCADPrepared)) { _ in
                Task { await project.prepareSupplierCAD() }
            }
            .onAppear {
                project.undoManager = undoManager
                project.onEdit = { [document] data in document.update(data) }
                if let data = try? project.encoded() { document.update(data) }
            }
            .onChange(of: undoManager) { _, new in project.undoManager = new }
            .onChange(of: fileURL, initial: true) { _, url in
                project.fileURL = url
                if let url { project.jobName = url.deletingPathExtension().lastPathComponent }
            }
    }
}

extension FocusedValues {
    /// The job in the key window, for menu commands.
    @Entry var project: ProjectModel?
}
