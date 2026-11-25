import Foundation

enum InnerTubeError: LocalizedError {
    case invalidURL
    case badResponse
    case streamNotFound(String)

    var errorDescription: String? {
        switch self {
        case .invalidURL:
            return "URL tidak valid."
        case .badResponse:
            return "Gagal menghubungi server musik."
        case .streamNotFound(let reason):
            return reason.isEmpty ? "Stream audio tidak ditemukan." : reason
        }
    }
}

final class YouTubeMusicService {
    static let shared = YouTubeMusicService()

    private let session: URLSession = {
        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = 18
        config.timeoutIntervalForResource = 30
        return URLSession(configuration: config)
    }()

    private init() {}

    // MARK: - Search Songs via YouTube Music WEB_REMIX

    func searchSongs(query: String) async throws -> [Song] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return [] }
        guard let url = URL(string: "https://music.youtube.com/youtubei/v1/search?prettyPrint=false") else {
            throw InnerTubeError.invalidURL
        }

        let body: [String: Any] = [
            "context": [
                "client": [
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20250310.01.00",
                    "hl": "id",
                    "gl": "ID"
                ]
            ],
            "query": trimmed,
            "params": "EgWKAQIIAWoMEAMQBBAJEAoQBRAV"
        ]

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("https://music.youtube.com", forHTTPHeaderField: "Origin")
        request.setValue("https://music.youtube.com/", forHTTPHeaderField: "Referer")
        request.setValue(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 18_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.3 Mobile/15E148 Safari/604.1",
            forHTTPHeaderField: "User-Agent"
        )
        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw InnerTubeError.badResponse
        }
        guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw InnerTubeError.badResponse
        }

        var results: [Song] = []
        var seenIds = Set<String>()
        collectSongs(from: json, into: &results, seenIds: &seenIds)
        return results
    }

    func fetchQuickPicks() async -> [Song] {
        let queries = ["Lagu Indonesia Populer Terbaru", "Global Top Hits 2025"]
        var combined: [Song] = []
        var seen = Set<String>()

        for q in queries {
            if let songs = try? await searchSongs(query: q) {
                for song in songs.prefix(8) where !seen.contains(song.id) {
                    seen.insert(song.id)
                    combined.append(song)
                }
            }
        }
        return combined
    }

    private func collectSongs(from node: Any, into results: inout [Song], seenIds: inout Set<String>) {
        if let dict = node as? [String: Any] {
            if let renderer = dict["musicResponsiveListItemRenderer"] as? [String: Any],
               let song = parseSongRenderer(renderer),
               !seenIds.contains(song.id) {
                seenIds.insert(song.id)
                results.append(song)
            }
            for value in dict.values {
                collectSongs(from: value, into: &results, seenIds: &seenIds)
            }
        } else if let array = node as? [Any] {
            for item in array {
                collectSongs(from: item, into: &results, seenIds: &seenIds)
            }
        }
    }

    private func parseSongRenderer(_ renderer: [String: Any]) -> Song? {
        var videoId: String?
        if let playlistItem = renderer["playlistItemData"] as? [String: Any],
           let id = playlistItem["videoId"] as? String {
            videoId = id
        } else if let overlay = renderer["overlay"] as? [String: Any],
                  let overlayRenderer = overlay["musicItemThumbnailOverlayRenderer"] as? [String: Any],
                  let content = overlayRenderer["content"] as? [String: Any],
                  let playButton = content["musicPlayButtonRenderer"] as? [String: Any],
                  let nav = playButton["playNavigationEndpoint"] as? [String: Any],
                  let watch = nav["watchEndpoint"] as? [String: Any],
                  let id = watch["videoId"] as? String {
            videoId = id
        }

        guard let validId = videoId, !validId.isEmpty else { return nil }

        let flexColumns = renderer["flexColumns"] as? [[String: Any]] ?? []
        let title = extractTextRuns(from: flexColumns.first) ?? "Unknown Track"
        let subtitleRuns = extractAllRuns(from: flexColumns.dropFirst().first)

        var artist = "Unknown Artist"
        var durationSeconds: Double = 0

        if !subtitleRuns.isEmpty {
            let meaningful = subtitleRuns
                .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                .filter { !$0.isEmpty && $0 != "•" && $0 != "·" && $0 != "," }

            if let last = meaningful.last, let parsedDur = parseDurationString(last) {
                durationSeconds = parsedDur
                let artistParts = meaningful.dropLast()
                let joined = artistParts.joined(separator: " ")
                if !joined.isEmpty { artist = joined }
            } else if let first = meaningful.first {
                artist = first
            }
        }

        var thumbURL: String?
        if let thumb = renderer["thumbnail"] as? [String: Any],
           let musicThumb = thumb["musicThumbnailRenderer"] as? [String: Any],
           let innerThumb = musicThumb["thumbnail"] as? [String: Any],
           let thumbs = innerThumb["thumbnails"] as? [[String: Any]],
           let best = thumbs.last,
           let rawURL = best["url"] as? String {
            thumbURL = rawURL
        }

        return Song(
            id: validId,
            title: title,
            artist: artist,
            thumbnailURL: thumbURL,
            durationSeconds: durationSeconds
        )
    }

    private func extractTextRuns(from column: [String: Any]?) -> String? {
        let runs = extractAllRuns(from: column)
        let text = runs.joined().trimmingCharacters(in: .whitespacesAndNewlines)
        return text.isEmpty ? nil : text
    }

    private func extractAllRuns(from column: [String: Any]?) -> [String] {
        guard let col = column,
              let item = col["musicResponsiveListItemFlexColumnRenderer"] as? [String: Any],
              let text = item["text"] as? [String: Any],
              let runs = text["runs"] as? [[String: Any]] else {
            return []
        }
        return runs.compactMap { $0["text"] as? String }
    }

    private func parseDurationString(_ raw: String) -> Double? {
        let parts = raw.split(separator: ":").compactMap { Int($0) }
        guard parts.count >= 2 && parts.count <= 3 else { return nil }
        if parts.count == 2 {
            return Double(parts[0] * 60 + parts[1])
        } else {
            return Double(parts[0] * 3600 + parts[1] * 60 + parts[2])
        }
    }

    // MARK: - Resolve Audio Stream (VISIONOS -> IOS -> ANDROID_VR)

    struct ResolvedStream {
        let url: URL
        let durationSeconds: Double?
        let userAgent: String
    }

    func resolveStreamURL(videoId: String) async throws -> ResolvedStream {
        // iOS AVPlayer natively supports AAC/MP4 (itag 140/139), NOT WebM/Opus (itag 251).
        if let vision = try? await fetchPlayerStream(
            videoId: videoId,
            clientName: "VISIONOS",
            clientVersion: "1.02",
            clientHeaderId: "101",
            userAgent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15",
            extraClientFields: [
                "deviceMake": "Apple",
                "deviceModel": "RealityDevice17,1",
                "osName": "visionOS",
                "osVersion": "26.5.23O471"
            ]
        ) {
            return vision
        }

        return try await fetchPlayerStream(
            videoId: videoId,
            clientName: "IOS",
            clientVersion: "20.10.4",
            clientHeaderId: "5",
            userAgent: "com.google.ios.youtube/20.10.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)",
            extraClientFields: [
                "deviceMake": "Apple",
                "deviceModel": "iPhone16,2",
                "osName": "iOS",
                "osVersion": "18.3.2.22D82"
            ]
        )
    }

    private func fetchPlayerStream(
        videoId: String,
        clientName: String,
        clientVersion: String,
        clientHeaderId: String,
        userAgent: String,
        extraClientFields: [String: Any]
    ) async throws -> ResolvedStream {
        guard let url = URL(string: "https://www.youtube.com/youtubei/v1/player?prettyPrint=false") else {
            throw InnerTubeError.invalidURL
        }

        var clientDict: [String: Any] = [
            "clientName": clientName,
            "clientVersion": clientVersion,
            "hl": "en",
            "gl": "US"
        ]
        for (k, v) in extraClientFields {
            clientDict[k] = v
        }

        let body: [String: Any] = [
            "context": [
                "client": clientDict
            ],
            "videoId": videoId,
            "contentCheckOk": true,
            "racyCheckOk": true
        ]

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue(userAgent, forHTTPHeaderField: "User-Agent")
        request.setValue(clientHeaderId, forHTTPHeaderField: "X-YouTube-Client-Name")
        request.setValue(clientVersion, forHTTPHeaderField: "X-YouTube-Client-Version")
        request.setValue("https://www.youtube.com", forHTTPHeaderField: "Origin")
        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        let (data, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
            throw InnerTubeError.badResponse
        }
        guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw InnerTubeError.badResponse
        }

        if let playability = json["playabilityStatus"] as? [String: Any],
           let status = playability["status"] as? String,
           status != "OK" {
            let reason = (playability["reason"] as? String) ?? status
            throw InnerTubeError.streamNotFound(reason)
        }

        var durationSeconds: Double?
        if let details = json["videoDetails"] as? [String: Any],
           let lengthStr = details["lengthSeconds"] as? String,
           let parsed = Double(lengthStr) {
            durationSeconds = parsed
        }

        let streamingData = json["streamingData"] as? [String: Any] ?? [:]
        let adaptive = streamingData["adaptiveFormats"] as? [[String: Any]] ?? []
        let muxed = streamingData["formats"] as? [[String: Any]] ?? []

        // Prefer AAC/MP4 adaptive audio for native AVPlayer compatibility on iOS
        let preferredItags = [140, 139, 141]
        for itag in preferredItags {
            if let match = adaptive.first(where: { ($0["itag"] as? Int) == itag }),
               let urlStr = match["url"] as? String,
               let streamURL = URL(string: urlStr) {
                return ResolvedStream(url: streamURL, durationSeconds: durationSeconds, userAgent: userAgent)
            }
        }

        let mp4Audio = adaptive
            .filter {
                let mime = ($0["mimeType"] as? String ?? "").lowercased()
                return mime.contains("audio/mp4") && ($0["url"] as? String) != nil
            }
            .sorted { ($0["bitrate"] as? Int ?? 0) > ($1["bitrate"] as? Int ?? 0) }

        if let best = mp4Audio.first,
           let urlStr = best["url"] as? String,
           let streamURL = URL(string: urlStr) {
            return ResolvedStream(url: streamURL, durationSeconds: durationSeconds, userAgent: userAgent)
        }

        // Fallback to muxed mp4 (itag 18 etc.) if adaptive URLs unavailable
        if let fallback = muxed.first(where: {
            let mime = ($0["mimeType"] as? String ?? "").lowercased()
            return mime.contains("mp4") && ($0["url"] as? String) != nil
        }), let urlStr = fallback["url"] as? String, let streamURL = URL(string: urlStr) {
            return ResolvedStream(url: streamURL, durationSeconds: durationSeconds, userAgent: userAgent)
        }

        throw InnerTubeError.streamNotFound("Format audio MP4/AAC tidak tersedia untuk lagu ini.")
    }

    // MARK: - Synced Lyrics via LRCLIB

    func fetchLyrics(title: String, artist: String, duration: Double) async -> [LyricLine] {
        let cleanTitle = cleanMetadata(title)
        let cleanArtist = cleanMetadata(artist)

        if let exact = await queryLRCLibExact(title: cleanTitle, artist: cleanArtist, duration: duration),
           !exact.isEmpty {
            return exact
        }

        if let searched = await queryLRCLibSearch(query: "\(cleanTitle) \(cleanArtist)"),
           !searched.isEmpty {
            return searched
        }

        return []
    }

    private func queryLRCLibExact(title: String, artist: String, duration: Double) async -> [LyricLine]? {
        var components = URLComponents(string: "https://lrclib.net/api/get")
        var items = [
            URLQueryItem(name: "track_name", value: title),
            URLQueryItem(name: "artist_name", value: artist)
        ]
        if duration > 10 {
            items.append(URLQueryItem(name: "duration", value: String(Int(duration.rounded()))))
        }
        components?.queryItems = items

        guard let url = components?.url else { return nil }
        var request = URLRequest(url: url)
        request.setValue("WhisMuno-iOS/2.0.10", forHTTPHeaderField: "User-Agent")

        guard let (data, response) = try? await session.data(for: request),
              let http = response as? HTTPURLResponse,
              http.statusCode == 200,
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return nil
        }

        return parseLRCLibRecord(obj)
    }

    private func queryLRCLibSearch(query: String) async -> [LyricLine]? {
        var components = URLComponents(string: "https://lrclib.net/api/search")
        components?.queryItems = [URLQueryItem(name: "q", value: query)]

        guard let url = components?.url else { return nil }
        var request = URLRequest(url: url)
        request.setValue("WhisMuno-iOS/2.0.10", forHTTPHeaderField: "User-Agent")

        guard let (data, response) = try? await session.data(for: request),
              let http = response as? HTTPURLResponse,
              http.statusCode == 200,
              let list = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
            return nil
        }

        for item in list {
            if let synced = item["syncedLyrics"] as? String, !synced.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                let parsed = parseLRC(synced)
                if !parsed.isEmpty { return parsed }
            }
        }
        if let first = list.first {
            return parseLRCLibRecord(first)
        }
        return nil
    }

    private func parseLRCLibRecord(_ obj: [String: Any]) -> [LyricLine] {
        if let synced = obj["syncedLyrics"] as? String, !synced.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            let parsed = parseLRC(synced)
            if !parsed.isEmpty { return parsed }
        }
        if let plain = obj["plainLyrics"] as? String, !plain.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            let lines = plain
                .components(separatedBy: .newlines)
                .map { $0.trimmingCharacters(in: .whitespaces) }
                .filter { !$0.isEmpty }
            return lines.enumerated().map { LyricLine(id: $0.offset, timeMs: -1, text: $0.element) }
        }
        return []
    }

    private func parseLRC(_ lrc: String) -> [LyricLine] {
        let pattern = #"\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else { return [] }

        var parsed: [(Int64, String)] = []
        for rawLine in lrc.components(separatedBy: .newlines) {
            let line = rawLine.trimmingCharacters(in: .whitespaces)
            guard !line.isEmpty else { continue }
            let nsLine = line as NSString
            let matches = regex.matches(in: line, range: NSRange(location: 0, length: nsLine.length))
            guard !matches.isEmpty else { continue }

            let lastEnd = matches.last!.range.location + matches.last!.range.length
            let lyricText = nsLine.substring(from: lastEnd).trimmingCharacters(in: .whitespaces)
            guard !lyricText.isEmpty else { continue }

            for m in matches {
                let minStr = nsLine.substring(with: m.range(at: 1))
                let secStr = nsLine.substring(with: m.range(at: 2))
                var msVal: Int64 = 0
                if m.range(at: 3).location != NSNotFound {
                    let fracStr = nsLine.substring(with: m.range(at: 3))
                    if fracStr.count == 1 {
                        msVal = (Int64(fracStr) ?? 0) * 100
                    } else if fracStr.count == 2 {
                        msVal = (Int64(fracStr) ?? 0) * 10
                    } else {
                        msVal = Int64(fracStr.prefix(3)) ?? 0
                    }
                }
                let mins = Int64(minStr) ?? 0
                let secs = Int64(secStr) ?? 0
                let totalMs = mins * 60_000 + secs * 1000 + msVal
                parsed.append((totalMs, lyricText))
            }
        }

        parsed.sort { $0.0 < $1.0 }
        return parsed.enumerated().map { idx, pair in
            LyricLine(id: idx, timeMs: pair.0, text: pair.1)
        }
    }

    private func cleanMetadata(_ input: String) -> String {
        var out = input
        let patterns = [
            #"\(feat\.[^)]*\)"#,
            #"\(ft\.[^)]*\)"#,
            #"\[[^\]]*\]"#,
            #"\(Official[^)]*\)"#,
            #"\(Lyric[^)]*\)"#,
            #"\(Audio[^)]*\)"#
        ]
        for p in patterns {
            if let regex = try? NSRegularExpression(pattern: p, options: [.caseInsensitive]) {
                let range = NSRange(location: 0, length: (out as NSString).length)
                out = regex.stringByReplacingMatches(in: out, range: range, withTemplate: "")
            }
        }
        return out.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
