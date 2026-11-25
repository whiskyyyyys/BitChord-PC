const FAV_KEY = "whismuno_favs_v1";
const REC_KEY = "whismuno_recents_v1";
const OFFSET_KEY = "whismuno_lyric_offsets_v1";
const INSTALL_DISMISSED_KEY = "whismuno_install_dismissed";

const SVG_PLAY = `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5.5v13a1 1 0 001.5.85l10.2-6.5a1 1 0 000-1.7L9.5 4.65A1 1 0 008 5.5z"/></svg>`;
const SVG_PAUSE = `<svg viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="5" width="4" height="14" rx="1.2"/><rect x="14" y="5" width="4" height="14" rx="1.2"/></svg>`;
const SVG_SPIN = `<svg class="spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6"><circle cx="12" cy="12" r="8" stroke-opacity="0.25"/><path d="M12 4a8 8 0 018 8" stroke-linecap="round"/></svg>`;
const SVG_HEART_OUTLINE = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 20.4l-1.2-1.1C5.4 14.5 2 11.4 2 7.5A5.4 5.4 0 017.5 2c1.9 0 3.7.9 4.5 2.4C12.8 2.9 14.6 2 16.5 2A5.4 5.4 0 0122 7.5c0 3.9-3.4 7-8.8 11.8L12 20.4z"/></svg>`;
const SVG_HEART_FILL = `<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 20.8l-1.3-1.2C5.2 14.7 1.8 11.5 1.8 7.5A5.6 5.6 0 017.5 1.9c1.9 0 3.7.9 4.5 2.3.8-1.4 2.6-2.3 4.5-2.3a5.6 5.6 0 015.7 5.6c0 4-3.4 7.2-8.9 12.1L12 20.8z"/></svg>`;

const state = {
  mood: "all",
  quickPicks: [],
  searchResults: [],
  favorites: loadStored(FAV_KEY),
  recents: loadStored(REC_KEY),
  lyricOffsets: loadStoredObj(OFFSET_KEY),
  queue: [],
  currentSong: null,
  lyrics: [],
  lyricsSyncType: "none",
  lyricsProvider: "none",
  lyricsOffsetMs: 0,
  lyricsDurationResolved: false,
  wordTimings: [], // per-line array of { word, startMs, endMs }
  lyricDomCache: new Map(), // container -> { introEl, lines: Array<{ el, words, lastState }> }
  activeLyricIdx: -1,
  sheetMode: "player", // "player" | "lyrics" | "queue"
  isPlaying: false,
  isLoadingStream: false,
  isDraggingSeek: false,
  libraryTab: "favorites",
  rafId: 0,
};

const audio = document.getElementById("audio-el");

// DOM references
const livePillBtn = document.getElementById("live-pill-btn");
const livePillText = document.getElementById("live-pill-text");
const iosBanner = document.getElementById("ios-install-banner");
const dismissInstallBtn = document.getElementById("dismiss-install-btn");

const topPicksRow = document.getElementById("top-picks-row");
const moodGrid = document.getElementById("mood-grid");
const recentsSection = document.getElementById("recents-section");
const recentsPager = document.getElementById("recents-pager");
const quickPicksPager = document.getElementById("quick-picks-pager");
const refreshPicksBtn = document.getElementById("refresh-picks-btn");

const searchForm = document.getElementById("search-form");
const searchInput = document.getElementById("search-input");
const clearSearchBtn = document.getElementById("clear-search-btn");
const searchResultsList = document.getElementById("search-results-list");

const libFavTab = document.getElementById("lib-fav-tab");
const libRecTab = document.getElementById("lib-rec-tab");
const libraryList = document.getElementById("library-list");

const miniPlayer = document.getElementById("mini-player");
const miniProgressFill = document.getElementById("mini-progress-fill");
const miniPlayerOpen = document.getElementById("mini-player-open");
const miniArt = document.getElementById("mini-art");
const miniTitle = document.getElementById("mini-title");
const miniSubtitle = document.getElementById("mini-subtitle");
const miniLyricsBtn = document.getElementById("mini-lyrics-btn");
const miniPlayBtn = document.getElementById("mini-play-btn");
const miniNextBtn = document.getElementById("mini-next-btn");

