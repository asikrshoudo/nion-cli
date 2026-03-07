#!/usr/bin/env node

const https = require("https");
const fs    = require("fs");
const path  = require("path");
const os    = require("os");
const cp    = require("child_process");

const REPO    = "asikrshoudo/nion-cli";
const VERSION = require("./package.json").version;

function getBinaryName() {
  const platform = os.platform();
  const arch     = os.arch();

  if (platform === "win32") return "nion-x86_64-windows.exe";

  if (platform === "darwin") {
    return arch === "arm64" ? "nion-aarch64-macos" : "nion-x86_64-macos";
  }

  // Linux and Android/Termux both come here
  if (platform === "linux" || platform === "android") {
    return arch === "arm64" || arch === "aarch64" || arch === "arm"
      ? "nion-aarch64-linux"
      : "nion-x86_64-linux";
  }

  console.error(`Unsupported platform: ${platform} ${arch}`);
  console.error(`Download manually: https://github.com/${REPO}/releases`);
  process.exit(1);
}

function download(url, dest, redirects = 0) {
  if (redirects > 10) throw new Error("Too many redirects");

  return new Promise((resolve, reject) => {
    https.get(url, { headers: { "User-Agent": "nion-cli-installer" } }, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        return resolve(download(res.headers.location, dest, redirects + 1));
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`HTTP ${res.statusCode} — ${url}`));
      }
      const file = fs.createWriteStream(dest);
      res.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", (e) => { fs.unlink(dest, () => {}); reject(e); });
    }).on("error", reject);
  });
}

// Find the best bin directory to install into
function getInstallDir() {
  const isWin = os.platform() === "win32";
  if (isWin) return null; // npm handles this on Windows

  // Termux: $PREFIX/bin
  const termuxBin = process.env.PREFIX
    ? path.join(process.env.PREFIX, "bin")
    : null;
  if (termuxBin && fs.existsSync(termuxBin)) return termuxBin;

  // Try npm global bin
  try {
    const npmBin = cp.execSync("npm bin -g 2>/dev/null || npm prefix -g", { encoding: "utf8" }).trim();
    const candidate = npmBin.includes("/bin") ? npmBin : path.join(npmBin, "bin");
    if (fs.existsSync(candidate)) return candidate;
  } catch (_) {}

  // Fallback: ~/.local/bin (same as curl installer)
  const localBin = path.join(os.homedir(), ".local", "bin");
  fs.mkdirSync(localBin, { recursive: true });
  return localBin;
}

async function main() {
  const binaryName = getBinaryName();
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${binaryName}`;
  const isWin = os.platform() === "win32";

  // Always put binary in npm package bin/ first (for npm linking)
  const pkgBinDir = path.join(__dirname, "bin");
  fs.mkdirSync(pkgBinDir, { recursive: true });
  const pkgBinPath = path.join(pkgBinDir, isWin ? "nion.exe" : "nion");

  console.log(`\nInstalling nion v${VERSION} (${binaryName})...`);

  try {
    await download(url, pkgBinPath);
  } catch (err) {
    console.error(`\nFailed to download nion.`);
    console.error(err.message);
    console.error(`\nInstall manually:`);
    console.error(`  curl -sSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash`);
    process.exit(1);
  }

  if (!isWin) fs.chmodSync(pkgBinPath, 0o755);

  // Also copy to system bin dir so it's in PATH without extra setup
  const installDir = getInstallDir();
  if (installDir) {
    const sysBinPath = path.join(installDir, "nion");
    try {
      fs.copyFileSync(pkgBinPath, sysBinPath);
      fs.chmodSync(sysBinPath, 0o755);
      console.log(`  Installed to ${sysBinPath}`);
    } catch (_) {
      // Not fatal — npm bin/ will still work if PATH includes npm bin
    }
  }

  console.log(`\n  nion installed successfully.`);
  console.log(`\n  Get started:`);
  console.log(`    nion config setup   <- add your API keys`);
  console.log(`    nion chat           <- start chatting`);
  console.log(`    nion agent          <- agentic mode\n`);
}

main();
