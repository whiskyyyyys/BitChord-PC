import http from "node:http";
import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { Readable } from "node:stream";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PUBLIC_DIR = path.join(__dirname, "public");
const PORT = Number(process.env.PORT || 3000);
const APP_VERSION = "2.0.10";

const MIME_TYPES = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".webmanifest": "application/manifest+json; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
};

const streamCache = new Map();
let quickPicksCache = { expiresAt: 0, songs: [] };

function sendJson(res, status, data) {
  const payload = JSON.stringify(data);
  res.writeHead(status, {
    "Content-Type": "application/json; charset=utf-8",
    "Cache-Control": "no-store",
    "Access-Control-Allow-Origin": "*",
  });
  res.end(payload);
}

// MARK: - YouTube Music Search (WEB_REMIX)

function parseDurationString(raw) {
  const parts = String(raw || "")
    .trim()
    .split(":")
    .map((p) => Number(p));
  if (parts.some((n) => !Number.isFinite(n))) return 0;
  if (parts.length === 2) return parts[0] * 60 + parts[1];
  if (parts.length === 3) return parts[0] * 3600 + parts[1] * 60 + parts[2];
  return 0;
}

function extractRuns(column) {
  const runs = column?.musicResponsiveListItemFlexColumnRenderer?.text?.runs;
  if (!Array.isArray(runs)) return [];
  return runs.map((r) => r?.text).filter((t) => typeof t === "string");
}

function upgradeArtwork(url) {
  if (!url) return "";
  return url
    .replace("w60-h60", "w544-h544")
    .replace("w120-h120", "w544-h544")
    .replace("w226-h226", "w544-h544");
}

function parseSongRenderer(renderer) {
  const videoId =
    renderer?.playlistItemData?.videoId ||
    renderer?.overlay?.musicItemThumbnailOverlayRenderer?.content?.musicPlayButtonRenderer
      ?.playNavigationEndpoint?.watchEndpoint?.videoId;

  if (!videoId || typeof videoId !== "string") return null;

  const flexColumns = Array.isArray(renderer.flexColumns) ? renderer.flexColumns : [];
  const titleRuns = extractRuns(flexColumns[0]);
  const title = titleRuns.join("").trim() || "Unknown Track";

  const subRuns = extractRuns(flexColumns[1])
    .map((s) => s.trim())
    .filter((s) => s && s !== "•" && s !== "·" && s !== ",");

  let artist = "Unknown Artist";
  let duration = 0;

  if (subRuns.length > 0) {
    const maybeDuration = parseDurationString(subRuns[subRuns.length - 1]);
    if (maybeDuration > 0) {
      duration = maybeDuration;
      const joined = subRuns.slice(0, -1).join(" ").trim();
      if (joined) artist = joined;
    } else {
      artist = subRuns[0];
    }
  }

  const thumbs = renderer?.thumbnail?.musicThumbnailRenderer?.thumbnail?.thumbnails;
  const rawThumb = Array.isArray(thumbs) && thumbs.length > 0 ? thumbs[thumbs.length - 1]?.url : "";

  return {
    id: videoId,
    title,
    artist,
    thumbnail: upgradeArtwork(rawThumb),
    duration,
  };
}

function collectSongs(node, results, seen) {
  if (!node || typeof node !== "object") return;
  if (Array.isArray(node)) {
    for (const item of node) collectSongs(item, results, seen);
    return;
  }
  if (node.musicResponsiveListItemRenderer) {
    const song = parseSongRenderer(node.musicResponsiveListItemRenderer);
    if (song && !seen.has(song.id)) {
      seen.add(song.id);
      results.push(song);
    }
  }
  for (const val of Object.values(node)) {
    if (val && typeof val === "object") {
      collectSongs(val, results, seen);
    }
  }
}

async function searchSongs(query) {
  const trimmed = String(query || "").trim();
  if (!trimmed) return [];

  const response = await fetch("https://music.youtube.com/youtubei/v1/search?prettyPrint=false", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Origin: "https://music.youtube.com",
      Referer: "https://music.youtube.com/",
      "User-Agent":
        "Mozilla/5.0 (iPhone; CPU iPhone OS 18_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.3 Mobile/15E148 Safari/604.1",
    },
    body: JSON.stringify({
      context: {
        client: {
          clientName: "WEB_REMIX",
          clientVersion: "1.20250310.01.00",
          hl: "id",
          gl: "ID",
        },
      },
      query: trimmed,
      params: "EgWKAQIIAWoMEAMQBBAJEAoQBRAV",
    }),
  });

  if (!response.ok) {
    throw new Error(`YouTube Music search HTTP ${response.status}`);
  }

  const json = await response.json();
  const results = [];
  const seen = new Set();
  collectSongs(json, results, seen);
  return results;
}