const sideMeshBg = document.getElementById("side-mesh-bg");
const sideEqBars = document.getElementById("side-eq-bars");
const sideNowCard = document.getElementById("side-now-card");
const sideArt = document.getElementById("side-art");
const sideTitle = document.getElementById("side-title");
const sideArtist = document.getElementById("side-artist");
const sideLyricsScroll = document.getElementById("side-lyrics-scroll");
const sideFullscreenBtn = document.getElementById("side-fullscreen-btn");

const playerSheet = document.getElementById("player-sheet");
const meshBg = document.getElementById("mesh-bg");
const closeSheetBtn = document.getElementById("close-sheet-btn");
const sheetCompactHead = document.getElementById("sheet-compact-head");
const compactArt = document.getElementById("compact-art");
const compactTitle = document.getElementById("compact-title");
const compactArtist = document.getElementById("compact-artist");
const compactFavBtn = document.getElementById("compact-fav-btn");
const sheetFullMeta = document.getElementById("sheet-full-meta");

const sheetArt = document.getElementById("sheet-art");
const sheetTitle = document.getElementById("sheet-title");
const sheetArtist = document.getElementById("sheet-artist");
const sheetStatus = document.getElementById("sheet-status");
const sheetFavBtn = document.getElementById("sheet-fav-btn");

const modeLyricsBtn = document.getElementById("mode-lyrics-btn");
const modeQueueBtn = document.getElementById("mode-queue-btn");
const panelArtwork = document.getElementById("panel-artwork");
const panelLyrics = document.getElementById("panel-lyrics");
const panelQueue = document.getElementById("panel-queue");
const lyricsScroll = document.getElementById("lyrics-scroll");
const lyricsSyncBadge = document.getElementById("lyrics-sync-badge");
const syncMinusBtn = document.getElementById("sync-minus-btn");
const syncOffsetLabel = document.getElementById("sync-offset-label");
const syncPlusBtn = document.getElementById("sync-plus-btn");
const queueScroll = document.getElementById("queue-scroll");

const seekSlider = document.getElementById("seek-slider");
const timeElapsed = document.getElementById("time-elapsed");
const timeRemaining = document.getElementById("time-remaining");
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

function loadStoredObj(key) {
  try {
    const raw = localStorage.getItem(key);
    const parsed = raw ? JSON.parse(raw) : {};
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : {};
  } catch {
    return {};
  }
}

function saveStored(key, list) {
  try {
    localStorage.setItem(key, JSON.stringify(list));
  } catch {}
}

function loadSongOffset(videoId) {
  const val = Number(state.lyricOffsets?.[videoId] || 0);
  return Number.isFinite(val) ? val : 0;
}

function setSongOffset(ms) {
  const clamped = Math.max(-5000, Math.min(5000, ms));
  state.lyricsOffsetMs = clamped;
  if (state.currentSong?.id) {
    if (clamped === 0) delete state.lyricOffsets[state.currentSong.id];
    else state.lyricOffsets[state.currentSong.id] = clamped;
    saveStored(OFFSET_KEY, state.lyricOffsets);
  }
  updateLyricsToolbarUI();
  updateKaraokeFrame(true);
}

