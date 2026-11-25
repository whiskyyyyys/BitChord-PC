import SwiftUI

struct NowPlayingView: View {
    @EnvironmentObject var player: AudioPlayerManager
    @Environment(\.dismiss) private var dismiss

    @State private var showLyrics: Bool = false
    @State private var isDraggingSlider: Bool = false
    @State private var sliderValue: Double = 0

    var body: some View {
        ZStack {
            WhisTheme.background.ignoresSafeArea()

            // Ambient artwork glow
            if let song = player.currentSong {
                AsyncImage(url: song.highResArtworkURL) { phase in
                    if case .success(let image) = phase {
                        image
                            .resizable()
                            .scaledToFill()
                            .blur(radius: 75)
                            .opacity(0.32)
                            .ignoresSafeArea()
                    }
                }
                LinearGradient(
                    colors: [
                        WhisTheme.background.opacity(0.35),
                        WhisTheme.background.opacity(0.85),
                        WhisTheme.background
                    ],
                    startPoint: .top,
                    endPoint: .bottom
                )
                .ignoresSafeArea()
            }

            VStack(spacing: 18) {
                headerBar

                if showLyrics {
                    lyricsPanel
                        .transition(.opacity.combined(with: .scale(scale: 0.98)))
                } else {
                    artworkPanel
                        .transition(.opacity.combined(with: .scale(scale: 0.98)))
                }

                if let song = player.currentSong {
                    trackMetaAndControls(song: song)
                }
            }
            .padding(.horizontal, 22)
            .padding(.top, 14)
            .padding(.bottom, 28)
        }
        .onAppear {
            sliderValue = player.currentTime
        }
        .onChange(of: player.currentTime) { newVal in
            if !isDraggingSlider {
                sliderValue = newVal
            }
        }
    }

    private var headerBar: some View {
        HStack {
            Button {
                dismiss()
            } label: {
                Image(systemName: "chevron.down")
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundColor(.white.opacity(0.85))
                    .frame(width: 36, height: 36)
                    .background(Color.white.opacity(0.08))
                    .clipShape(Circle())
            }

            Spacer()

            HStack(spacing: 4) {
                modePill(title: "Pemutar", active: !showLyrics) {
                    withAnimation(.spring(response: 0.35, dampingFraction: 0.85)) {
                        showLyrics = false
                    }
                }
                modePill(title: "Lirik", active: showLyrics) {
                    withAnimation(.spring(response: 0.35, dampingFraction: 0.85)) {
                        showLyrics = true
                    }
                }
            }
            .padding(4)
            .background(Color.white.opacity(0.08))
            .clipShape(Capsule())

            Spacer()

            if let song = player.currentSong {
                Button {
                    withAnimation(.spring(response: 0.3, dampingFraction: 0.7)) {
                        player.toggleFavorite(song)
                    }
                } label: {
                    Image(systemName: player.isFavorite(song) ? "heart.fill" : "heart")
                        .font(.system(size: 16, weight: .semibold))
                        .foregroundColor(player.isFavorite(song) ? WhisTheme.mint : .white.opacity(0.85))
                        .frame(width: 36, height: 36)
                        .background(Color.white.opacity(0.08))
                        .clipShape(Circle())
                }
            } else {
                Color.clear.frame(width: 36, height: 36)
            }
        }
    }