async function getQuickPicks() {
  if (Date.now() < quickPicksCache.expiresAt && quickPicksCache.songs.length > 0) {
    return quickPicksCache.songs;
  }

  const queries = ["Lagu Indonesia Populer Terbaru", "Global Top Hits 2025"];
  const settled = await Promise.allSettled(queries.map((q) => searchSongs(q)));
  const combined = [];
  const seen = new Set();

  for (const res of settled) {
    if (res.status === "fulfilled") {
      for (const song of res.value.slice(0, 10)) {
        if (!seen.has(song.id)) {
          seen.add(song.id);
          combined.push(song);
        }
      }
    }
  }

  if (combined.length > 0) {
    quickPicksCache = { expiresAt: Date.now() + 30 * 60_000, songs: combined };
  }
  return combined;
}

// MARK: - InnerTube Audio Stream Resolver (iOS Safari MP4/AAC friendly)

const INNERTUBE_CLIENTS = [
  {
    clientName: "VISIONOS",
    clientVersion: "1.02",
    headerId: "101",
    userAgent:
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15",
    extra: {
      deviceMake: "Apple",
      deviceModel: "RealityDevice17,1",
      osName: "visionOS",
      osVersion: "26.5.23O471",
    },
  },
  {
    clientName: "IOS",
    clientVersion: "20.10.4",
    headerId: "5",
    userAgent: "com.google.ios.youtube/20.10.4 (iPhone16,2; U; CPU iOS 18_3_2 like Mac OS X;)",
    extra: {
      deviceMake: "Apple",
      deviceModel: "iPhone16,2",
      osName: "iOS",
      osVersion: "18.3.2.22D82",
    },
  },
  {
    clientName: "ANDROID_VR",
    clientVersion: "1.61.48",
    headerId: "28",
    userAgent:
      "com.google.android.apps.youtube.vr.oculus/1.61.48 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip",
    extra: {
      deviceMake: "Oculus",
      deviceModel: "Quest 3",
      osName: "Android",
      osVersion: "12L",
      androidSdkVersion: 32,
    },
  },
];

async function fetchPlayerForClient(videoId, client) {
  const response = await fetch("https://www.youtube.com/youtubei/v1/player?prettyPrint=false", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "User-Agent": client.userAgent,
      "X-YouTube-Client-Name": client.headerId,
      "X-YouTube-Client-Version": client.clientVersion,
      Origin: "https://www.youtube.com",
    },
    body: JSON.stringify({
      context: {
        client: {
          clientName: client.clientName,
          clientVersion: client.clientVersion,
          hl: "en",
          gl: "US",
          ...client.extra,
        },
      },
      videoId,
      contentCheckOk: true,
      racyCheckOk: true,
    }),
  });

  if (!response.ok) {
    throw new Error(`${client.clientName} HTTP ${response.status}`);
  }

  const json = await response.json();
  const status = json?.playabilityStatus?.status;
  if (status && status !== "OK") {
    throw new Error(json?.playabilityStatus?.reason || status);
  }

  const adaptive = Array.isArray(json?.streamingData?.adaptiveFormats)
    ? json.streamingData.adaptiveFormats
    : [];
  const muxed = Array.isArray(json?.streamingData?.formats) ? json.streamingData.formats : [];

  for (const itag of [140, 139, 141]) {
    const match = adaptive.find((f) => Number(f?.itag) === itag && typeof f?.url === "string");
    if (match) {
      return {
        url: match.url,
        mimeType: match.mimeType || "audio/mp4",
        userAgent: client.userAgent,
      };
    }
  }

  const mp4Audio = adaptive
    .filter((f) => String(f?.mimeType || "").toLowerCase().includes("audio/mp4") && typeof f?.url === "string")
    .sort((a, b) => Number(b?.bitrate || 0) - Number(a?.bitrate || 0));

  if (mp4Audio.length > 0) {
    return {
      url: mp4Audio[0].url,
      mimeType: mp4Audio[0].mimeType || "audio/mp4",
      userAgent: client.userAgent,
    };
  }

  const mp4Muxed = muxed.find(
    (f) => String(f?.mimeType || "").toLowerCase().includes("mp4") && typeof f?.url === "string"
  );
  if (mp4Muxed) {
    return {
      url: mp4Muxed.url,
      mimeType: "audio/mp4",
      userAgent: client.userAgent,
    };
  }

  throw new Error(`${client.clientName}: no playable MP4/AAC url`);
}

