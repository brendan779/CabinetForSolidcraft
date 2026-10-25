import SwiftUI

/// The first-run welcome card (shown over the job window, not as an AppKit sheet): connect SolidCraft, pick how much detail to build, then start. Help ›
/// Welcome to Cabinet Studio shows it again.
struct WelcomeSheet: View {
    @Environment(ProjectModel.self) private var project
    @AppStorage("welcomeShown") private var welcomeShown = false

    var body: some View {
        @Bindable var project = project
        VStack(alignment: .leading, spacing: 22) {
            HStack(spacing: 14) {
                Image(nsImage: NSApp.applicationIconImage)
                    .resizable()
                    .frame(width: 56, height: 56)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    Text("Welcome to Cabinet Studio").font(.title2.weight(.semibold))
                    Text("Draw cabinets here; SolidCraft builds them as solid models.")
                        .foregroundStyle(.secondary)
                }
            }

            step(1, "Connect SolidCraft") {
                HStack(spacing: 10) {
                    ConnectionDot()
                    Text(connectionText).foregroundStyle(.secondary)
                    Spacer()
                    if project.connection == .connected {
                        Label("Ready", systemImage: "checkmark.circle.fill")
                            .foregroundStyle(.green)
                    } else {
                        Button("Open SolidCraft") { Task { await project.openSolidCraft() } }
                        Button("Check Again") { Task { await project.checkConnection() } }
                    }
                }
            }

            step(2, "How much to build") {
                HStack(spacing: 10) {
                    ForEach(DetailLevel.allCases, id: \.self) { level in
                        Button {
                            withAnimation(.snappy) { project.detail = level }
                        } label: {
                            VStack(spacing: 6) {
                                DetailThumbnail(level: level).frame(height: 54)
                                Text(level.rawValue).font(.callout.weight(.medium))
                                Text(blurb(level))
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .multilineTextAlignment(.center)
                                    .lineLimit(3, reservesSpace: true)
                            }
                            .padding(10)
                            .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(TileStyle(selected: project.detail == level))
                        .accessibilityLabel("\(level.rawValue) detail")
                        .accessibilityHint(blurb(level))
                    }
                }
            }

            step(3, "Start") {
                Text("Add cabinets with + in the sidebar, pick options by their pictures in the inspector, then Build in SolidCraft (⇧⌘B). Cut lists and cut sheets follow every change.")
                    .foregroundStyle(.secondary)
                    .lineLimit(4, reservesSpace: true)
            }

            HStack {
                Spacer()
                Button("Get Started") {
                    welcomeShown = true
                    project.showWelcome = false
                }
                .keyboardShortcut(.defaultAction)
            }
        }
        .padding(26)
        .frame(width: 600, height: 540)
        .task { await project.checkConnection() }
    }

    private var connectionText: String {
        switch project.connection {
        case .connected: "SolidCraft answers on port \(project.port)."
        case .offline: "SolidCraft is not answering on port \(project.port)."
        case .unknown: "Looking for SolidCraft…"
        }
    }

    private func blurb(_ level: DetailLevel) -> String {
        switch level {
        case .design: "Plain panels. Fast, for laying out a kitchen."
        case .manufacturing: "Every bore, groove and slot, ready for the workshop."
        case .exact: "True biscuit slot shapes. About three times slower."
        }
    }

    private func step<Content: View>(_ n: Int, _ title: String, @ViewBuilder content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Text("\(n)")
                    .font(.caption.weight(.bold))
                    .frame(width: 20, height: 20)
                    .background(Circle().fill(Color.accentColor.opacity(0.2)))
                Text(title).font(.headline)
            }
            content()
        }
    }
}

/// A panel corner as each detail level builds it: plain, machined, or with lens-shaped slots.
struct DetailThumbnail: View {
    let level: DetailLevel

    var body: some View {
        Canvas { ctx, size in
            let r = CGRect(x: size.width * 0.15, y: size.height * 0.2, width: size.width * 0.7, height: size.height * 0.6)
            ctx.fill(Path(roundedRect: r, cornerRadius: 2), with: .color(.primary.opacity(0.1)))
            ctx.stroke(Path(roundedRect: r, cornerRadius: 2), with: .color(.primary.opacity(0.7)), lineWidth: 1)
            guard level != .design else { return }
            for i in 0..<4 {
                let x = r.minX + r.width * (0.2 + 0.2 * CGFloat(i))
                let hole = Path(ellipseIn: CGRect(x: x - 2.5, y: r.midY - 2.5, width: 5, height: 5))
                ctx.stroke(hole, with: .color(.accentColor), lineWidth: 1.2)
            }
            let slot = CGRect(x: r.minX + r.width * 0.3, y: r.maxY - 8, width: r.width * 0.4, height: 5)
            let shape = level == .exact ? Path(ellipseIn: slot) : Path(slot)
            ctx.fill(shape, with: .color(.accentColor.opacity(0.4)))
            ctx.stroke(shape, with: .color(.accentColor), lineWidth: 1)
        }
        .accessibilityHidden(true)
    }
}
