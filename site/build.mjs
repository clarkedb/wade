import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import AdmZip from "adm-zip";

const SITE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(SITE);
export const BOARDS = {
  cores3: { name: "M5Stack CoreS3 Lite", family: "ESP32-S3" },
  cyd: { name: "CYD ESP32-2432S028R", family: "ESP32" },
};
const FILES = [
  "build.json",
  "install.bin",
  "app.bin",
  "install.json",
  "update.json",
];
const sha256 = (content) => createHash("sha256").update(content).digest("hex");
const escape = (text) =>
  text.replace(
    /[&<>"']/g,
    (char) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        char
      ],
  );

export function firmwareAssets(release) {
  const tag = release.tag_name;
  if (release.draft || release.prerelease || !/^v\d+\.\d+\.\d+$/.test(tag))
    return [];
  return release.assets.filter((asset) =>
    Object.keys(BOARDS).some(
      (board) => asset.name === `wade-${tag}-${board}.zip`,
    ),
  );
}

function downloadPackages(repo, output) {
  const pages = JSON.parse(
    execFileSync(
      "gh",
      ["api", `repos/${repo}/releases`, "--paginate", "--slurp"],
      { encoding: "utf8" },
    ),
  );
  for (const release of pages.flat()) {
    for (const asset of firmwareAssets(release)) {
      execFileSync(
        "gh",
        [
          "release",
          "download",
          release.tag_name,
          "--repo",
          repo,
          "--pattern",
          asset.name,
          "--dir",
          output,
        ],
        { stdio: "inherit" },
      );
    }
  }
}

function renderDownloads(catalog) {
  if (!catalog.length)
    return '<p>No public firmware releases yet. Follow <a href="https://github.com/clarkedb/wade/releases">GitHub Releases</a> for updates.</p>';
  const entries = catalog.map((release, index) => {
    const version = escape(release.version);
    const links = Object.entries(BOARDS)
      .filter(([board]) => release.boards[board])
      .map(
        ([board, { name }]) =>
          `<li><a href="${escape(release.boards[board].package)}">${name} ${version} firmware (ZIP)</a></li>`,
      )
      .join("");
    return `<li><a href="https://github.com/clarkedb/wade/releases/tag/${escape(release.tag)}">Wade ${version} release notes${index === 0 ? " · latest" : ""}</a><ul>${links}</ul></li>`;
  });
  return `<ul class="release-list">${entries.join("")}</ul>`;
}

export function build(packages, output) {
  mkdirSync(output, { recursive: true });
  cpSync(join(SITE, "public"), output, { recursive: true });
  copyFileSync(
    join(ROOT, "wade-core/tests/snapshots/pupils_0_neutral.png"),
    join(output, "wade.png"),
  );
  const releases = new Map();
  const filenames = existsSync(packages) ? readdirSync(packages) : [];
  for (const filename of filenames
    .filter((name) => /^wade-v.*\.zip$/.test(name))
    .sort()) {
    const archive = new AdmZip(join(packages, filename));
    const files = Object.fromEntries(
      FILES.map((name) => {
        const entry = archive.getEntry(name);
        assert(entry, `${filename}: missing ${name}`);
        return [name, entry.getData()];
      }),
    );
    const { version, board } = JSON.parse(files["build.json"]);
    assert.match(
      version,
      /^\d+\.\d+\.\d+$/,
      "Installer serves stable versions",
    );
    assert(
      Object.hasOwn(BOARDS, board) &&
        filename === `wade-v${version}-${board}.zip`,
      "Package board or filename mismatch",
    );
    const checksums = new Map(
      archive
        .readAsText("SHA256SUMS")
        .trim()
        .split("\n")
        .map((line) => {
          const match = line.match(/^([a-f0-9]{64})  (.+)$/);
          assert(match, `${filename}: invalid checksum line`);
          return [match[2], match[1]];
        }),
    );
    for (const [name, content] of Object.entries(files))
      assert.equal(
        sha256(content),
        checksums.get(name),
        `${filename}: ${name} checksum`,
      );
    assert.equal(files["app.bin"][0], 0xe9, "Invalid app flash header");
    assert(
      files["install.bin"]
        .subarray(0x10000, 0x10000 + files["app.bin"].length)
        .equals(files["app.bin"]),
      "Installation image must contain the same app",
    );
    for (const [mode, binary, offset] of [
      ["install", "install.bin", 0],
      ["update", "app.bin", 0x10000],
    ]) {
      const manifest = JSON.parse(files[`${mode}.json`]);
      assert.equal(manifest.version, version, "Manifest version mismatch");
      assert.equal(
        manifest.new_install_prompt_erase,
        true,
        "Erase prompt must be available",
      );
      assert.equal(
        manifest.new_install_improv_wait_time,
        0,
        "Wade does not use Improv",
      );
      assert.deepEqual(
        manifest.builds,
        [
          {
            chipFamily: BOARDS[board].family,
            parts: [{ path: binary, offset }],
          },
        ],
        `Invalid ${mode} manifest`,
      );
    }
    const relative = `firmware/v${version}/${board}`;
    const target = join(output, relative);
    mkdirSync(target, { recursive: true });
    for (const [name, content] of Object.entries(files))
      writeFileSync(join(target, name), content);
    copyFileSync(join(packages, filename), join(target, filename));
    if (!releases.has(version))
      releases.set(version, { version, tag: `v${version}`, boards: {} });
    releases.get(version).boards[board] = {
      install: `${relative}/install.json`,
      update: `${relative}/update.json`,
      package: `${relative}/${filename}`,
    };
  }
  const catalog = [...releases.values()].sort((a, b) => {
    const left = a.version.split(".").map(Number),
      right = b.version.split(".").map(Number);
    return right[0] - left[0] || right[1] - left[1] || right[2] - left[2];
  });
  writeFileSync(
    join(output, "releases.json"),
    JSON.stringify(catalog, null, 2) + "\n",
  );
  const page = join(output, "index.html");
  const template = readFileSync(page, "utf8");
  const marker =
    /<!-- release-downloads -->[\s\S]*?<!-- \/release-downloads -->/g;
  assert.equal(
    [...template.matchAll(marker)].length,
    1,
    "HTML needs one release download placeholder",
  );
  writeFileSync(
    page,
    template.replace(
      marker,
      `<!-- release-downloads -->\n${renderDownloads(catalog)}\n<!-- /release-downloads -->`,
    ),
  );
  writeFileSync(join(output, ".nojekyll"), "");
  return catalog;
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const { values } = parseArgs({
    options: {
      packages: { type: "string" },
      output: { type: "string", default: join(ROOT, "dist/site") },
      download: { type: "boolean", default: false },
      repo: { type: "string", default: "clarkedb/wade" },
    },
  });
  assert(
    !values.download || !values.packages,
    "Choose local packages or download published releases",
  );
  const packages = values.download
    ? mkdtempSync(join(tmpdir(), "wade-releases-"))
    : resolve(values.packages ?? join(ROOT, "dist"));
  try {
    if (values.download) downloadPackages(values.repo, packages);
    const output = resolve(values.output);
    const catalog = build(packages, output);
    console.log(`Installer: ${catalog.length} releases at ${output}`);
  } finally {
    if (values.download) rmSync(packages, { recursive: true, force: true });
  }
}
