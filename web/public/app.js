const FAV_KEY = "whismuno_favs_v1";
const REC_KEY = "whismuno_recents_v1";
const INSTALL_DISMISSED_KEY = "whismuno_install_dismissed";

const state = {
  quickPicks: [],
  searchResults: [],
  favorites: loadStored(FAV_KEY),
  recents: loadStored(REC_KEY),
  queue: [],
  currentSong: null,
  lyrics: [],
  activeLyricIdx: -1,
  isPlaying: false,
  isLoadingStream: false,
  isDraggingSeek: false,
  libraryTab: "favorites",
};

const audio = document.getElementById("audio-el");

// DOM references
const livePillBtn = document.getElementById("live-pill-btn");
const livePillText = document.getElementById("live-pill-text");
const iosBanner = document.getElementById("ios-install-banner");
const dismissInstallBtn = document.getElementById("dismiss-install-btn");

const recentsSection = document.getElementById("recents-section");
const recentsRow = document.getElementById("recents-row");
const quickPicksList = document.getElementById("quick-picks-list");
const refreshPicksBtn = document.getElementById("refresh-picks-btn");

const searchForm = document.getElementById("search-form");
const searchInput = document.getElementById("search-input");
const clearSearchBtn = document.getElementById("clear-search-btn");
const searchResultsList = document.getElementById("search-results-list");

const libFavTab = document.getElementById("lib-fav-tab");
const libRecTab = document.getElementById("lib-rec-tab");
const libraryList = document.getElementById("library-list");

const miniPlayer = document.getElementById("mini-player");
const miniPlayerOpen = document.getElementById("mini-player-open");
const miniArt = document.getElementById("mini-art");
const miniTitle = document.getElementById("mini-title");
const miniSubtitle = document.getElementById("mini-subtitle");
const miniPlayBtn = document.getElementById("mini-play-btn");
const miniNextBtn = document.getElementById("mini-next-btn");

const playerSheet = document.getElementById("player-sheet");
const closeSheetBtn = document.getElementById("close-sheet-btn");
const playerAmbient = document.getElementById("player-ambient");
const sheetArt = document.getElementById("sheet-art");
const sheetTitle = document.getElementById("sheet-title");
const sheetArtist = document.getElementById("sheet-artist");
const sheetStatus = document.getElementById("sheet-status");
const sheetFavBtn = document.getElementById("sheet-fav-btn");

const modePlayerBtn = document.getElementById("mode-player-btn");
const modeLyricsBtn = document.getElementById("mode-lyrics-btn");
const panelArtwork = document.getElementById("panel-artwork");
const panelLyrics = document.getElementById("panel-lyrics");
const lyricsScroll = document.getElementById("lyrics-scroll");

const seekSlider = document.getElementById("seek-slider");
const timeElapsed = document.getElementById("time-elapsed");
const timeTotal = document.getElementById("time-total");
const prevBtn = document.getElementById("prev-btn");
const playPauseBtn = document.getElementById("play-pause-btn");
const nextBtn = document.getElementById("next-btn");

// MARK: - Helpers

