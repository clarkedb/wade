import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  unlinkSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import AdmZip from "adm-zip";
import { BOARDS, build, firmwareAssets } from "../build.mjs";

function workspace(t) {
  const directory = mkdtempSync(join(tmpdir(), "wade-installer-test-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const packages = join(directory, "packages"),
    output = join(directory, "site");
  mkdirSync(packages);
  return { packages, output };
}

function packageFirmware(directory, version, board, updateOffset = 0x10000) {
  const app = Buffer.from("\xe9firmware", "latin1");
  const files = {
    "build.json": Buffer.from(JSON.stringify({ version, board })),
    "app.bin": app,
    "install.bin": Buffer.concat([Buffer.alloc(0x10000), app]),
  };
  for (const [mode, binary, offset] of [
    ["install", "install.bin", 0],
    ["update", "app.bin", updateOffset],
  ]) {
    files[`${mode}.json`] = Buffer.from(
      JSON.stringify({
        version,
        new_install_prompt_erase: true,
        new_install_improv_wait_time: 0,
        builds: [
          {
            chipFamily: BOARDS[board].family,
            parts: [{ path: binary, offset }],
          },
        ],
      }),
    );
  }
  const checksums = Object.entries(files)
    .map(
      ([name, content]) =>
        `${createHash("sha256").update(content).digest("hex")}  ${name}\n`,
    )
    .join("");
  const zip = new AdmZip();
  for (const [name, content] of Object.entries(files))
    zip.addFile(name, content);
  zip.addFile("SHA256SUMS", Buffer.from(checksums));
  const filename = join(directory, `wade-v${version}-${board}.zip`);
  zip.writeZip(filename);
  return filename;
}

function downloads(output) {
  const html = readFileSync(join(output, "index.html"), "utf8")
    .split("<!-- release-downloads -->")[1]
    .split("<!-- /release-downloads -->")[0];
  return {
    html,
    links: [...html.matchAll(/href="([^"]+)"/g)].map((match) => match[1]),
  };
}

test("HTML downloads match catalog and numeric version order without JavaScript", (t) => {
  const { packages, output } = workspace(t);
  packageFirmware(packages, "0.2.0", "cyd");
  packageFirmware(packages, "0.10.0", "cores3");
  packageFirmware(packages, "0.10.0", "cyd");
  const catalog = build(packages, output),
    page = downloads(output);
  assert.deepEqual(
    catalog.map((item) => item.version),
    ["0.10.0", "0.2.0"],
  );
  assert.deepEqual(
    page.links.filter((link) => link.startsWith("https:")),
    [
      "https://github.com/clarkedb/wade/releases/tag/v0.10.0",
      "https://github.com/clarkedb/wade/releases/tag/v0.2.0",
    ],
  );
  assert.deepEqual(
    new Set(page.links.filter((link) => !link.startsWith("https:"))),
    new Set(
      catalog.flatMap((release) =>
        Object.values(release.boards).map((board) => board.package),
      ),
    ),
  );
  assert.equal((page.html.match(/latest/g) ?? []).length, 1);
  for (const link of page.links.filter((link) => !link.startsWith("https:")))
    assert(existsSync(join(output, link)), link);
  for (const filename of [
    "build.mjs",
    "package.json",
    "package-lock.json",
    "node_modules",
  ])
    assert(
      !existsSync(join(output, filename)),
      "Build tools must not be deployed",
    );
});

test("empty or missing packages explain availability without JavaScript", (t) => {
  const { packages, output } = workspace(t);
  for (const input of [packages, join(packages, "missing")]) {
    assert.deepEqual(build(input, output), []);
    assert.match(downloads(output).html, /No public firmware releases yet/);
    assert.deepEqual(downloads(output).links, [
      "https://github.com/clarkedb/wade/releases",
    ]);
  }
});

test("rebuilding removes withdrawn releases from the HTML and catalog", (t) => {
  const { packages, output } = workspace(t);
  const filename = packageFirmware(packages, "0.1.0", "cyd");
  build(packages, output);
  unlinkSync(filename);
  assert.deepEqual(build(packages, output), []);
  assert(!downloads(output).html.includes("0.1.0"));
  assert.deepEqual(JSON.parse(readFileSync(join(output, "releases.json"))), []);
});

test("reject corrupted package contents", (t) => {
  const { packages, output } = workspace(t);
  const filename = packageFirmware(packages, "0.1.0", "cyd");
  const zip = new AdmZip(filename);
  zip.updateFile("app.bin", Buffer.from("changed"));
  zip.writeZip(filename);
  assert.throws(() => build(packages, output), /app.bin checksum/);
});

test("reject an update manifest aimed at the settings partition", (t) => {
  const { packages, output } = workspace(t);
  packageFirmware(packages, "0.1.0", "cyd", 0x9000);
  assert.throws(() => build(packages, output), /Invalid update manifest/);
});

test("only published stable releases and exact supported packages are downloaded", () => {
  const release = {
    draft: false,
    prerelease: false,
    tag_name: "v0.1.0",
    assets: [
      { name: "wade-v0.1.0-cores3.zip" },
      { name: "wade-v0.1.0-cyd.zip" },
      { name: "wade-v0.1.0-other.zip" },
      { name: "wade-v0.2.0-cyd.zip" },
      { name: "debug.elf" },
    ],
  };
  assert.deepEqual(
    firmwareAssets(release).map((asset) => asset.name),
    ["wade-v0.1.0-cores3.zip", "wade-v0.1.0-cyd.zip"],
  );
  for (const overrides of [
    { draft: true },
    { prerelease: true },
    { tag_name: "v0.1.0-beta.1" },
    { tag_name: "main" },
  ])
    assert.deepEqual(firmwareAssets({ ...release, ...overrides }), []);
});
