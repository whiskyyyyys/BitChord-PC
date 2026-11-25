import AVFoundation
import Combine
import Foundation
import MediaPlayer
import UIKit

@MainActor
final class AudioPlayerManager: ObservableObject {
    @Published var currentSong: Song?
    @Published var queue: [Song] = []
    @Published var isPlaying: Bool = false
    @Published var isLoadingStream: Bool = false
    @Published var currentTime: Double = 0
    @Published var duration: Double = 0
    @Published var lyrics: [LyricLine] = []
    @Published var isLoadingLyrics: Bool = false
    @Published var errorMessage: String?

    @Published var favorites: [Song] = []
    @Published var recents: [Song] = []
    @Published var quickPicks: [Song] = []
    @Published var isLoadingQuickPicks: Bool = false

    private var player: AVPlayer?
    private var timeObserver: Any?
    private var endObserver: NSObjectProtocol?
    private var currentStreamTask: Task<Void, Never>?
    private var cachedArtworkImage: UIImage?

    private let favoritesKey = "whismuno_favorites_v1"
    private let recentsKey = "whismuno_recents_v1"

    init() {
        loadPersistedLists()
        configureAudioSession()
        configureRemoteCommands()
        Task {
            await loadQuickPicksIfNeeded()
        }
    }

    deinit {
        if let endObserver {
            NotificationCenter.default.removeObserver(endObserver)
        }
    }

    var activeLyricIndex: Int? {
        guard !lyrics.isEmpty else { return nil }
        let synced = lyrics.filter(\.isSynced)
        guard !synced.isEmpty else { return nil }
        let posMs = Int64((currentTime + 0.12) * 1000.0)
        var best: Int?
        for line in lyrics where line.isSynced {
            if line.timeMs <= posMs {
                best = line.id
            } else {
                break
            }
        }
        return best
    }

    var activeLyricText: String? {
        guard let idx = activeLyricIndex,
              let line = lyrics.first(where: { $0.id == idx }) else {
            return nil
        }
        return line.text
    }

    func isFavorite(_ song: Song) -> Bool {
        favorites.contains(where: { $0.id == song.id })
    }

    func toggleFavorite(_ song: Song) {
        if let idx = favorites.firstIndex(where: { $0.id == song.id }) {
            favorites.remove(at: idx)
        } else {
            favorites.insert(song, at: 0)
        }
        persistLists()
    }

    func loadQuickPicksIfNeeded() async {
        guard quickPicks.isEmpty && !isLoadingQuickPicks else { return }
        isLoadingQuickPicks = true
        let picks = await YouTubeMusicService.shared.fetchQuickPicks()
        quickPicks = picks
        isLoadingQuickPicks = false
    }

    func play(song: Song, inQueue newQueue: [Song]? = nil) {
        if let newQueue, !newQueue.isEmpty {
            queue = newQueue
        } else if !queue.contains(where: { $0.id == song.id }) {
            queue.insert(song, at: 0)
        }

        currentStreamTask?.cancel()
        errorMessage = nil
        currentSong = song
        currentTime = 0
        duration = song.durationSeconds
        lyrics = []
        isPlaying = false
        isLoadingStream = true
        cachedArtworkImage = nil

        addToRecents(song)
        updateNowPlayingInfo()

        currentStreamTask = Task { [weak self] in
            guard let self else { return }
            await self.startPlayback(for: song)
        }
    }

    private func startPlayback(for song: Song) async {
        async let artworkFetch: UIImage? = fetchArtworkImage(for: song)
        async let lyricsFetch: [LyricLine] = {
            self.isLoadingLyrics = true
            let lines = await YouTubeMusicService.shared.fetchLyrics(
                title: song.title,
                artist: song.artist,
                duration: song.durationSeconds
            )
            return lines
        }()

        do {
            let resolved = try await YouTubeMusicService.shared.resolveStreamURL(videoId: song.id)
            guard !Task.isCancelled, currentSong?.id == song.id else { return }

            if let dur = resolved.durationSeconds, dur > 0 {
                duration = dur
            }

            cleanupPlayerObservers()

            let asset = AVURLAsset(
                url: resolved.url,
                options: [
                    "AVURLAssetHTTPHeaderFieldsKey": [
                        "User-Agent": resolved.userAgent
                    ]
                ]
            )
            let item = AVPlayerItem(asset: asset)
            let newPlayer = AVPlayer(playerItem: item)
            newPlayer.automaticallyWaitsToMinimizeStalling = true
            player = newPlayer

            setupPlayerObservers(for: newPlayer, item: item)
            newPlayer.play()
            isPlaying = true
            isLoadingStream = false
            updateNowPlayingInfo()

            cachedArtworkImage = await artworkFetch
            updateNowPlayingInfo()

            let fetchedLyrics = await lyricsFetch
            if !Task.isCancelled && currentSong?.id == song.id {
                lyrics = fetchedLyrics
                isLoadingLyrics = false
            }
        } catch {
            guard !Task.isCancelled else { return }
            isLoadingStream = false
            isLoadingLyrics = false
            isPlaying = false
            errorMessage = error.localizedDescription
        }
    }

    func togglePlayPause() {
        guard let player else {
            if let song = currentSong {
                play(song: song, inQueue: queue)
            }
            return
        }
        if isPlaying {
            player.pause()
            isPlaying = false
        } else {
            try? AVAudioSession.sharedInstance().setActive(true)
            player.play()
            isPlaying = true
        }
        updateNowPlayingInfo()
    }

