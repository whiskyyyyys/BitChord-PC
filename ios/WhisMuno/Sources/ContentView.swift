import SwiftUI

enum MainTab: String, CaseIterable {
    case home = "Beranda"
    case search = "Cari"
    case library = "Koleksi"

    var icon: String {
        switch self {
        case .home: return "house.fill"
        case .search: return "magnifyingglass"
        case .library: return "books.vertical.fill"
        }
    }
}

struct ContentView: View {
    @EnvironmentObject var player: AudioPlayerManager

    @State private var selectedTab: MainTab = .home
    @State private var showFullPlayer: Bool = false

    @State private var searchQuery: String = ""
    @State private var searchResults: [Song] = []
    @State private var isSearching: Bool = false
    @State private var searchError: String?

    @State private var librarySegment: Int = 0

    var body: some View {
        ZStack(alignment: .bottom) {
            WhisTheme.background.ignoresSafeArea()

            VStack(spacing: 0) {
                topBrandNotchHeader

                Group {
                    switch selectedTab {
                    case .home:
                        homeView
                    case .search:
                        searchView
                    case .library:
                        libraryView
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)

                if let current = player.currentSong {
                    miniPlayerBar(song: current)
                        .padding(.horizontal, 14)
                        .padding(.bottom, 8)
                        .transition(.move(edge: .bottom).combined(with: .opacity))
                }

                bottomTabBar
            }
        }
        .sheet(isPresented: $showFullPlayer) {
            NowPlayingView()
                .environmentObject(player)
        }
    }

    // MARK: - Top Brand & Dynamic Live Lyric Pill

    private var topBrandNotchHeader: some View {
        HStack(spacing: 12) {
            HStack(spacing: 8) {
                Circle()
                    .fill(WhisTheme.accentGradient)
                    .frame(width: 10, height: 10)
                    .shadow(color: WhisTheme.mint.opacity(0.65), radius: 6)

                Text("WhisMuno")
                    .font(.headline.weight(.bold))
                    .foregroundColor(.white)
            }

            Spacer()

            if let current = player.currentSong {
                Button {
                    showFullPlayer = true
                } label: {
                    HStack(spacing: 8) {
                        Image(systemName: player.isPlaying ? "waveform" : "music.note")
                            .font(.caption.weight(.bold))
                            .foregroundColor(WhisTheme.mint)

                        Text(player.activeLyricText ?? current.title)
                            .font(.caption.weight(.semibold))
                            .foregroundColor(.white.opacity(0.9))
                            .lineLimit(1)
                            .frame(maxWidth: 165, alignment: .leading)
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 7)
                    .background(WhisTheme.surfaceElevated)
                    .clipShape(Capsule())
                    .overlay(
                        Capsule().stroke(WhisTheme.mint.opacity(0.32), lineWidth: 1)
                    )
                }
                .animation(.easeInOut(duration: 0.25), value: player.activeLyricText)
            } else {
                Text("v2.0.10")
                    .font(.caption2.weight(.semibold))
                    .foregroundColor(WhisTheme.mint)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 4)
                    .background(WhisTheme.mint.opacity(0.12))
                    .clipShape(Capsule())
            }
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 12)
    }

    // MARK: - Home View

    private var homeView: some View {
        ScrollView(showsIndicators: false) {
            VStack(alignment: .leading, spacing: 22) {
                if !player.recents.isEmpty {
                    VStack(alignment: .leading, spacing: 12) {
                        sectionTitle("Baru Diputar")
                        ScrollView(.horizontal, showsIndicators: false) {
                            HStack(spacing: 14) {
                                ForEach(player.recents.prefix(12)) { song in
                                    recentCard(song: song, queue: player.recents)
                                }
                            }
                            .padding(.horizontal, 18)
                        }
                    }
                }

                VStack(alignment: .leading, spacing: 12) {
                    sectionTitle("Pilihan Cepat")

                    if player.isLoadingQuickPicks {
                        HStack {
                            Spacer()
                            ProgressView().tint(WhisTheme.mint)
                            Spacer()
                        }
                        .padding(.vertical, 36)
                    } else if player.quickPicks.isEmpty {
                        VStack(spacing: 10) {
                            Text("Belum ada rekomendasi termuat.")
                                .font(.subheadline)
                                .foregroundColor(WhisTheme.textSecondary)
                            Button("Muat Ulang") {
                                Task { await player.loadQuickPicksIfNeeded() }
                            }
                            .font(.subheadline.weight(.semibold))
                            .foregroundColor(WhisTheme.mint)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 32)
                    } else {
                        LazyVStack(spacing: 10) {
                            ForEach(player.quickPicks) { song in
                                songRow(song: song, queue: player.quickPicks)
                            }
                        }
                        .padding(.horizontal, 18)
                    }
                }

                Spacer().frame(height: 20)
            }
            .padding(.top, 6)
        }
    }