    private func modePill(title: String, active: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(.caption.weight(.semibold))
                .foregroundColor(active ? WhisTheme.background : .white.opacity(0.75))
                .padding(.horizontal, 14)
                .padding(.vertical, 6)
                .background(
                    Group {
                        if active {
                            WhisTheme.accentGradient
                        } else {
                            Color.clear
                        }
                    }
                )
                .clipShape(Capsule())
        }
    }

    private var artworkPanel: some View {
        VStack {
            Spacer(minLength: 4)
            if let song = player.currentSong {
                AsyncImage(url: song.highResArtworkURL) { phase in
                    switch phase {
                    case .success(let image):
                        image
                            .resizable()
                            .scaledToFill()
                    default:
                        ZStack {
                            WhisTheme.surfaceElevated
                            Image(systemName: "music.note")
                                .font(.system(size: 56))
                                .foregroundColor(WhisTheme.mint.opacity(0.7))
                        }
                    }
                }
                .frame(maxWidth: 310, maxHeight: 310)
                .aspectRatio(1, contentMode: .fit)
                .clipShape(RoundedRectangle(cornerRadius: 24, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 24, style: .continuous)
                        .stroke(Color.white.opacity(0.14), lineWidth: 1)
                )
                .shadow(color: WhisTheme.mint.opacity(player.isPlaying ? 0.22 : 0.08), radius: 28, y: 14)
                .scaleEffect(player.isPlaying ? 1.0 : 0.92)
                .animation(.spring(response: 0.45, dampingFraction: 0.78), value: player.isPlaying)
            }
            Spacer(minLength: 4)
        }
    }

    private var lyricsPanel: some View {
        VStack {
            if player.isLoadingLyrics {
                Spacer()
                ProgressView()
                    .tint(WhisTheme.mint)
                Text("Mencari lirik tersinkronisasi...")
                    .font(.caption)
                    .foregroundColor(WhisTheme.textSecondary)
                    .padding(.top, 8)
                Spacer()
            } else if player.lyrics.isEmpty {
                Spacer()
                Image(systemName: "quote.bubble")
                    .font(.system(size: 38))
                    .foregroundColor(WhisTheme.textSecondary)
                Text("Lirik tidak ditemukan untuk lagu ini")
                    .font(.subheadline)
                    .foregroundColor(WhisTheme.textSecondary)
                    .padding(.top, 8)
                Spacer()
            } else {
                ScrollViewReader { proxy in
                    ScrollView(showsIndicators: false) {
                        LazyVStack(alignment: .leading, spacing: 18) {
                            Spacer().frame(height: 18)
                            ForEach(player.lyrics) { line in
                                let isActive = (player.activeLyricIndex == line.id)
                                Button {
                                    if line.isSynced {
                                        player.seek(to: Double(line.timeMs) / 1000.0)
                                    }
                                } label: {
                                    Text(line.text)
                                        .font(isActive ? .title2.weight(.bold) : .title3.weight(.semibold))
                                        .foregroundColor(
                                            isActive
                                                ? .white
                                                : (!line.isSynced ? .white.opacity(0.82) : .white.opacity(0.36))
                                        )
                                        .shadow(
                                            color: isActive ? WhisTheme.mint.opacity(0.45) : .clear,
                                            radius: isActive ? 10 : 0
                                        )
                                        .multilineTextAlignment(.leading)
                                        .frame(maxWidth: .infinity, alignment: .leading)
                                        .scaleEffect(isActive ? 1.02 : 1.0, anchor: .leading)
                                        .animation(.easeOut(duration: 0.28), value: isActive)
                                }
                                .id(line.id)
                            }
                            Spacer().frame(height: 60)
                        }
                        .padding(.horizontal, 4)
                    }
                    .onChange(of: player.activeLyricIndex) { activeId in
                        if let activeId {
                            withAnimation(.easeInOut(duration: 0.35)) {
                                proxy.scrollTo(activeId, anchor: .center)
                            }
                        }
                    }
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func trackMetaAndControls(song: Song) -> some View {
        VStack(spacing: 16) {
            VStack(spacing: 4) {
                Text(song.title)
                    .font(.title3.weight(.bold))
                    .foregroundColor(.white)
                    .lineLimit(1)
                Text(song.artist)
                    .font(.subheadline)
                    .foregroundColor(WhisTheme.mint.opacity(0.92))
                    .lineLimit(1)
            }
            .frame(maxWidth: .infinity)

            if let error = player.errorMessage {
                Text(error)
                    .font(.caption)
                    .foregroundColor(.red.opacity(0.9))
                    .multilineTextAlignment(.center)
            }

            VStack(spacing: 4) {
                let maxDur = max(player.duration, 1)
                Slider(
                    value: Binding(
                        get: { min(sliderValue, maxDur) },
                        set: { sliderValue = $0 }
                    ),
                    in: 0...maxDur,
                    onEditingChanged: { editing in
                        isDraggingSlider = editing
                        if !editing {
                            player.seek(to: sliderValue)
                        }
                    }
                )
                .tint(WhisTheme.mint)

                HStack {
                    Text(Song.formatTime(isDraggingSlider ? sliderValue : player.currentTime))
                    Spacer()
                    Text(Song.formatTime(player.duration))
                }
                .font(.caption2.monospacedDigit())
                .foregroundColor(WhisTheme.textSecondary)
            }

            HStack(spacing: 34) {
                Button {
                    player.playPrevious()
                } label: {
                    Image(systemName: "backward.fill")
                        .font(.system(size: 24))
                        .foregroundColor(.white)
                }

                Button {
                    player.togglePlayPause()
                } label: {
                    ZStack {
                        Circle()
                            .fill(WhisTheme.accentGradient)
                            .frame(width: 68, height: 68)
                            .shadow(color: WhisTheme.mint.opacity(0.35), radius: 16, y: 6)

                        if player.isLoadingStream {
                            ProgressView()
                                .tint(WhisTheme.background)
                        } else {
                            Image(systemName: player.isPlaying ? "pause.fill" : "play.fill")
                                .font(.system(size: 26, weight: .bold))
                                .foregroundColor(WhisTheme.background)
                                .offset(x: player.isPlaying ? 0 : 2)
                        }
                    }
                }

                Button {
                    player.playNext()
                } label: {
                    Image(systemName: "forward.fill")
                        .font(.system(size: 24))
                        .foregroundColor(.white)
                }
            }
            .padding(.top, 4)
        }
    }
}