function loadStored(key) {
  try {
    const raw = localStorage.getItem(key);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function saveStored(key, list) {
  try {
    localStorage.setItem(key, JSON.stringify(list));
  } catch {}
}

function formatTime(secs) {
  if (!Number.isFinite(secs) || secs <= 0) return "0:00";
  const total = Math.floor(secs);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

function isFavorite(id) {
  return state.favorites.some((s) => s.id === id);
}

function toggleFavorite(song) {
  if (!song) return;
  const idx = state.favorites.findIndex((s) => s.id === song.id);
  if (idx >= 0) state.favorites.splice(idx, 1);
  else state.favorites.unshift(song);
  saveStored(FAV_KEY, state.favorites);
  renderAllLists();
  updateSheetFav();
}

function addRecent(song) {
  state.recents = state.recents.filter((s) => s.id !== song.id);
  state.recents.unshift(song);
  if (state.recents.length > 40) state.recents = state.recents.slice(0, 40);
  saveStored(REC_KEY, state.recents);
}

function showSheetError(msg) {
  if (!msg) {
    sheetStatus.textContent = "";
    sheetStatus.classList.add("hidden");
  } else {
    sheetStatus.textContent = msg;
    sheetStatus.classList.remove("hidden");
  }
}

// MARK: - Rendering

function createSongRow(song, queue) {
  const row = document.createElement("div");
  row.className = "song-row" + (state.currentSong?.id === song.id ? " playing" : "");

  const img = document.createElement("img");
  img.src = song.thumbnail || "/icon.png";
  img.alt = song.title;
  img.loading = "lazy";

  const meta = document.createElement("div");
  meta.className = "song-row-meta";

  const title = document.createElement("div");
  title.className = "song-row-title";
  title.textContent = song.title;

  const artist = document.createElement("div");
  artist.className = "song-row-artist";
  artist.textContent = song.artist;

  meta.append(title, artist);

  const dur = document.createElement("span");
  dur.className = "song-row-dur";
  dur.textContent = song.duration > 0 ? formatTime(song.duration) : "";

  const fav = document.createElement("button");
  fav.type = "button";
  fav.className = "fav-btn" + (isFavorite(song.id) ? " active" : "");
  fav.textContent = isFavorite(song.id) ? "♥" : "♡";
  fav.addEventListener("click", (e) => {
    e.stopPropagation();
    toggleFavorite(song);
  });

  row.append(img, meta, dur, fav);
  row.addEventListener("click", () => playSong(song, queue));
  return row;
}

function renderSongList(container, songs, queue, emptyMsg) {
  container.innerHTML = "";
  if (!songs || songs.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.innerHTML = `<div class="empty-glyph">♪</div><p>${emptyMsg}</p>`;
    container.appendChild(empty);
    return;
  }
  const frag = document.createDocumentFragment();
  for (const song of songs) {
    frag.appendChild(createSongRow(song, queue));
  }
  container.appendChild(frag);
}

function renderRecents() {
  if (state.recents.length === 0) {
    recentsSection.classList.add("hidden");
    return;
  }
  recentsSection.classList.remove("hidden");
  recentsRow.innerHTML = "";
  const frag = document.createDocumentFragment();
  for (const song of state.recents.slice(0, 12)) {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "recent-card" + (state.currentSong?.id === song.id ? " playing" : "");
    card.innerHTML = `
      <img src="${song.thumbnail || "/icon.png"}" alt="" loading="lazy" />
      <div class="recent-title"></div>
      <div class="recent-artist"></div>
    `;
    card.querySelector(".recent-title").textContent = song.title;
    card.querySelector(".recent-artist").textContent = song.artist;
    card.addEventListener("click", () => playSong(song, state.recents));
    frag.appendChild(card);
  }
  recentsRow.appendChild(frag);
}

function renderLibrary() {
  libFavTab.textContent = `Favorit (${state.favorites.length})`;
  libRecTab.textContent = `Riwayat (${state.recents.length})`;
  const list = state.libraryTab === "favorites" ? state.favorites : state.recents;
  const emptyMsg =
    state.libraryTab === "favorites" ? "Belum ada lagu favorit." : "Belum ada riwayat putar.";
  renderSongList(libraryList, list, list, emptyMsg);
}

function renderAllLists() {
  renderRecents();
  renderSongList(
    quickPicksList,
    state.quickPicks,
    state.quickPicks,
    "Belum ada rekomendasi. Tekan Muat Ulang."
  );
  if (state.searchResults.length > 0) {
    renderSongList(searchResultsList, state.searchResults, state.searchResults, "");
  }
  renderLibrary();
}

function renderLyrics() {
  lyricsScroll.innerHTML = "";
  if (!state.lyrics || state.lyrics.length === 0) {
    lyricsScroll.innerHTML = `<div class="empty-state"><p>Lirik tidak ditemukan untuk lagu ini.</p></div>`;
    return;
  }
  const frag = document.createDocumentFragment();
  for (const line of state.lyrics) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "lyric-line" + (line.timeMs < 0 ? " unsynced" : "");
    btn.dataset.id = String(line.id);
    btn.textContent = line.text;
    if (line.timeMs >= 0) {
      btn.addEventListener("click", () => {
        if (Number.isFinite(audio.duration) && audio.duration > 0) {
          audio.currentTime = line.timeMs / 1000;
        }
      });
    }
    frag.appendChild(btn);
  }
  lyricsScroll.appendChild(frag);
}

function updateActiveLyric() {
  if (!state.lyrics || state.lyrics.length === 0) return;
  const posMs = (audio.currentTime + 0.12) * 1000;
  let activeIdx = -1;
  for (const line of state.lyrics) {
    if (line.timeMs >= 0 && line.timeMs <= posMs) activeIdx = line.id;
  }
  if (activeIdx === state.activeLyricIdx) return;
  state.activeLyricIdx = activeIdx;

  const lineObj = state.lyrics.find((l) => l.id === activeIdx);
  const pillText = lineObj?.text || state.currentSong?.title || "v2.0.10";
  livePillText.textContent = pillText;
  miniSubtitle.textContent = lineObj?.text || state.currentSong?.artist || "";

  const nodes = lyricsScroll.querySelectorAll(".lyric-line");
  nodes.forEach((node) => {
    const isMatch = Number(node.dataset.id) === activeIdx;
    node.classList.toggle("active", isMatch);
    if (isMatch && panelLyrics.classList.contains("active")) {
      node.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  });
}

function updateSheetFav() {
  const active = state.currentSong && isFavorite(state.currentSong.id);
  sheetFavBtn.textContent = active ? "♥" : "♡";
  sheetFavBtn.classList.toggle("active", Boolean(active));
}

function updatePlaybackUI() {
  const playGlyph = state.isLoadingStream ? "⏳" : state.isPlaying ? "❚❚" : "▶";
  miniPlayBtn.textContent = playGlyph;
  playPauseBtn.textContent = playGlyph;
  sheetArt.classList.toggle("paused", !state.isPlaying);
}

// MARK: - Playback & Media Session

async function playSong(song, queue) {
  if (Array.isArray(queue) && queue.length > 0) {
    state.queue = queue;
  } else if (!state.queue.some((s) => s.id === song.id)) {
    state.queue.unshift(song);
  }

  state.currentSong = song;
  state.lyrics = [];
  state.activeLyricIdx = -1;
  state.isLoadingStream = true;
  state.isPlaying = false;
  showSheetError("");
  addRecent(song);

  const cover = song.thumbnail || "/icon.png";
  miniPlayer.classList.remove("hidden");
  miniArt.src = cover;
  miniTitle.textContent = song.title;
  miniSubtitle.textContent = song.artist;
  livePillText.textContent = song.title;

  sheetArt.src = cover;
  playerAmbient.style.backgroundImage = `url("${cover}")`;
  sheetTitle.textContent = song.title;
  sheetArtist.textContent = song.artist;
  timeElapsed.textContent = "0:00";
  timeTotal.textContent = formatTime(song.duration);
  seekSlider.value = "0";
  seekSlider.max = String(Math.max(song.duration || 1, 1));

  updateSheetFav();
  updatePlaybackUI();
  renderAllLists();
  lyricsScroll.innerHTML = `<div class="empty-state"><p>Mencari lirik tersinkronisasi...</p></div>`;

  // Point <audio> directly at our range-capable server proxy inside click handler
  audio.src = `/api/stream?videoId=${encodeURIComponent(song.id)}`;
  audio.load();

  try {
    await audio.play();
    state.isPlaying = true;
    state.isLoadingStream = false;
    updateMediaSession(song);
  } catch (err) {
    state.isLoadingStream = false;
    state.isPlaying = false;
    if (err?.name !== "AbortError") {
      showSheetError("Audio gagal diputar di browser. Coba tap Play lagi.");
    }
  }
  updatePlaybackUI();

  // Fetch synced lyrics concurrently
  try {
    const res = await fetch(
      `/api/lyrics?title=${encodeURIComponent(song.title)}&artist=${encodeURIComponent(song.artist)}&duration=${encodeURIComponent(song.duration || 0)}`
    );
    if (res.ok && state.currentSong?.id === song.id) {
      const data = await res.json();
      state.lyrics = Array.isArray(data.lines) ? data.lines : [];
      renderLyrics();
    }
  } catch {}
}

function togglePlayPause() {
  if (!state.currentSong) return;
  if (audio.paused) {
    audio.play().catch(() => {});
  } else {
    audio.pause();
  }
}

function playNext() {
  if (!state.currentSong || state.queue.length === 0) return;
  const idx = state.queue.findIndex((s) => s.id === state.currentSong.id);
  const nextIdx = idx >= 0 ? (idx + 1) % state.queue.length : 0;
  playSong(state.queue[nextIdx], state.queue);
}

function playPrev() {
  if (audio.currentTime > 3) {
    audio.currentTime = 0;
    return;
  }
  if (!state.currentSong || state.queue.length === 0) return;
  const idx = state.queue.findIndex((s) => s.id === state.currentSong.id);
  const prevIdx = idx >= 0 ? (idx - 1 + state.queue.length) % state.queue.length : 0;
  playSong(state.queue[prevIdx], state.queue);
}

function updateMediaSession(song) {
  if (!("mediaSession" in navigator)) return;
  try {
    navigator.mediaSession.metadata = new MediaMetadata({
      title: song.title,
      artist: song.artist,
      album: "WhisMuno",
      artwork: [
        { src: song.thumbnail || "/icon.png", sizes: "544x544", type: "image/jpeg" },
      ],
    });
    navigator.mediaSession.setActionHandler("play", () => togglePlayPause());
    navigator.mediaSession.setActionHandler("pause", () => togglePlayPause());
    navigator.mediaSession.setActionHandler("nexttrack", () => playNext());
    navigator.mediaSession.setActionHandler("previoustrack", () => playPrev());
    navigator.mediaSession.setActionHandler("seekto", (details) => {
      if (typeof details.seekTime === "number") audio.currentTime = details.seekTime;
    });
  } catch {}
}

// MARK: - Audio Events

audio.addEventListener("playing", () => {
  state.isPlaying = true;
  state.isLoadingStream = false;
  showSheetError("");
  updatePlaybackUI();
});

audio.addEventListener("waiting", () => {
  state.isLoadingStream = true;
  updatePlaybackUI();
});

audio.addEventListener("pause", () => {
  state.isPlaying = false;
  state.isLoadingStream = false;
  updatePlaybackUI();
});

audio.addEventListener("ended", () => {
  playNext();
});

audio.addEventListener("error", () => {
  state.isPlaying = false;
  state.isLoadingStream = false;
  updatePlaybackUI();
  if (state.currentSong) {
    showSheetError("Stream gagal dimuat. Coba lagu lain atau tap Play ulang.");
  }
});

audio.addEventListener("timeupdate", () => {
  const dur = Number.isFinite(audio.duration) && audio.duration > 0
    ? audio.duration
    : state.currentSong?.duration || 0;
  if (!state.isDraggingSeek) {
    seekSlider.max = String(Math.max(dur, 1));
    seekSlider.value = String(Math.min(audio.currentTime, dur || audio.currentTime));
    timeElapsed.textContent = formatTime(audio.currentTime);
  }
  if (dur > 0) timeTotal.textContent = formatTime(dur);
  updateActiveLyric();
});

// MARK: - UI Event Wiring

document.querySelectorAll(".nav-btn").forEach((btn) => {
  btn.addEventListener("click", () => {
    const tab = btn.dataset.tab;
    document.querySelectorAll(".nav-btn").forEach((b) => b.classList.toggle("active", b === btn));
    document.querySelectorAll(".tab-view").forEach((v) => v.classList.remove("active"));
    document.getElementById(`view-${tab}`).classList.add("active");
    if (tab === "search") searchInput.focus();
  });
});

libFavTab.addEventListener("click", () => {
  state.libraryTab = "favorites";
  libFavTab.classList.add("active");
  libRecTab.classList.remove("active");
  renderLibrary();
});

libRecTab.addEventListener("click", () => {
  state.libraryTab = "recents";
  libRecTab.classList.add("active");
  libFavTab.classList.remove("active");
  renderLibrary();
});

searchInput.addEventListener("input", () => {
  clearSearchBtn.classList.toggle("hidden", !searchInput.value);
});

clearSearchBtn.addEventListener("click", () => {
  searchInput.value = "";
  clearSearchBtn.classList.add("hidden");
  state.searchResults = [];
  searchResultsList.innerHTML = `<div class="empty-state"><div class="empty-glyph">♫</div><p>Ketik judul lagu atau artis lalu tekan Cari</p></div>`;
});

searchForm.addEventListener("submit", async (e) => {
  e.preventDefault();
  const q = searchInput.value.trim();
  if (!q) return;
  searchResultsList.innerHTML = `<div class="empty-state"><p>Mencari di YouTube Music...</p></div>`;
  try {
    const res = await fetch(`/api/search?q=${encodeURIComponent(q)}`);
    const data = await res.json();
    state.searchResults = Array.isArray(data.songs) ? data.songs : [];
    renderSongList(searchResultsList, state.searchResults, state.searchResults, "Lagu tidak ditemukan.");
  } catch {
    searchResultsList.innerHTML = `<div class="empty-state"><p>Gagal menghubungi server pencarian.</p></div>`;
  }
});

function openSheet() {
  if (!state.currentSong) return;
  playerSheet.classList.remove("hidden");
  playerSheet.setAttribute("aria-hidden", "false");
}

function closeSheet() {
  playerSheet.classList.add("hidden");
  playerSheet.setAttribute("aria-hidden", "true");
}

miniPlayerOpen.addEventListener("click", openSheet);
livePillBtn.addEventListener("click", openSheet);
closeSheetBtn.addEventListener("click", closeSheet);

modePlayerBtn.addEventListener("click", () => {
  modePlayerBtn.classList.add("active");
  modeLyricsBtn.classList.remove("active");
  panelArtwork.classList.add("active");
  panelLyrics.classList.remove("active");
});

modeLyricsBtn.addEventListener("click", () => {
  modeLyricsBtn.classList.add("active");
  modePlayerBtn.classList.remove("active");
  panelLyrics.classList.add("active");
  panelArtwork.classList.remove("active");
  updateActiveLyric();
});

sheetFavBtn.addEventListener("click", () => toggleFavorite(state.currentSong));
miniPlayBtn.addEventListener("click", (e) => { e.stopPropagation(); togglePlayPause(); });
miniNextBtn.addEventListener("click", (e) => { e.stopPropagation(); playNext(); });
playPauseBtn.addEventListener("click", togglePlayPause);
nextBtn.addEventListener("click", playNext);
prevBtn.addEventListener("click", playPrev);

seekSlider.addEventListener("input", () => {
  state.isDraggingSeek = true;
  timeElapsed.textContent = formatTime(Number(seekSlider.value));
});

seekSlider.addEventListener("change", () => {
  audio.currentTime = Number(seekSlider.value);
  state.isDraggingSeek = false;
});

refreshPicksBtn.addEventListener("click", loadQuickPicks);

async function loadQuickPicks() {
  quickPicksList.innerHTML = `<div class="empty-state"><p>Memuat Pilihan Cepat...</p></div>`;
  try {
    const res = await fetch("/api/quick-picks");
    const data = await res.json();
    state.quickPicks = Array.isArray(data.songs) ? data.songs : [];
  } catch {
    state.quickPicks = [];
  }
  renderAllLists();
}

// Show iOS Safari Add to Home Screen banner when not standalone
function checkIosInstallBanner() {
  const isIos = /iPhone|iPad|iPod/i.test(navigator.userAgent);
  const isStandalone = window.navigator.standalone === true || window.matchMedia("(display-mode: standalone)").matches;
  const dismissed = localStorage.getItem(INSTALL_DISMISSED_KEY) === "1";
  if (isIos && !isStandalone && !dismissed) {
    iosBanner.classList.remove("hidden");
  }
}

dismissInstallBtn.addEventListener("click", () => {
  iosBanner.classList.add("hidden");
  localStorage.setItem(INSTALL_DISMISSED_KEY, "1");
});

if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("/sw.js").catch(() => {});
}

renderAllLists();
checkIosInstallBanner();
loadQuickPicks();