async function resolveStream(videoId, forceRefresh = false) {
  if (!forceRefresh) {
    const cached = streamCache.get(videoId);
    if (cached && Date.now() < cached.expiresAt) {
      return cached;
    }
  }

  let lastError = null;
  for (const client of INNERTUBE_CLIENTS) {
    try {
      const resolved = await fetchPlayerForClient(videoId, client);
      const entry = { ...resolved, expiresAt: Date.now() + 20 * 60_000 };
      streamCache.set(videoId, entry);
      return entry;
    } catch (err) {
      lastError = err;
    }
  }
  throw lastError || new Error("Gagal mendapatkan stream audio");
}

async function handleStreamProxy(req, res, videoId) {
  if (!/^[a-zA-Z0-9_-]{6,20}$/.test(videoId)) {
    sendJson(res, 400, { error: "Invalid videoId" });
    return;
  }

  const rangeHeader = req.headers.range;
  let resolved;
  try {
    resolved = await resolveStream(videoId, false);
  } catch (err) {
    sendJson(res, 502, { error: err?.message || "Stream resolution failed" });
    return;
  }

  const forwardUpstream = async (target, method) => {
    const upstreamHeaders = {
      "User-Agent": target.userAgent,
    };
    if (rangeHeader) {
      upstreamHeaders.Range = rangeHeader;
    }
    return fetch(target.url, { method, headers: upstreamHeaders });
  };

  let upstream = await forwardUpstream(resolved, req.method === "HEAD" ? "HEAD" : "GET");
  if ((upstream.status === 403 || upstream.status === 404 || upstream.status === 410) && !req.headers["x-retried"]) {
    resolved = await resolveStream(videoId, true);
    upstream = await forwardUpstream(resolved, req.method === "HEAD" ? "HEAD" : "GET");
  }

  if (!upstream.ok && upstream.status !== 206) {
    sendJson(res, 502, { error: `Upstream audio returned HTTP ${upstream.status}` });
    return;
  }

  const outHeaders = {
    "Content-Type": (upstream.headers.get("content-type") || "audio/mp4").split(";")[0],
    "Accept-Ranges": upstream.headers.get("accept-ranges") || "bytes",
    "Cache-Control": "no-store",
    "Access-Control-Allow-Origin": "*",
  };
  const contentLength = upstream.headers.get("content-length");
  if (contentLength) outHeaders["Content-Length"] = contentLength;
  const contentRange = upstream.headers.get("content-range");
  if (contentRange) outHeaders["Content-Range"] = contentRange;

  res.writeHead(upstream.status, outHeaders);
  if (req.method === "HEAD" || !upstream.body) {
    res.end();
    return;
  }

  const nodeStream = Readable.fromWeb(upstream.body);
  req.on("close", () => {
    nodeStream.destroy();
  });
  nodeStream.on("error", () => {
    if (!res.writableEnded) res.end();
  });
  nodeStream.pipe(res);
}

// MARK: - LRCLIB Synced Lyrics

function cleanMetadata(raw) {
  return String(raw || "")
    .replace(/\(feat\.[^)]*\)/gi, "")
    .replace(/\(ft\.[^)]*\)/gi, "")
    .replace(/\[[^\]]*\]/g, "")
    .replace(/\(Official[^)]*\)/gi, "")
    .replace(/\(Lyric[^)]*\)/gi, "")
    .replace(/\(Audio[^)]*\)/gi, "")
    .trim();
}

function parseLrc(lrc) {
  const lines = [];
  const tsRegex = /\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]/g;
  for (const rawLine of String(lrc || "").split(/\r?\n/)) {
    const trimmed = rawLine.trim();
    if (!trimmed) continue;
    const matches = [...trimmed.matchAll(tsRegex)];
    if (matches.length === 0) continue;
    const last = matches[matches.length - 1];
    const text = trimmed.slice((last.index || 0) + last[0].length).trim();
    if (!text) continue;
    for (const m of matches) {
      const mins = Number(m[1] || 0);
      const secs = Number(m[2] || 0);
      const frac = String(m[3] || "");
      let ms = 0;
      if (frac.length === 1) ms = Number(frac) * 100;
      else if (frac.length === 2) ms = Number(frac) * 10;
      else if (frac.length >= 3) ms = Number(frac.slice(0, 3));
      lines.push({ timeMs: mins * 60_000 + secs * 1000 + ms, text });
    }
  }
  lines.sort((a, b) => a.timeMs - b.timeMs);
  return lines.map((line, idx) => ({ id: idx, ...line }));
}

