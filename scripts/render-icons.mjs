// `npm run icons`: rasterises the app icon from src-tauri/icons/source/ and regenerates every bundle icon.
//
// 1. Renders the full art (icon.svg) to a 1024 px src-tauri/icons/icon.png with the project's Playwright WebKit.
// 2. Runs Tauri's own generator on it for the full platform set (`tauri icon`), then restores the 1024 px icon.png
//    (the generator overwrites it with a 512 px copy).
// 3. Builds the macOS icon.icns with `iconutil` from an .iconset: the simplified art (icon-small.svg) for 16x16,
//    16x16@2x and 32x32, the full art for 32x32@2x and every larger size. It replaces the generated icon.icns, and
//    the small art at 32 px replaces the generated 32x32.png.
//
// `--preview <dir>` also writes preview PNGs (full art at 1024 and 128, small art at 32 and 16) into <dir>.
// macOS only (iconutil); no tools beyond the installed devDependencies. PLAYWRIGHT_BROWSERS_PATH=0 is set by the
// npm script so the WebKit in node_modules is used.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { webkit } from "@playwright/test";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const icons = join(root, "src-tauri", "icons");
const art = {
  full: readFileSync(join(icons, "source", "icon.svg"), "utf8"),
  small: readFileSync(join(icons, "source", "icon-small.svg"), "utf8"),
};

// iconutil's slot names, each with the art and the pixel size it holds.
const ICONSET = [
  ["icon_16x16.png", "small", 16],
  ["icon_16x16@2x.png", "small", 32],
  ["icon_32x32.png", "small", 32],
  ["icon_32x32@2x.png", "full", 64],
  ["icon_128x128.png", "full", 128],
  ["icon_128x128@2x.png", "full", 256],
  ["icon_256x256.png", "full", 256],
  ["icon_256x256@2x.png", "full", 512],
  ["icon_512x512.png", "full", 512],
  ["icon_512x512@2x.png", "full", 1024],
];

const PREVIEWS = [
  ["icon-1024.png", "full", 1024],
  ["icon-128.png", "full", 128],
  ["icon-32.png", "small", 32],
  ["icon-16.png", "small", 16],
];

function previewDir() {
  const i = process.argv.indexOf("--preview");
  return i > 0 && process.argv[i + 1] ? resolve(process.argv[i + 1]) : null;
}

/** Renders `svg` to a transparent PNG of exactly `size` x `size` device pixels. */
async function render(browser, svg, size, out) {
  const page = await browser.newPage({ viewport: { width: size, height: size }, deviceScaleFactor: 1 });
  const sized = svg.replace(/\swidth="\d+"\s+height="\d+"/, ` width="${size}" height="${size}"`);
  await page.setContent(
    `<!doctype html><html><body style="margin:0;background:transparent;overflow:hidden">${sized}</body></html>`,
  );
  await page.locator("svg").screenshot({ path: out, omitBackground: true });
  await page.close();
}

function pixelSize(file) {
  const out = execFileSync("sips", ["-g", "pixelWidth", "-g", "pixelHeight", file], { encoding: "utf8" });
  const [w, h] = [/pixelWidth: (\d+)/, /pixelHeight: (\d+)/].map((re) => Number(out.match(re)?.[1]));
  return { w, h };
}

function expectSize(file, size) {
  const { w, h } = pixelSize(file);
  if (w !== size || h !== size) throw new Error(`${file} is ${w}x${h}, expected ${size}x${size}`);
}

const work = mkdtempSync(join(tmpdir(), "fanout-icons-"));
const iconset = join(work, "icon.iconset");
mkdirSync(iconset);
const browser = await webkit.launch();
try {
  // `tauri icon` writes its own 512 px icon.png, so the 1024 px master is put back after it runs.
  const master = join(work, "icon-1024.png");
  const iconPng = join(icons, "icon.png");
  await render(browser, art.full, 1024, master);
  expectSize(master, 1024);
  copyFileSync(master, iconPng);
  execFileSync("npx", ["--no-install", "tauri", "icon", iconPng, "-o", icons], { cwd: root, stdio: "inherit" });
  copyFileSync(master, iconPng);
  expectSize(iconPng, 1024);

  for (const [name, which, size] of ICONSET) {
    const file = join(iconset, name);
    await render(browser, art[which], size, file);
    expectSize(file, size);
  }
  execFileSync("iconutil", ["-c", "icns", iconset, "-o", join(icons, "icon.icns")]);
  copyFileSync(join(iconset, "icon_32x32.png"), join(icons, "32x32.png"));

  const preview = previewDir();
  if (preview) {
    mkdirSync(preview, { recursive: true });
    for (const [name, which, size] of PREVIEWS) await render(browser, art[which], size, join(preview, name));
  }

  // Round-trip the .icns to prove every slot is in it at its size.
  const check = join(work, "check.iconset");
  execFileSync("iconutil", ["-c", "iconset", join(icons, "icon.icns"), "-o", check]);
  for (const [name, , size] of ICONSET) {
    expectSize(join(check, name), size);
    console.log(`icon.icns  ${name.padEnd(22)} ${size}x${size}`);
  }
} finally {
  await browser.close();
  rmSync(work, { recursive: true, force: true });
}
