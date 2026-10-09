const board = document.querySelector('#board');
const release = document.querySelector('#release');
const installer = document.querySelector('#installer');
const connect = document.querySelector('#connect');
const status = document.querySelector('#status');
const download = document.querySelector('#download');
const notes = document.querySelector('#notes');
const hint = document.querySelector('#mode-hint');
let releases = [];
let toolsState = 'idle';

document.querySelector('#usb-controls').hidden = false;
status.textContent = 'Loading available firmware…';

async function loadTools() {
  toolsState = 'loading';
  try {
    await import('https://unpkg.com/esp-web-tools@10.4.0/dist/web/install-button.js?module');
    await customElements.whenDefined('esp-web-install-button');
    toolsState = 'ready';
  } catch (error) {
    toolsState = 'failed';
    console.error('USB installer failed to load', error);
  }
  update();
}

function update() {
  connect.disabled = true;
  installer.removeAttribute('manifest');
  const selected = releases.find(item => item.tag === release.value);
  const build = selected?.boards[board.value];
  download.hidden = !build;
  notes.hidden = !selected;
  if (selected) notes.href = `https://github.com/clarkedb/wade/releases/tag/${encodeURIComponent(selected.tag)}`;
  if (build) {
    download.href = build.package;
    download.textContent = `Download ${board.options[board.selectedIndex].text} ${selected.version} firmware`;
  }
  if (selected) notes.textContent = `Wade ${selected.version} release notes`;
  const mode = document.querySelector('input[name="mode"]:checked')?.value;
  hint.textContent = mode === 'update'
    ? 'In the installer, leave “Erase device” unchecked to keep your settings. Update requires Wade to be installed already.'
    : mode === 'install'
      ? 'This replaces existing firmware and resets settings. Use a USB data cable and close any serial monitor.'
      : 'Use a USB data cable and close any serial monitor.';
  if (!releases.length) return;
  if (!board.value) { status.textContent = 'Choose your device to continue.'; return; }
  if (!build) { status.textContent = 'This version has no firmware for your board. Choose another version.'; return; }
  if (!mode) { status.textContent = 'Choose first installation or an update.'; return; }
  if (!('serial' in navigator)) { status.textContent = 'Download the package, or use a desktop browser with Web Serial.'; return; }
  if (!window.isSecureContext) { status.textContent = 'USB installation requires HTTPS. Firmware downloads are still available.'; return; }
  if (toolsState === 'idle') loadTools();
  if (toolsState !== 'ready') {
    status.textContent = toolsState === 'loading'
      ? 'Loading the USB installer…'
      : 'USB installer unavailable. Download the firmware package instead.';
    return;
  }
  installer.setAttribute('manifest', new URL(build[mode], window.location.href).href);
  connect.textContent = mode === 'update' ? 'Connect and update' : 'Connect and install';
  connect.disabled = false;
  status.textContent = `Wade ${selected.version} is ready. Connect your board over USB.`;
}

board.addEventListener('change', update);
release.addEventListener('change', update);
document.querySelectorAll('input[name="mode"]').forEach(input => input.addEventListener('change', update));

async function load() {
  const browserHint = document.querySelector('#browser-hint');
  if (!('serial' in navigator) || !window.isSecureContext) {
    browserHint.hidden = false;
    browserHint.textContent = !('serial' in navigator)
      ? 'This browser cannot install over USB. Use desktop Chrome, Edge, or Firefox 151 or newer, or download a firmware package.'
      : 'Open this page over HTTPS to connect your device. Firmware downloads are still available.';
  }
  try {
    const response = await fetch('releases.json');
    if (!response.ok) throw new Error(`Release catalog: ${response.status}`);
    releases = await response.json();
    if (!Array.isArray(releases)) throw new Error('Invalid release catalog');
    release.replaceChildren();
    if (!releases.length) {
      release.add(new Option('No releases yet', ''));
      status.textContent = 'No public firmware releases yet. Check GitHub for project updates.';
      return;
    }
    releases.forEach((item, index) => release.add(new Option(`${item.version}${index === 0 ? ' · latest' : ''}`, item.tag)));
    release.disabled = false;
    update();
  } catch (error) {
    release.replaceChildren(new Option('Releases unavailable', ''));
    status.textContent = 'Firmware releases could not be loaded. Download a package from GitHub Releases.';
    download.href = 'https://github.com/clarkedb/wade/releases';
    download.hidden = false;
    console.error(error);
  } finally {
    release.removeAttribute('aria-busy');
  }
}

load();
