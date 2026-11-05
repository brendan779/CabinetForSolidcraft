import Foundation

enum ToolRunner {
    struct Failure: LocalizedError, CustomStringConvertible {
        let description: String
        var errorDescription: String? { description }
    }

    /// The process status is authoritative: Blender add-ons can log tracebacks without
    /// failing the export. Blender scripts must be invoked with --python-exit-code.
    static func run(_ tool: URL, _ arguments: [String]) throws {
        let process = Process()
        process.executableURL = tool
        process.arguments = arguments
        let log = FileManager.default.temporaryDirectory.appending(path: "cabinet-studio-\(UUID().uuidString).log")
        FileManager.default.createFile(atPath: log.path, contents: nil)
        defer { try? FileManager.default.removeItem(at: log) }
        let handle = try FileHandle(forWritingTo: log)
        defer { try? handle.close() }
        process.standardOutput = handle
        process.standardError = handle
        try process.run()
        process.waitUntilExit()
        if process.terminationStatus != 0 {
            let text = (try? String(contentsOf: log, encoding: .utf8)) ?? ""
            let tail = text.split(separator: "\n").suffix(8).joined(separator: "\n")
            throw Failure(description: "\(tool.lastPathComponent) failed (exit \(process.terminationStatus)): \(tail)")
        }
    }
}