    func seek(to seconds: Double) {
        guard let player else { return }
        let clamped = max(0, min(seconds, duration > 0 ? duration : seconds))
        let target = CMTime(seconds: clamped, preferredTimescale: 600)
        currentTime = clamped
        player.seek(to: target, toleranceBefore: .zero, toleranceAfter: .zero) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.updateNowPlayingInfo()
            }
        }
    }

    func playNext() {
        guard let current = currentSong, !queue.isEmpty else { return }
        if let idx = queue.firstIndex(where: { $0.id == current.id }) {
            let nextIdx = (idx + 1) % queue.count
            play(song: queue[nextIdx], inQueue: queue)
        } else if let first = queue.first {
            play(song: first, inQueue: queue)
        }
    }

    func playPrevious() {
        if currentTime > 3 {
            seek(to: 0)
            return
        }
        guard let current = currentSong, !queue.isEmpty else { return }
        if let idx = queue.firstIndex(where: { $0.id == current.id }) {
            let prevIdx = (idx - 1 + queue.count) % queue.count
            play(song: queue[prevIdx], inQueue: queue)
        }
    }

    // MARK: - Private Setup & Persistence

    private func configureAudioSession() {
        do {
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.playback, mode: .default, options: [])
            try session.setActive(true)
        } catch {
            // Best-effort configuration
        }
    }

    private func configureRemoteCommands() {
        let center = MPRemoteCommandCenter.shared()
        center.playCommand.isEnabled = true
        center.playCommand.addTarget { [weak self] _ in
            Task { @MainActor [weak self] in
                guard let self else { return }
                if !self.isPlaying { self.togglePlayPause() }
            }
            return .success
        }

        center.pauseCommand.isEnabled = true
        center.pauseCommand.addTarget { [weak self] _ in
            Task { @MainActor [weak self] in
                guard let self else { return }
                if self.isPlaying { self.togglePlayPause() }
            }
            return .success
        }

        center.nextTrackCommand.isEnabled = true
        center.nextTrackCommand.addTarget { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.playNext()
            }
            return .success
        }

        center.previousTrackCommand.isEnabled = true
        center.previousTrackCommand.addTarget { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.playPrevious()
            }
            return .success
        }

        center.changePlaybackPositionCommand.isEnabled = true
        center.changePlaybackPositionCommand.addTarget { [weak self] event in
            guard let posEvent = event as? MPChangePlaybackPositionCommandEvent else {
                return .commandFailed
            }
            Task { @MainActor [weak self] in
                self?.seek(to: posEvent.positionTime)
            }
            return .success
        }
    }

    private func setupPlayerObservers(for player: AVPlayer, item: AVPlayerItem) {
        let interval = CMTime(seconds: 0.12, preferredTimescale: 600)
        timeObserver = player.addPeriodicTimeObserver(forInterval: interval, queue: .main) { [weak self] time in
            Task { @MainActor [weak self] in
                guard let self else { return }
                let secs = time.seconds
                if secs.isFinite && secs >= 0 {
                    self.currentTime = secs
                }
                if let itemDur = player.currentItem?.duration.seconds,
                   itemDur.isFinite && itemDur > 0 {
                    self.duration = itemDur
                }
            }
        }

        endObserver = NotificationCenter.default.addObserver(
            forName: .AVPlayerItemDidPlayToEndTime,
            object: item,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.playNext()
            }
        }
    }

    private func cleanupPlayerObservers() {
        if let observer = timeObserver, let oldPlayer = player {
            oldPlayer.removeTimeObserver(observer)
            timeObserver = nil
        }
        if let endObserver {
            NotificationCenter.default.removeObserver(endObserver)
            self.endObserver = nil
        }
        player?.pause()
    }

    private func updateNowPlayingInfo() {
        guard let song = currentSong else {
            MPNowPlayingInfoCenter.default().nowPlayingInfo = nil
            return
        }

        var info: [String: Any] = [
            MPMediaItemPropertyTitle: song.title,
            MPMediaItemPropertyArtist: song.artist,
            MPMediaItemPropertyPlaybackDuration: duration > 0 ? duration : song.durationSeconds,
            MPNowPlayingInfoPropertyElapsedPlaybackTime: currentTime,
            MPNowPlayingInfoPropertyPlaybackRate: isPlaying ? 1.0 : 0.0
        ]

        if let image = cachedArtworkImage {
            info[MPMediaItemPropertyArtwork] = MPMediaItemArtwork(boundsSize: image.size) { _ in image }
        }

        MPNowPlayingInfoCenter.default().nowPlayingInfo = info
    }

    private func fetchArtworkImage(for song: Song) async -> UIImage? {
        guard let url = song.highResArtworkURL else { return nil }
        guard let (data, _) = try? await URLSession.shared.data(from: url) else { return nil }
        return UIImage(data: data)
    }

    private func addToRecents(_ song: Song) {
        recents.removeAll(where: { $0.id == song.id })
        recents.insert(song, at: 0)
        if recents.count > 40 {
            recents = Array(recents.prefix(40))
        }
        persistLists()
    }

    private func loadPersistedLists() {
        let decoder = JSONDecoder()
        if let favData = UserDefaults.standard.data(forKey: favoritesKey),
           let decoded = try? decoder.decode([Song].self, from: favData) {
            favorites = decoded
        }
        if let recData = UserDefaults.standard.data(forKey: recentsKey),
           let decoded = try? decoder.decode([Song].self, from: recData) {
            recents = decoded
        }
    }

    private func persistLists() {
        let encoder = JSONEncoder()
        if let favData = try? encoder.encode(favorites) {
            UserDefaults.standard.set(favData, forKey: favoritesKey)
        }
        if let recData = try? encoder.encode(recents) {
            UserDefaults.standard.set(recData, forKey: recentsKey)
        }
    }
}
