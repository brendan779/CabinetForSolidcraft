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

/// Preferences › General.
struct SettingsView: View {
    @AppStorage(LengthUnit.key) private var unit: LengthUnit = .millimetres

    var body: some View {
        Form {
            Picker("Show lengths in", selection: $unit) {
                ForEach(LengthUnit.allCases) { u in
                    Text(u.title).tag(u)
                }
            }
            .pickerStyle(.radioGroup)
            Text("Cabinets are built, saved and sent to SolidCraft in millimetres. Cut lists exported as CSV stay in millimetres.")
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            Text("e.g. 600 mm = \(unit.format(600))")
                .font(.callout.monospacedDigit())
        }
        .padding(20)
        .frame(width: 420)
    }
}