function updateLyricsToolbarUI() {
  if (lyricsSyncBadge) {
    const labelMap = {
      word: "Word Sync",
      line: "Line Sync",
      plain: "Plain Lyrics",
      none: "Lyrics",
    };
    const base = labelMap[state.lyricsSyncType] || "Lyrics";
    lyricsSyncBadge.textContent = state.lyricsProvider && state.lyricsProvider !== "none"
      ? `${base} · ${state.lyricsProvider}`
      : base;
  }
  if (syncOffsetLabel) {
    const secs = state.lyricsOffsetMs / 1000;
    const prefix = secs > 0 ? "+" : "";
    syncOffsetLabel.textContent = `${prefix}${secs.toFixed(2).replace(/\.00$/, ".0")}s`;
    syncOffsetLabel.classList.toggle("modified", state.lyricsOffsetMs !== 0);
  }
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

function updateMeshBackdrop(coverUrl) {
  const safeUrl = `url("${coverUrl || "/icon.png"}")`;
  for (const bg of [meshBg, sideMeshBg]) {
    if (!bg) continue;
    bg.querySelectorAll(".mesh-layer-static").forEach((layer) => {
      layer.style.opacity = "0";
      layer.style.backgroundImage = safeUrl;
      requestAnimationFrame(() => {
        layer.style.opacity = "0.78";
      });
    });
  }
}

// MARK: - Rendering Android-Style Shelves & Rows

function createTrackRow(song, queue) {
  const row = document.createElement("div");
  row.className = "track-row" + (state.currentSong?.id === song.id ? " playing" : "");

  const thumbBox = document.createElement("div");
  thumbBox.className = "track-thumb-box";
  thumbBox.innerHTML = `
    <img src="${song.thumbnail || "/icon.png"}" alt="" loading="lazy" />
    <div class="track-playing-overlay">
      <span class="eq-bars ${state.isPlaying ? "playing" : ""}"><span></span><span></span><span></span></span>
    </div>
  `;

  const meta = document.createElement("div");
  meta.className = "track-meta";
  const title = document.createElement("div");
  title.className = "track-title";
  title.textContent = song.title;
  const artist = document.createElement("div");
  artist.className = "track-artist";
  artist.textContent = song.artist;
  meta.append(title, artist);

  const dur = document.createElement("span");
  dur.className = "track-dur";
  dur.textContent = song.duration > 0 ? formatTime(song.duration) : "";

  const fav = document.createElement("button");
  fav.type = "button";
  const isFav = isFavorite(song.id);
  fav.className = "fav-btn" + (isFav ? " active" : "");
  fav.innerHTML = isFav ? SVG_HEART_FILL : SVG_HEART_OUTLINE;
  fav.addEventListener("click", (e) => {
    e.stopPropagation();
    toggleFavorite(song);
  });

  row.append(thumbBox, meta, dur, fav);
  row.addEventListener("click", () => playSong(song, queue));
  return row;
}

function renderTopPicks() {
  topPicksRow.innerHTML = "";
  const picks = state.quickPicks.slice(0, 6);
  if (picks.length === 0) {
    topPicksRow.innerHTML = `<div class="empty-state"><p>Memuat Top Picks...</p></div>`;
    return;
  }
  const frag = document.createDocumentFragment();
  picks.forEach((song, idx) => {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "hero-card";
    card.innerHTML = `
      <img src="${song.thumbnail || "/icon.png"}" alt="" loading="lazy" />
      <div class="hero-card-scrim"></div>
      <span class="hero-card-tag">${idx === 0 ? "Top Pick" : "For You"}</span>
      <div class="hero-card-copy">
        <div class="hero-card-title"></div>
        <div class="hero-card-artist"></div>
      </div>
    `;
    card.querySelector(".hero-card-title").textContent = song.title;
    card.querySelector(".hero-card-artist").textContent = song.artist;
    card.addEventListener("click", () => playSong(song, state.quickPicks));
    frag.appendChild(card);
  });
  topPicksRow.appendChild(frag);
}

// 4-rows-per-column horizontal pager (matching HomeScreen.kt RecentShelf)
function renderPagedTrackShelf(container, songs, queue, emptyMsg) {
  container.innerHTML = "";
  if (!songs || songs.length === 0) {
    container.innerHTML = `<div class="empty-state"><p>${emptyMsg}</p></div>`;
    return;
  }
  const frag = document.createDocumentFragment();
  for (let i = 0; i < songs.length; i += 4) {
    const col = document.createElement("div");
    col.className = "track-column";
    for (const song of songs.slice(i, i + 4)) {
      col.appendChild(createTrackRow(song, queue));
    }
    frag.appendChild(col);
  }
  container.appendChild(frag);
}

function renderFlatTrackList(container, songs, queue, emptyMsg) {
  container.innerHTML = "";
  if (!songs || songs.length === 0) {
    container.innerHTML = `<div class="empty-state"><div class="empty-glyph">♪</div><p>${emptyMsg}</p></div>`;
    return;
  }
  const frag = document.createDocumentFragment();
  for (const song of songs) {
    frag.appendChild(createTrackRow(song, queue));
  }
  container.appendChild(frag);
}

function renderLibrary() {
  libFavTab.textContent = `Favorites (${state.favorites.length})`;
  libRecTab.textContent = `History (${state.recents.length})`;
  const list = state.libraryTab === "favorites" ? state.favorites : state.recents;
  const emptyMsg =
    state.libraryTab === "favorites" ? "Belum ada lagu favorit." : "Belum ada riwayat putar.";
  renderFlatTrackList(libraryList, list, list, emptyMsg);
}

function renderAllLists() {
  renderTopPicks();
  if (state.recents.length > 0) {
    recentsSection.classList.remove("hidden");
    renderPagedTrackShelf(recentsPager, state.recents.slice(0, 16), state.recents, "");
  } else {
    recentsSection.classList.add("hidden");
  }
  renderPagedTrackShelf(
    quickPicksPager,
    state.quickPicks,
    state.quickPicks,
    "Belum ada rekomendasi. Tekan Muat Ulang."
  );
  if (state.searchResults.length > 0) {
    renderFlatTrackList(searchResultsList, state.searchResults, state.searchResults, "");
  }
  renderLibrary();
  renderFlatTrackList(queueScroll, state.queue, state.queue, "Antrean masih kosong.");
}

// MARK: - 60fps Word-by-Word Karaoke Lyrics (Zero-DOM-Query Cache + Word Sync Support)

function computeWordTimings(lines) {
  return lines.map((line, i) => {
    if (line.timeMs < 0) return [];
    if (Array.isArray(line.words) && line.words.length > 0) {
      return line.words.map((w) => ({
        word: w.word,
        startMs: Number(w.startMs || line.timeMs),
        endMs: Math.max(Number(w.startMs || line.timeMs) + 80, Number(w.endMs || w.startMs + 300)),
      }));
    }
    const words = String(line.text || "").split(/\s+/).filter(Boolean);
    if (words.length === 0) return [];
    const startMs = line.timeMs;
    const next = lines[i + 1];
    let spanMs = Math.max(950, words.length * 360);
    if (next && next.timeMs > startMs) {
      spanMs = next.timeMs - startMs;
    } else if (line.endMs && line.endMs > startMs) {
      spanMs = line.endMs - startMs;
    }
    // Cap synthetic spread so short phrases don't crawl across long instrumental gaps
    const maxRealisticMs = Math.max(950, words.length * 420);
    const usableMs = Math.max(Math.min(spanMs * 0.9, maxRealisticMs), 480);
    const totalChars = words.reduce((acc, w) => acc + Math.max(w.length, 2), 0);
    let cursor = startMs;
    return words.map((w) => {
      const weight = Math.max(w.length, 2) / totalChars;
      const dur = usableMs * weight;
      const entry = { word: w, startMs: cursor, endMs: cursor + dur };
      cursor += dur;
      return entry;
    });
  });
}

function buildLyricsIntoContainer(container) {
  container.innerHTML = "";
  const frag = document.createDocumentFragment();
  let introEl = null;

  if (state.lyrics.length > 0 && state.lyrics[0].timeMs > 1500) {
    introEl = document.createElement("div");
    introEl.className = "lyric-dots";
    introEl.dataset.intro = "1";
    introEl.innerHTML = "<span></span><span></span><span></span>";
    frag.appendChild(introEl);
  }

  const lineEntries = state.lyrics.map((line, i) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "lyric-line" + (line.timeMs < 0 ? " unsynced" : "");
    btn.dataset.idx = String(i);

    const wTimings = state.wordTimings[i] || [];
    const wordEls = [];
    if (wTimings.length > 0) {
      wTimings.forEach((wt, wi) => {
        const span = document.createElement("span");
        span.className = "lword";
        span.dataset.w = String(wi);
        span.textContent = wt.word;
        btn.appendChild(span);
        wordEls.push(span);
      });
    } else {
      const span = document.createElement("span");
      span.className = "lword";
      span.textContent = line.text;
      btn.appendChild(span);
      wordEls.push(span);
    }

    if (line.timeMs >= 0) {
      btn.addEventListener("click", () => {
        if (Number.isFinite(audio.duration) && audio.duration > 0) {
          audio.currentTime = Math.max(0, (line.timeMs - state.lyricsOffsetMs) / 1000);
          updateKaraokeFrame(true);
        }
      });
    }
    frag.appendChild(btn);
    return {
      idx: i,
      el: btn,
      words: wordEls,
      lastPcts: new Array(wordEls.length).fill(-2),
    };
  });

  container.appendChild(frag);
  state.lyricDomCache.set(container, { introEl, lines: lineEntries });
}

