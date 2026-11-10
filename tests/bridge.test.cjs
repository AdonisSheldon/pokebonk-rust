const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

const script = fs.readFileSync('src-tauri/bridge.js', 'utf8');
function load(origin = 'https://pokebonk.com') {
  const calls = [], events = [], handlers = {};
  const state = { mode: 'window', size: '1280x720', sizes: ['1280x720'], monitor: 0, monitors: [], angle: 'auto', angles: ['auto'], vsync: true, discord: false };
  const window = {
    location: new URL(origin),
    addEventListener: (type, handler) => { handlers[type] = handler; },
    __TAURI__: { core: { invoke: async (command, args) => {
      calls.push({ command, args });
      if (command === 'set_display') state.mode = args.patch.mode ?? state.mode;
      return { ...state };
    } } },
  };
  const document = { dispatchEvent: (e) => events.push(e.type) };
  vm.runInNewContext(script, { window, document, URL, Event, console, setTimeout, clearTimeout });
  return { window, document, calls, events, handlers, state };
}

test('desktop bridge is absent on non-game pages and iframe origins', () => {
  for (const origin of ['http://pokebonk.com', 'https://pokebonk.com.evil.test', 'https://api.pokebonk.com', 'https://pokebonk.com:8443']) {
    assert.equal(load(origin).window.pokebonkDesktop, undefined, origin);
  }
});

test('desktop display follows the actual native fullscreen result', async () => {
  const { window, events } = load();
  const desktop = window.pokebonkDesktop;
  assert.equal(desktop.platform, 'darwin');
  assert.equal(desktop.engine, 'webkit');
  assert.equal(desktop.fullscreen(), false);
  await desktop.setDisplay({ mode: 'fullscreen' });
  assert.equal(desktop.fullscreen(), true);
  assert.ok(events.includes('fullscreenchange'));
  assert.ok(Object.isFrozen(desktop));
});

test('compatibility update API never installs Electron updates', async () => {
  const { window, calls } = load();
  assert.equal((await window.pokebonkDesktop.update.state()).state, 'off');
  window.pokebonkDesktop.update.install();
  assert.equal(calls.some(c => /install|update/i.test(c.command)), false);
});

test('F11 goes through the native fullscreen command, Escape remains a game key', async () => {
  const { handlers, calls } = load();
  let prevented = false;
  handlers.keydown({ key: 'F11', preventDefault: () => { prevented = true; } });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(prevented, true);
  assert.ok(calls.some(c => c.command === 'set_display' && c.args.patch.mode === 'fullscreen'));
  prevented = false;
  handlers.keydown({ key: 'Escape', preventDefault: () => { prevented = true; } });
  assert.equal(prevented, false);
});
