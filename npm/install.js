#!/usr/bin/env node

const https = require("https");
const fs    = require("fs");
const path  = require("path");
const os    = require("os");

const REPO    = "asikrshoudo/nion-cli";
const VERSION = require("./package.json").version;

// Map Node's os.platform() + os.arch() to the binary name in GitHub releases
function getBinaryName() {
  const platform = os.platform();
  const arch     = os.arch();

  if (platform === "win32") {
    return "nion-x86_64-windows.exe";
  }

  if (platform === "darwin") {
    // Apple Silicon vs Intel
    return arch === "arm64"
      ? "nion-aarch64-macos"
      : "nion-x86_64-macos";
  }

  if (platform === "linux") {
    return arch === "arm64" || arch === "aarch64"
      ? "nion-aarch64-linux"
      : "nion-x86_64-linux";
  }

  console.error(`Unsupported platform: ${platform} ${arch}`);
  console.error(`Download manually from: https://github.com/${REPO}/releases`);
  process.exit(1);
}

function download(url, dest, redirects = 0) {
  if (redirects > 5) {
    console.error("Too many redirects.");
    process.exit(1);
  }

  return new Promise((resolve, reject) => {
    https.get(url, { headers: { "User-Agent": "nion-cli-installer" } }, (res) => {
      // Follow redirects (GitHub releases redirect to CDN)
      if (res.statusCode === 301 || res.statusCode === 302) {
        return resolve(download(res.headers.location, dest, redirects + 1));
      }

      if (res.statusCode !== 200) {
        reject(new Error(`Download failed: HTTP ${res.statusCode}\nURL: ${url}`));
        return;
      }

      const file = fs.createWriteStream(dest);
      res.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", (err) => {
        fs.unlink(dest, () => {});
        reject(err);
      });
    }).on("error", reject);
  });
}

async function main() {
  const binaryName = getBinaryName();
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${binaryName}`;

  const binDir  = path.join(__dirname, "bin");
  const isWin   = os.platform() === "win32";
  const outName = isWin ? "nion.exe" : "nion";
  const outPath = path.join(binDir, outName);

  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  console.log(`\nInstalling nion v${VERSION} (${binaryName})...`);

  try {
    await download(url, outPath);
  } catch (err) {
    console.error(`\nFailed to download nion binary.`);
    console.error(err.message);
    console.error(`\nYou can install manually:`);
    console.error(`  curl -sSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash`);
    process.exit(1);
  }

  // Make executable on unix
  if (!isWin) {
    fs.chmodSync(outPath, 0o755);
  }

  console.log(`\n  nion installed successfully.`);
  console.log(`\n  Get started:`);
  console.log(`    nion config setup   <- add your API keys`);
  console.log(`    nion chat           <- start chatting`);
  console.log(`    nion agent          <- agentic mode (reads/writes files)\n`);
}

main();