function resetLineWords(entry) {
  if (!entry) return;
  for (let wi = 0; wi < entry.words.length; wi++) {
    if (entry.lastPcts[wi] !== -1) {
      entry.words[wi].className = "lword";
      entry.words[wi].style.removeProperty("--p");
      entry.lastPcts[wi] = -1;
    }
  }
}

function renderLyrics() {
  state.wordTimings = computeWordTimings(state.lyrics);
  state.lyricDomCache.clear();
  updateLyricsToolbarUI();

  const targets = [lyricsScroll, sideLyricsScroll];
  if (!state.lyrics || state.lyrics.length === 0) {
    for (const el of targets) {
      el.innerHTML = `<div class="empty-state"><p>Lirik tidak ditemukan untuk lagu ini.</p></div>`;
    }
    return;
  }
  for (const el of targets) {
    buildLyricsIntoContainer(el);
  }
  updateKaraokeFrame(true);
}

let userScrollTimer = 0;
function markUserScrolling(el) {
  el.classList.add("user-scrolling");
  clearTimeout(userScrollTimer);
  userScrollTimer = setTimeout(() => {
    el.classList.remove("user-scrolling");
  }, 2400);
}
lyricsScroll.addEventListener("touchstart", () => markUserScrolling(lyricsScroll), { passive: true });
lyricsScroll.addEventListener("wheel", () => markUserScrolling(lyricsScroll), { passive: true });

