import Foundation
import SwiftUI

struct Song: Identifiable, Codable, Hashable {
    let id: String
    let title: String
    let artist: String
    let thumbnailURL: String?
    let durationSeconds: Double

    var highResArtworkURL: URL? {
        guard let raw = thumbnailURL, !raw.isEmpty else { return nil }
        let upgraded = raw
            .replacingOccurrences(of: "w60-h60", with: "w544-h544")
            .replacingOccurrences(of: "w120-h120", with: "w544-h544")
            .replacingOccurrences(of: "w226-h226", with: "w544-h544")
        return URL(string: upgraded)
    }

    var formattedDuration: String {
        Self.formatTime(durationSeconds)
    }

    static func formatTime(_ seconds: Double) -> String {
        guard seconds.isFinite && seconds > 0 else { return "0:00" }
        let total = Int(seconds.rounded(.down))
        let mins = total / 60
        let secs = total % 60
        return String(format: "%d:%02d", mins, secs)
    }
}

struct LyricLine: Identifiable, Hashable {
    let id: Int
    let timeMs: Int64
    let text: String

    var isSynced: Bool { timeMs >= 0 }
}

enum WhisTheme {
    static let background = Color(hex: 0x04070D)
    static let surface = Color(hex: 0x0B121E)
    static let surfaceElevated = Color(hex: 0x111C2D)
    static let mint = Color(hex: 0x66EBCB)
    static let aqua = Color(hex: 0x38BDF8)
    static let textPrimary = Color.white
    static let textSecondary = Color.white.opacity(0.68)
    static let border = Color.white.opacity(0.10)

    static let accentGradient = LinearGradient(
        colors: [mint, aqua],
        startPoint: .topLeading,
        endPoint: .bottomTrailing
    )
}

extension Color {
    init(hex: UInt, alpha: Double = 1.0) {
        self.init(
            .sRGB,
            red: Double((hex >> 16) & 0xFF) / 255.0,
            green: Double((hex >> 8) & 0xFF) / 255.0,
            blue: Double(hex & 0xFF) / 255.0,
            opacity: alpha
        )
    }
}