    // MARK: - Search View

    private var searchView: some View {
        VStack(spacing: 14) {
            HStack(spacing: 10) {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(WhisTheme.mint)

                TextField("Cari lagu, artis, atau lirik...", text: $searchQuery)
                    .foregroundColor(.white)
                    .submitLabel(.search)
                    .onSubmit {
                        runSearch()
                    }

                if !searchQuery.isEmpty {
                    Button {
                        searchQuery = ""
                        searchResults = []
                        searchError = nil
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(WhisTheme.textSecondary)
                    }
                }
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 11)
            .background(WhisTheme.surfaceElevated)
            .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 14, style: .continuous)
                    .stroke(WhisTheme.border, lineWidth: 1)
            )
            .padding(.horizontal, 18)

            if isSearching {
                Spacer()
                ProgressView().tint(WhisTheme.mint)
                Text("Mencari di YouTube Music...")
                    .font(.caption)
                    .foregroundColor(WhisTheme.textSecondary)
                    .padding(.top, 8)
                Spacer()
            } else if let err = searchError {
                Spacer()
                Text(err)
                    .font(.subheadline)
                    .foregroundColor(.red.opacity(0.9))
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 24)
                Spacer()
            } else if searchResults.isEmpty {
                Spacer()
                Image(systemName: "music.quarternote.3")
                    .font(.system(size: 42))
                    .foregroundColor(WhisTheme.mint.opacity(0.55))
                Text("Ketik judul lagu atau nama artis lalu tekan Cari")
                    .font(.subheadline)
                    .foregroundColor(WhisTheme.textSecondary)
                    .multilineTextAlignment(.center)
                    .padding(.top, 10)
                    .padding(.horizontal, 36)
                Spacer()
            } else {
                ScrollView(showsIndicators: false) {
                    LazyVStack(spacing: 10) {
                        ForEach(searchResults) { song in
                            songRow(song: song, queue: searchResults)
                        }
                    }
                    .padding(.horizontal, 18)
                    .padding(.bottom, 20)
                }
            }
        }
        .padding(.top, 6)
    }

    // MARK: - Library View

    private var libraryView: some View {
        VStack(spacing: 14) {
            Picker("Koleksi", selection: $librarySegment) {
                Text("Favorit (\(player.favorites.count))").tag(0)
                Text("Riwayat (\(player.recents.count))").tag(1)
            }
            .pickerStyle(.segmented)
            .padding(.horizontal, 18)

            let items = librarySegment == 0 ? player.favorites : player.recents
            if items.isEmpty {
                Spacer()
                Image(systemName: librarySegment == 0 ? "heart.slash" : "clock.arrow.circlepath")
                    .font(.system(size: 40))
                    .foregroundColor(WhisTheme.textSecondary)
                Text(librarySegment == 0 ? "Belum ada lagu favorit." : "Belum ada riwayat putar.")
                    .font(.subheadline)
                    .foregroundColor(WhisTheme.textSecondary)
                    .padding(.top, 8)
                Spacer()
            } else {
                ScrollView(showsIndicators: false) {
                    LazyVStack(spacing: 10) {
                        ForEach(items) { song in
                            songRow(song: song, queue: items)
                        }
                    }
                    .padding(.horizontal, 18)
                    .padding(.bottom, 20)
                }
            }
        }
        .padding(.top, 6)
    }

    // MARK: - Reusable Components

    private func sectionTitle(_ title: String) -> some View {
        Text(title)
            .font(.title3.weight(.bold))
            .foregroundColor(.white)
            .padding(.horizontal, 18)
    }

    private func recentCard(song: Song, queue: [Song]) -> some View {
        Button {
            player.play(song: song, inQueue: queue)
        } label: {
            VStack(alignment: .leading, spacing: 8) {
                AsyncImage(url: song.highResArtworkURL) { phase in
                    switch phase {
                    case .success(let image):
                        image.resizable().scaledToFill()
                    default:
                        ZStack {
                            WhisTheme.surfaceElevated
                            Image(systemName: "music.note").foregroundColor(WhisTheme.mint)
                        }
                    }
                }
                .frame(width: 132, height: 132)
                .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 16, style: .continuous)
                        .stroke(
                            player.currentSong?.id == song.id ? WhisTheme.mint : WhisTheme.border,
                            lineWidth: player.currentSong?.id == song.id ? 2 : 1
                        )
                )

                Text(song.title)
                    .font(.subheadline.weight(.semibold))
                    .foregroundColor(.white)
                    .lineLimit(1)
                Text(song.artist)
                    .font(.caption)
                    .foregroundColor(WhisTheme.textSecondary)
                    .lineLimit(1)
            }
            .frame(width: 132, alignment: .leading)
        }
    }

    private func songRow(song: Song, queue: [Song]) -> some View {
        let isCurrent = player.currentSong?.id == song.id

        return Button {
            player.play(song: song, inQueue: queue)
        } label: {
            HStack(spacing: 12) {
                AsyncImage(url: song.highResArtworkURL) { phase in
                    switch phase {
                    case .success(let image):
                        image.resizable().scaledToFill()
                    default:
                        ZStack {
                            WhisTheme.surfaceElevated
                            Image(systemName: "music.note").foregroundColor(WhisTheme.mint)
                        }
                    }
                }
                .frame(width: 52, height: 52)
                .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))

                VStack(alignment: .leading, spacing: 3) {
                    Text(song.title)
                        .font(.subheadline.weight(.semibold))
                        .foregroundColor(isCurrent ? WhisTheme.mint : .white)
                        .lineLimit(1)

                    Text(song.artist)
                        .font(.caption)
                        .foregroundColor(WhisTheme.textSecondary)
                        .lineLimit(1)
                }

                Spacer()

                if song.durationSeconds > 0 {
                    Text(song.formattedDuration)
                        .font(.caption2.monospacedDigit())
                        .foregroundColor(WhisTheme.textSecondary)
                }

                Button {
                    player.toggleFavorite(song)
                } label: {
                    Image(systemName: player.isFavorite(song) ? "heart.fill" : "heart")
                        .font(.subheadline)
                        .foregroundColor(player.isFavorite(song) ? WhisTheme.mint : WhisTheme.textSecondary)
                        .padding(6)
                }
            }
            .padding(10)
            .background(isCurrent ? WhisTheme.mint.opacity(0.10) : WhisTheme.surface)
            .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 16, style: .continuous)
                    .stroke(isCurrent ? WhisTheme.mint.opacity(0.45) : WhisTheme.border, lineWidth: 1)
            )
        }
    }

    private func miniPlayerBar(song: Song) -> some View {
        Button {
            showFullPlayer = true
        } label: {
            HStack(spacing: 12) {
                AsyncImage(url: song.highResArtworkURL) { phase in
                    switch phase {
                    case .success(let image):
                        image.resizable().scaledToFill()
                    default:
                        ZStack {
                            WhisTheme.surface
                            Image(systemName: "music.note").foregroundColor(WhisTheme.mint)
                        }
                    }
                }
                .frame(width: 44, height: 44)
                .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))

                VStack(alignment: .leading, spacing: 2) {
                    Text(song.title)
                        .font(.subheadline.weight(.semibold))
                        .foregroundColor(.white)
                        .lineLimit(1)
                    Text(player.activeLyricText ?? song.artist)
                        .font(.caption)
                        .foregroundColor(WhisTheme.mint.opacity(0.9))
                        .lineLimit(1)
                }

                Spacer()

                Button {
                    player.togglePlayPause()
                } label: {
                    Group {
                        if player.isLoadingStream {
                            ProgressView().tint(WhisTheme.mint)
                        } else {
                            Image(systemName: player.isPlaying ? "pause.fill" : "play.fill")
                                .font(.system(size: 18, weight: .bold))
                                .foregroundColor(.white)
                        }
                    }
                    .frame(width: 34, height: 34)
                }

                Button {
                    player.playNext()
                } label: {
                    Image(systemName: "forward.fill")
                        .font(.system(size: 16))
                        .foregroundColor(.white.opacity(0.85))
                        .frame(width: 30, height: 30)
                }
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 10)
            .background(WhisTheme.surfaceElevated.opacity(0.96))
            .clipShape(RoundedRectangle(cornerRadius: 18, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .stroke(WhisTheme.mint.opacity(0.28), lineWidth: 1)
            )
            .shadow(color: .black.opacity(0.45), radius: 14, y: 6)
        }
    }

    private var bottomTabBar: some View {
        HStack {
            ForEach(MainTab.allCases, id: \.self) { tab in
                let isSelected = (selectedTab == tab)
                Button {
                    withAnimation(.easeInOut(duration: 0.18)) {
                        selectedTab = tab
                    }
                } label: {
                    VStack(spacing: 4) {
                        Image(systemName: tab.icon)
                            .font(.system(size: 18, weight: .semibold))
                        Text(tab.rawValue)
                            .font(.caption2.weight(.semibold))
                    }
                    .foregroundColor(isSelected ? WhisTheme.mint : WhisTheme.textSecondary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                }
            }
        }
        .padding(.horizontal, 12)
        .background(WhisTheme.surface.opacity(0.96))
        .overlay(
            Rectangle()
                .frame(height: 1)
                .foregroundColor(WhisTheme.border),
            alignment: .top
        )
    }

    private func runSearch() {
        let q = searchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !q.isEmpty else { return }
        isSearching = true
        searchError = nil
        Task {
            do {
                let songs = try await YouTubeMusicService.shared.searchSongs(query: q)
                searchResults = songs
            } catch {
                searchError = error.localizedDescription
            }
            isSearching = false
        }
    }
}