function updateKaraokeFrame(forceScroll = false) {
  if (!state.lyrics || state.lyrics.length === 0) return;
  const posMs = audio.currentTime * 1000 + state.lyricsOffsetMs;

  let activeIdx = -1;
  for (let i = 0; i < state.lyrics.length; i++) {
    const t = state.lyrics[i].timeMs;
    if (t >= 0 && t <= posMs) activeIdx = i;
  }

  const prevIdx = state.activeLyricIdx;
  const lineChanged = forceScroll || activeIdx !== prevIdx;
  if (lineChanged) {
    state.activeLyricIdx = activeIdx;
    const lineObj = activeIdx >= 0 ? state.lyrics[activeIdx] : null;
    if (lineObj?.text) {
      livePillText.textContent = lineObj.text;
      miniSubtitle.textContent = lineObj.text;
    } else if (state.currentSong) {
      livePillText.textContent = state.currentSong.title;
      miniSubtitle.textContent = state.currentSong.artist;
    }
  }

  const showSide = window.innerWidth >= 880;
  const showSheetLyrics =
    !playerSheet.classList.contains("hidden") && state.sheetMode === "lyrics";

  for (const container of [lyricsScroll, sideLyricsScroll]) {
    if (container === sideLyricsScroll && !showSide) continue;
    if (container === lyricsScroll && !showSheetLyrics) continue;

    const cached = state.lyricDomCache.get(container);
    if (!cached) continue;

    if (cached.introEl && lineChanged) {
      cached.introEl.style.display = activeIdx < 0 ? "inline-flex" : "none";
    }

    if (lineChanged) {
      if (prevIdx >= 0 && prevIdx !== activeIdx) {
        resetLineWords(cached.lines[prevIdx]);
      }
      for (let i = 0; i < cached.lines.length; i++) {
        const entry = cached.lines[i];
        const node = entry.el;
        const isActive = i === activeIdx;
        const isPast = activeIdx >= 0 && i < activeIdx;
        node.classList.toggle("active", isActive);
        node.classList.toggle("past", isPast);
        node.classList.remove("dist-1", "dist-2", "dist-far");
        if (activeIdx >= 0 && !isActive) {
          const dist = Math.abs(i - activeIdx);
          if (dist === 1) node.classList.add("dist-1");
          else if (dist === 2) node.classList.add("dist-2");
          else node.classList.add("dist-far");
        }
        if (isActive && !container.classList.contains("user-scrolling")) {
          node.scrollIntoView({ behavior: "smooth", block: "center" });
        }
      }
    }

    // Per-word sweep ONLY on the single active line, quantized to 1% steps
    if (activeIdx >= 0 && cached.lines[activeIdx]) {
      const entry = cached.lines[activeIdx];
      const wTimings = state.wordTimings[activeIdx] || [];
      for (let wi = 0; wi < entry.words.length; wi++) {
        const wt = wTimings[wi];
        const wEl = entry.words[wi];
        if (!wt) continue;
        if (posMs >= wt.endMs) {
          if (entry.lastPcts[wi] !== 101) {
            wEl.className = "lword sung";
            wEl.style.removeProperty("--p");
            entry.lastPcts[wi] = 101;
          }
        } else if (posMs >= wt.startMs) {
          const pct = Math.min(100, Math.max(0, Math.round(((posMs - wt.startMs) / Math.max(wt.endMs - wt.startMs, 1)) * 100)));
          if (entry.lastPcts[wi] !== pct) {
            if (entry.lastPcts[wi] < 0 || entry.lastPcts[wi] > 100) {
              wEl.className = "lword singing";
            }
            wEl.style.setProperty("--p", `${pct}%`);
            entry.lastPcts[wi] = pct;
          }
        } else if (entry.lastPcts[wi] !== -1) {
          wEl.className = "lword";
          wEl.style.removeProperty("--p");
          entry.lastPcts[wi] = -1;
        }
      }
    }
  }
}

