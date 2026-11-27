import CryptoKit
import Foundation

/// The official material packs (docs/material-packs.md): each brand's decor and slab pictures, as
/// assets of the GitHub release `material-packs`. On launch the app downloads any it is missing, or
/// has an older version of, checks each against the index's checksum, and installs it with the
/// core's checker, which writes nothing unless the whole pack is sound.
@MainActor @Observable final class MaterialPacks {
    static let shared = MaterialPacks()
    static let index = URL(string: "https://github.com/brendan779/CabinetForSolidcraft/releases/download/material-packs/packs.json")!
    static var appDir: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appending(path: "CabinetStudio")
    }

    /// What the status bar shows while packs come in; nil when there's nothing to say.
    private(set) var message: String?
    /// How far the whole download is, 0 to 1; nil when none is running.
    private(set) var fraction: Double?
    private var running = false

    private struct Pack: Decodable {
        let id, name, version, file, sha256: String
        let size: Int64
    }
    private struct Index: Decodable { let packs: [Pack] }

    func updateIfNeeded() async {
        guard !running else { return }
        running = true
        defer { running = false; fraction = nil }
        do {
            let (data, response) = try await URLSession.shared.data(from: Self.index)
            // No packs published, or offline: the app works with colours until they are.
            guard (response as? HTTPURLResponse)?.statusCode == 200 else { return }
            let index = try JSONDecoder().decode(Index.self, from: data)
            let dir = Self.appDir.path
            let have = Dictionary(uniqueKeysWithValues: installedMaterialPacks(appDir: dir).map { ($0.id, $0.version) })
            // Versions are dates, YYYY.MM.DD: a later one sorts after.
            let wanted = index.packs.filter { (have[$0.id] ?? "") < $0.version }
            guard !wanted.isEmpty else { return }
            let total = Double(wanted.reduce(0) { $0 + $1.size })
            var done = 0.0
            fraction = 0
            for pack in wanted {
                message = "Downloading \(pack.name)…"
                let url = Self.index.deletingLastPathComponent().appending(path: pack.file)
                let (file, reply) = try await URLSession.shared.download(from: url)
                guard (reply as? HTTPURLResponse)?.statusCode == 200 else {
                    throw CocoaError(.fileReadUnknown, userInfo: [NSLocalizedDescriptionKey: "\(pack.file) is not on GitHub"])
                }
                let installed = try await Task.detached(priority: .utility) { () throws -> InstalledPack in
                    defer { try? FileManager.default.removeItem(at: file) }
                    let bytes = try Data(contentsOf: file, options: .mappedIfSafe)
                    let sum = SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
                    guard sum == pack.sha256 else {
                        throw CocoaError(.fileReadCorruptFile, userInfo: [NSLocalizedDescriptionKey: "\(pack.file) did not download whole"])
                    }
                    return try installMaterialPack(zipPath: file.path, appDir: dir)
                }.value
                done += Double(pack.size)
                fraction = done / max(total, 1)
                message = "Installed \(installed.name) (\(installed.images) pictures)"
            }
            message = wanted.count == 1 ? "Materials up to date" : "\(wanted.count) material packs installed"
            try? await Task.sleep(for: .seconds(6))
            message = nil
        } catch {
            message = "Materials not downloaded: \(error.localizedDescription)"
        }
    }
}