function parseLrcRecord(record) {
  if (record?.syncedLyrics && record.syncedLyrics.trim()) {
    const synced = parseLrc(record.syncedLyrics);
    if (synced.length > 0) return synced;
  }
  if (record?.plainLyrics && record.plainLyrics.trim()) {
    return record.plainLyrics
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter(Boolean)
      .map((text, idx) => ({ id: idx, timeMs: -1, text }));
  }
  return [];
}

async function fetchLyrics(title, artist, duration) {
  const cTitle = cleanMetadata(title);
  const cArtist = cleanMetadata(artist);
  if (!cTitle) return [];

  const headers = { "User-Agent": `WhisMuno-PWA/${APP_VERSION}` };
  const exactUrl = new URL("https://lrclib.net/api/get");
  exactUrl.searchParams.set("track_name", cTitle);
  if (cArtist) exactUrl.searchParams.set("artist_name", cArtist);
  if (duration > 10) exactUrl.searchParams.set("duration", String(Math.round(duration)));

  try {
    const r1 = await fetch(exactUrl, { headers });
    if (r1.ok) {
      const parsed = parseLrcRecord(await r1.json());
      if (parsed.length > 0) return parsed;
    }
  } catch {}

  const searchUrl = new URL("https://lrclib.net/api/search");
  searchUrl.searchParams.set("q", `${cTitle} ${cArtist}`.trim());
  try {
    const r2 = await fetch(searchUrl, { headers });
    if (r2.ok) {
      const list = await r2.json();
      if (Array.isArray(list)) {
        for (const item of list) {
          if (item?.syncedLyrics) {
            const parsed = parseLrc(item.syncedLyrics);
            if (parsed.length > 0) return parsed;
          }
        }
        if (list.length > 0) {
          return parseLrcRecord(list[0]);
        }
      }
    }
  } catch {}

  return [];
}

// MARK: - Static File Server & Router

async function serveStatic(res, reqPath) {
  const safePath = path.normalize(reqPath === "/" ? "/index.html" : reqPath).replace(/^(\.\.[/\\])+/, "");
  let filePath = path.join(PUBLIC_DIR, safePath);
  try {
    const s = await stat(filePath);
    if (!s.isFile()) filePath = path.join(PUBLIC_DIR, "index.html");
  } catch {
    filePath = path.join(PUBLIC_DIR, "index.html");
  }

  const ext = path.extname(filePath).toLowerCase();
  const contentType = MIME_TYPES[ext] || "application/octet-stream";
  const content = await readFile(filePath);
  res.writeHead(200, {
    "Content-Type": contentType,
    "Cache-Control": ext === ".html" || ext === ".js" || ext === ".css" ? "no-cache" : "public, max-age=86400",
  });
  res.end(content);
}

const server = http.createServer(async (req, res) => {
  try {
    if (req.method === "OPTIONS") {
      res.writeHead(204, {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Methods": "GET, HEAD, OPTIONS",
        "Access-Control-Allow-Headers": "Range, Content-Type",
      });
      res.end();
      return;
    }

    const url = new URL(req.url || "/", `http://${req.headers.host || "localhost"}`);

    if (url.pathname === "/health") {
      sendJson(res, 200, { ok: true, version: APP_VERSION });
      return;
    }

    if (url.pathname === "/api/search") {
      const q = url.searchParams.get("q") || "";
      const songs = await searchSongs(q);
      sendJson(res, 200, { songs });
      return;
    }

    if (url.pathname === "/api/quick-picks") {
      const songs = await getQuickPicks();
      sendJson(res, 200, { songs });
      return;
    }

    if (url.pathname === "/api/lyrics") {
      const title = url.searchParams.get("title") || "";
      const artist = url.searchParams.get("artist") || "";
      const duration = Number(url.searchParams.get("duration") || 0);
      const lines = await fetchLyrics(title, artist, duration);
      sendJson(res, 200, { lines });
      return;
    }

    if (url.pathname === "/api/stream") {
      const videoId = url.searchParams.get("videoId") || "";
      await handleStreamProxy(req, res, videoId);
      return;
    }

    await serveStatic(res, url.pathname);
  } catch (err) {
    if (!res.headersSent) {
      sendJson(res, 500, { error: err?.message || "Internal server error" });
    } else if (!res.writableEnded) {
      res.end();
    }
  }
});

server.listen(PORT, "0.0.0.0", () => {
  console.log(`WhisMuno PWA server v${APP_VERSION} listening on http://0.0.0.0:${PORT}`);
});