function startKaraokeLoop() {
  cancelAnimationFrame(state.rafId);
  const step = () => {
    updateKaraokeFrame(false);
    if (!audio.paused && !audio.ended) {
      state.rafId = requestAnimationFrame(step);
    }
  };
  state.rafId = requestAnimationFrame(step);
}

function updateSheetFav() {
  const active = state.currentSong && isFavorite(state.currentSong.id);
  const icon = active ? SVG_HEART_FILL : SVG_HEART_OUTLINE;
  for (const btn of [sheetFavBtn, compactFavBtn]) {
    btn.innerHTML = icon;
    btn.classList.toggle("active", Boolean(active));
  }
}

function updatePlaybackUI() {
  const iconHtml = state.isLoadingStream ? SVG_SPIN : state.isPlaying ? SVG_PAUSE : SVG_PLAY;
  miniPlayBtn.innerHTML = iconHtml;
  playPauseBtn.innerHTML = iconHtml;
  sheetArt.classList.toggle("paused", !state.isPlaying);
  if (sideEqBars) sideEqBars.classList.toggle("playing", state.isPlaying);
  document.querySelectorAll(".track-row.playing .eq-bars").forEach((el) => {
    el.classList.toggle("playing", state.isPlaying);
  });
}

// MARK: - Playback & Media Session

async function loadLyricsForSong(song, resolvedDuration = 0) {
  const dur = resolvedDuration > 10 ? Math.round(resolvedDuration) : (song.duration || 0);
  try {
    const res = await fetch(
      `/api/lyrics?videoId=${encodeURIComponent(song.id)}&title=${encodeURIComponent(song.title)}&artist=${encodeURIComponent(song.artist)}&duration=${encodeURIComponent(dur)}`
    );
    if (res.ok && state.currentSong?.id === song.id) {
      const data = await res.json();
      state.lyrics = Array.isArray(data.lines) ? data.lines : [];
      state.lyricsSyncType = data.syncType || "none";
      state.lyricsProvider = data.provider || "none";
      state.lyricsDurationResolved = dur > 10;
      renderLyrics();
    }
  } catch {
    if (state.currentSong?.id === song.id) {
      state.lyrics = [];
      state.lyricsSyncType = "none";
      state.lyricsProvider = "none";
      renderLyrics();
    }
  }
}

