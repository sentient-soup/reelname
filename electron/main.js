const { app, Tray, Menu, shell, nativeImage } = require("electron");
const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");
const net = require("net");
const http = require("http");

// ── Constants ──────────────────────────────────────────

const APP_NAME = "ReelName";
const DEFAULT_PORT = 5267;
const HOSTNAME = "127.0.0.1";
const BROWSER_URL_BASE = "http://reelname.localhost";
const HEALTH_CHECK_INTERVAL = 500;
const HEALTH_CHECK_MAX_ATTEMPTS = 30;
const SHUTDOWN_TIMEOUT = 5000;

// ── Single instance lock ───────────────────────────────

const gotLock = app.requestSingleInstanceLock();
if (!gotLock) {
  app.quit();
}

// ── State ──────────────────────────────────────────────

let tray = null;
let serverProcess = null;
let serverPort = DEFAULT_PORT;
let serverRunning = false;

// Project root: electron/main.js lives one level inside the project
const PROJECT_ROOT = path.join(__dirname, "..");

// ── Platform paths ─────────────────────────────────────

function getConfigFilePath() {
  return path.join(app.getPath("userData"), "reelname-config.json");
}

function readConfig() {
  try {
    return JSON.parse(fs.readFileSync(getConfigFilePath(), "utf-8"));
  } catch {
    return {};
  }
}

function getDataDir() {
  const config = readConfig();
  if (config.data_dir) return config.data_dir;
  return path.join(app.getPath("userData"), "data");
}

function getNodeBinary() {
  if (app.isPackaged) {
    const ext = process.platform === "win32" ? ".exe" : "";
    return path.join(process.resourcesPath, "node", `node${ext}`);
  }
  // Dev: use system Node from PATH (NOT process.execPath which is electron.exe)
  return process.platform === "win32" ? "node.exe" : "node";
}

function getServerEntry() {
  if (app.isPackaged) {
    return path.join(process.resourcesPath, "standalone", "server.js");
  }
  return path.join(PROJECT_ROOT, ".next", "standalone", "server.js");
}

// ── Port checking ──────────────────────────────────────

function isPortAvailable(port) {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.once("error", () => resolve(false));
    server.once("listening", () => {
      server.close(() => resolve(true));
    });
    server.listen(port, HOSTNAME);
  });
}

async function findAvailablePort() {
  if (await isPortAvailable(DEFAULT_PORT)) return DEFAULT_PORT;
  // Scan nearby ports
  for (let p = DEFAULT_PORT + 1; p < DEFAULT_PORT + 100; p++) {
    if (await isPortAvailable(p)) return p;
  }
  throw new Error("No available port found");
}

// ── Health check ───────────────────────────────────────

function healthCheck(port) {
  return new Promise((resolve) => {
    const req = http.get(
      `http://${HOSTNAME}:${port}/api/settings`,
      (res) => {
        resolve(res.statusCode === 200);
        res.resume();
      }
    );
    req.on("error", () => resolve(false));
    req.setTimeout(2000, () => {
      req.destroy();
      resolve(false);
    });
  });
}

async function waitForServer(port) {
  for (let i = 0; i < HEALTH_CHECK_MAX_ATTEMPTS; i++) {
    if (await healthCheck(port)) return true;
    await new Promise((r) => setTimeout(r, HEALTH_CHECK_INTERVAL));
  }
  return false;
}

// ── Server management ──────────────────────────────────

async function startServer() {
  if (serverProcess) return;

  serverPort = await findAvailablePort();
  const dataDir = getDataDir();
  fs.mkdirSync(dataDir, { recursive: true });

  const nodeBin = getNodeBinary();
  const serverEntry = getServerEntry();

  console.log(`Starting server: ${nodeBin} ${serverEntry}`);
  console.log(`Port: ${serverPort}, Data dir: ${dataDir}`);

  serverProcess = spawn(nodeBin, [serverEntry], {
    env: {
      ...process.env,
      PORT: String(serverPort),
      HOSTNAME: HOSTNAME,
      REELNAME_DATA_DIR: dataDir,
      NODE_ENV: "production",
    },
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });

  serverProcess.stdout.on("data", (data) => {
    console.log(`[server] ${data.toString().trim()}`);
  });

  serverProcess.stderr.on("data", (data) => {
    console.error(`[server] ${data.toString().trim()}`);
  });

  serverProcess.on("exit", (code, signal) => {
    console.log(`Server exited: code=${code}, signal=${signal}`);
    serverProcess = null;
    serverRunning = false;
    updateTrayMenu();
    if (code !== 0 && code !== null) {
      console.error("Server stopped unexpectedly");
    }
  });

  updateTrayMenu("Starting...");

  const ready = await waitForServer(serverPort);
  if (ready) {
    serverRunning = true;
    console.log(`Server ready on port ${serverPort}`);
    updateTrayMenu();
  } else {
    console.error("Server failed to start within timeout");
    stopServer();
    updateTrayMenu("Failed to start");
  }

  return ready;
}