async function playSong(song, queue) {
  if (Array.isArray(queue) && queue.length > 0) {
    state.queue = queue;
  } else if (!state.queue.some((s) => s.id === song.id)) {
    state.queue.unshift(song);
  }

  state.currentSong = song;
  state.lyrics = [];
  state.lyricsSyncType = "none";
  state.lyricsProvider = "none";
  state.lyricsOffsetMs = loadSongOffset(song.id);
  state.lyricsDurationResolved = (song.duration || 0) > 10;
  state.wordTimings = [];
  state.lyricDomCache.clear();
  state.activeLyricIdx = -1;
  updateLyricsToolbarUI();
  state.isLoadingStream = true;
  state.isPlaying = false;
  showSheetError("");
  addRecent(song);

  const cover = song.thumbnail || "/icon.png";
  updateMeshBackdrop(cover);

  livePillBtn.classList.remove("hidden");
  livePillText.textContent = song.title;

  miniPlayer.classList.remove("hidden");
  miniProgressFill.style.width = "0%";
  miniArt.src = cover;
  miniTitle.textContent = song.title;
  miniSubtitle.textContent = song.artist;

  sideNowCard.classList.remove("hidden");
  sideArt.src = cover;
  sideTitle.textContent = song.title;
  sideArtist.textContent = song.artist;

  sheetArt.src = cover;
  compactArt.src = cover;
  sheetTitle.textContent = song.title;
  sheetArtist.textContent = song.artist;
  compactTitle.textContent = song.title;
  compactArtist.textContent = song.artist;
  timeElapsed.textContent = "0:00";
  timeRemaining.textContent = song.duration > 0 ? `-${formatTime(song.duration)}` : "-0:00";
  seekSlider.value = "0";
  seekSlider.max = String(Math.max(song.duration || 1, 1));

  updateSheetFav();
  updatePlaybackUI();
  renderAllLists();

  const loadingHtml = `<div class="empty-state"><p>Mencari lirik tersinkronisasi...</p></div>`;
  lyricsScroll.innerHTML = loadingHtml;
  sideLyricsScroll.innerHTML = loadingHtml;

  loadLyricsForSong(song);

  audio.src = `/api/stream?videoId=${encodeURIComponent(song.id)}`;
  audio.load();

  try {
    await audio.play();
    state.isPlaying = true;
    state.isLoadingStream = false;
    updateMediaSession(song);
    startKaraokeLoop();
  } catch (err) {
    state.isLoadingStream = false;
    state.isPlaying = false;
    if (err?.name !== "AbortError") {
      showSheetError("Audio gagal dimuat di browser. Coba tap Play lagi.");
    }
  }
  updatePlaybackUI();
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
    updateKaraokeFrame(true);
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
      if (typeof details.seekTime === "number") {
        audio.currentTime = details.seekTime;
        updateKaraokeFrame(true);
      }
    });
  } catch {}
}

// MARK: - Audio Events

audio.addEventListener("playing", () => {
  state.isPlaying = true;
  state.isLoadingStream = false;
  showSheetError("");
  updatePlaybackUI();
  startKaraokeLoop();
});

audio.addEventListener("waiting", () => {
  state.isLoadingStream = true;
  updatePlaybackUI();
});

audio.addEventListener("pause", () => {
  state.isPlaying = false;
  state.isLoadingStream = false;
  cancelAnimationFrame(state.rafId);
  updatePlaybackUI();
});

audio.addEventListener("ended", () => {
  cancelAnimationFrame(state.rafId);
  playNext();
});

audio.addEventListener("error", () => {
  state.isPlaying = false;
  state.isLoadingStream = false;
  cancelAnimationFrame(state.rafId);
  updatePlaybackUI();
  if (state.currentSong) {
    showSheetError("Stream gagal dimuat. Coba tap Play ulang.");
  }
});

audio.addEventListener("loadedmetadata", () => {
  if (
    state.currentSong &&
    !state.lyricsDurationResolved &&
    Number.isFinite(audio.duration) &&
    audio.duration > 10
  ) {
    state.lyricsDurationResolved = true;
    loadLyricsForSong(state.currentSong, audio.duration);
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
  if (dur > 0) {
    const rem = Math.max(0, dur - audio.currentTime);
    timeRemaining.textContent = `-${formatTime(rem)}`;
    const pct = Math.max(0, Math.min(100, (audio.currentTime / dur) * 100));
    miniProgressFill.style.width = `${pct.toFixed(2)}%`;
  }
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

moodGrid.querySelectorAll(".mood-tile").forEach((tile) => {
  tile.addEventListener("click", () => {
    const mood = tile.dataset.mood || "all";
    state.mood = mood;
    moodGrid.querySelectorAll(".mood-tile").forEach((c) => c.classList.toggle("active", c === tile));
    loadQuickPicks(mood);
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
  searchResultsList.innerHTML = `<div class="empty-state"><div class="empty-glyph">♫</div><p>Cari lagu, artis, atau penggalan lirik di YouTube Music</p></div>`;
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
    renderFlatTrackList(searchResultsList, state.searchResults, state.searchResults, "Lagu tidak ditemukan.");
  } catch {
    searchResultsList.innerHTML = `<div class="empty-state"><p>Gagal menghubungi server pencarian.</p></div>`;
  }
});

function setSheetMode(mode) {
  state.sheetMode = mode;
  const isPlayer = mode === "player";
  panelArtwork.classList.toggle("active", isPlayer);
  panelLyrics.classList.toggle("active", mode === "lyrics");
  panelQueue.classList.toggle("active", mode === "queue");
  sheetFullMeta.classList.toggle("hidden", !isPlayer);
  sheetCompactHead.classList.toggle("hidden", isPlayer);
  modeLyricsBtn.classList.toggle("active", mode === "lyrics");
  modeQueueBtn.classList.toggle("active", mode === "queue");
  if (mode === "lyrics") {
    requestAnimationFrame(() => updateKaraokeFrame(true));
  }
}

function openSheet(mode = "player") {
  if (!state.currentSong) return;
  setSheetMode(mode);
  playerSheet.classList.remove("hidden");
  playerSheet.setAttribute("aria-hidden", "false");
  if (mode === "lyrics") {
    requestAnimationFrame(() => updateKaraokeFrame(true));
  }
}

function closeSheet() {
  playerSheet.classList.add("hidden");
  playerSheet.setAttribute("aria-hidden", "true");
}

miniPlayerOpen.addEventListener("click", () => openSheet("player"));
miniLyricsBtn.addEventListener("click", (e) => { e.stopPropagation(); openSheet("lyrics"); });
livePillBtn.addEventListener("click", () => openSheet("lyrics"));
sideFullscreenBtn.addEventListener("click", () => openSheet("lyrics"));
closeSheetBtn.addEventListener("click", closeSheet);

modeLyricsBtn.addEventListener("click", () => {
  setSheetMode(state.sheetMode === "lyrics" ? "player" : "lyrics");
});
modeQueueBtn.addEventListener("click", () => {
  setSheetMode(state.sheetMode === "queue" ? "player" : "queue");
});

sheetFavBtn.addEventListener("click", () => toggleFavorite(state.currentSong));
compactFavBtn.addEventListener("click", () => toggleFavorite(state.currentSong));
miniPlayBtn.addEventListener("click", (e) => { e.stopPropagation(); togglePlayPause(); });
miniNextBtn.addEventListener("click", (e) => { e.stopPropagation(); playNext(); });
playPauseBtn.addEventListener("click", togglePlayPause);
nextBtn.addEventListener("click", playNext);
prevBtn.addEventListener("click", playPrev);

syncMinusBtn?.addEventListener("click", () => setSongOffset(state.lyricsOffsetMs - 250));
syncPlusBtn?.addEventListener("click", () => setSongOffset(state.lyricsOffsetMs + 250));
syncOffsetLabel?.addEventListener("click", () => setSongOffset(0));

seekSlider.addEventListener("input", () => {
  state.isDraggingSeek = true;
  timeElapsed.textContent = formatTime(Number(seekSlider.value));
});

seekSlider.addEventListener("change", () => {
  audio.currentTime = Number(seekSlider.value);
  state.isDraggingSeek = false;
  updateKaraokeFrame(true);
});

refreshPicksBtn.addEventListener("click", () => loadQuickPicks(state.mood));

async function loadQuickPicks(mood = state.mood) {
  quickPicksPager.innerHTML = `<div class="empty-state"><p>Memuat Quick Picks...</p></div>`;
  try {
    const res = await fetch(`/api/quick-picks?mood=${encodeURIComponent(mood)}`);
    const data = await res.json();
    state.quickPicks = Array.isArray(data.songs) ? data.songs : [];
  } catch {
    state.quickPicks = [];
  }
  renderAllLists();
}

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

updatePlaybackUI();
updateSheetFav();
renderAllLists();
checkIosInstallBanner();
loadQuickPicks("all");