function stopServer() {
  if (!serverProcess) return;

  const proc = serverProcess;
  serverProcess = null;
  serverRunning = false;

  // Graceful shutdown
  if (process.platform === "win32") {
    // Windows: use taskkill for tree kill
    spawn("taskkill", ["/pid", String(proc.pid), "/T", "/F"], {
      windowsHide: true,
    });
  } else {
    proc.kill("SIGTERM");
    setTimeout(() => {
      try {
        proc.kill("SIGKILL");
      } catch {
        // Already dead
      }
    }, SHUTDOWN_TIMEOUT);
  }
}

async function restartServer() {
  updateTrayMenu("Restarting...");
  stopServer();
  // Brief pause to release port
  await new Promise((r) => setTimeout(r, 1000));
  const ready = await startServer();
  if (ready) openBrowser();
}

// ── Browser ────────────────────────────────────────────

function openBrowser() {
  const url = `${BROWSER_URL_BASE}:${serverPort}`;
  shell.openExternal(url);
}

// ── Tray ───────────────────────────────────────────────

function createFallbackIcon() {
  // 16x16 indigo square as a raw RGBA buffer -> PNG via nativeImage
  const size = 16;
  const buf = Buffer.alloc(size * size * 4);
  for (let i = 0; i < size * size; i++) {
    buf[i * 4] = 79;      // R (indigo #4F46E5)
    buf[i * 4 + 1] = 70;  // G
    buf[i * 4 + 2] = 229; // B
    buf[i * 4 + 3] = 255; // A
  }
  return nativeImage.createFromBuffer(buf, { width: size, height: size });
}

function createTray() {
  const iconPath = app.isPackaged
    ? path.join(process.resourcesPath, "icon.png")
    : path.join(PROJECT_ROOT, "build-resources", "icon.png");

  let icon;
  try {
    icon = nativeImage.createFromPath(iconPath);
    if (icon.isEmpty()) throw new Error("Empty image");
    icon = icon.resize({ width: 16, height: 16 });
  } catch (err) {
    console.warn(`Failed to load tray icon from ${iconPath}:`, err);
    icon = createFallbackIcon();
  }

  tray = new Tray(icon);
  tray.setToolTip(APP_NAME);

  tray.on("double-click", () => {
    if (serverRunning) openBrowser();
  });

  updateTrayMenu();
}

function updateTrayMenu(statusOverride) {
  if (!tray) return;

  let statusLabel;
  if (statusOverride) {
    statusLabel = statusOverride;
  } else if (serverRunning) {
    statusLabel = `Running on port ${serverPort}`;
  } else {
    statusLabel = "Stopped";
  }

  const menu = Menu.buildFromTemplate([
    { label: `${APP_NAME} — ${statusLabel}`, enabled: false },
    { type: "separator" },
    {
      label: "Open in Browser",
      enabled: serverRunning,
      click: openBrowser,
    },
    {
      label: "Restart Server",
      enabled: !statusOverride, // Disabled during transitions
      click: restartServer,
    },
    { type: "separator" },
    {
      label: "Quit",
      click: () => {
        stopServer();
        app.quit();
      },
    },
  ]);

  tray.setContextMenu(menu);
}

// ── App lifecycle ──────────────────────────────────────

// macOS: hide dock icon (tray-only app)
if (process.platform === "darwin") {
  app.dock.hide();
}

app.on("second-instance", () => {
  if (serverRunning) openBrowser();
});

app.whenReady().then(async () => {
  createTray();
  const ready = await startServer();
  if (ready) openBrowser();
});

app.on("window-all-closed", (e) => {
  // Prevent default quit — we're a tray app
  e.preventDefault?.();
});

app.on("before-quit", () => {
  stopServer();
});
