const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = path.join(__dirname, '../src-tauri/pointer-lock.js');
const script = fs.existsSync(source) ? fs.readFileSync(source, 'utf8') : '';
const flush = () => new Promise(resolve => setImmediate(resolve));

function load({ iframe = false, rejectLock = false, deferLock = false } = {}) {
  const calls = [], changes = [], errors = [], observers = [], pending = [];
  class Target {
    handlers = new Map();
    addEventListener(type, handler) {
      const handlers = this.handlers.get(type) ?? [];
      handlers.push(handler);
      this.handlers.set(type, handlers);
    }
    removeEventListener(type, handler) {
      this.handlers.set(type, (this.handlers.get(type) ?? []).filter(value => value !== handler));
    }
    dispatchEvent(event) {
      if (!event.target) Object.defineProperty(event, 'target', { value: this });
      for (const handler of this.handlers.get(event.type) ?? []) handler.call(this, event);
      if (event.bubbles && this.parent) this.parent.dispatchEvent(event);
      return !event.defaultPrevented;
    }
  }
  class Element extends Target {
    constructor(document) { super(); this.ownerDocument = document; this.parent = document; this.isConnected = true; }
    remove() {
      this.isConnected = false;
      this.parent = null;
      for (const observer of [...observers]) observer.callback([{ type: 'childList', removedNodes: [this], addedNodes: [] }]);
    }
  }
  const originalRequest = Element.prototype.requestPointerLock = () => Promise.reject(new DOMException('The window does not have focus', 'WrongDocumentError'));
  class Document extends Target {
    pointerLockElement = null;
    hidden = false;
    visibilityState = 'visible';
    hasFocus() { return true; }
    createElement() { return new Element(this); }
    contains(element) { return element.isConnected && element.ownerDocument === this; }
  }
  const document = new Document();
  const originalExit = document.exitPointerLock = () => {};
  document.documentElement = new Element(document);
  document.body = new Element(document);
  const window = new Target();
  document.parent = window;
  document.defaultView = window;
  window.top = iframe ? {} : window;
  window.self = window;
  window.window = window;
  window.document = document;
  window.location = new URL('https://pokebonk.com');
  window.__TAURI__ = { core: { invoke: (command, args) => {
    calls.push({ command, args: JSON.parse(JSON.stringify(args)) });
    if (args.locked && deferLock) return new Promise((resolve, reject) => pending.push({ resolve, reject }));
    if (args.locked && rejectLock) return Promise.reject(new Error('native permission denied'));
    return Promise.resolve();
  } } };
  class MutationObserver {
    constructor(callback) { this.callback = callback; }
    observe() { observers.push(this); }
    disconnect() { const index = observers.indexOf(this); if (index >= 0) observers.splice(index, 1); }
  }
  class MouseEvent extends Event {
    constructor(type, options = {}) {
      super(type, options);
      this.buttons = options.buttons ?? 0;
      this.clientX = options.clientX ?? 0;
      this.clientY = options.clientY ?? 0;
      for (const key of ['shiftKey', 'ctrlKey', 'altKey', 'metaKey']) this[key] = options[key] ?? false;
    }
    get movementX() { return 0; }
    get movementY() { return 0; }
  }
  Object.assign(window, { Element, Document, Event, MouseEvent, DOMException, MutationObserver, navigator: { userActivation: { isActive: true } } });
  document.addEventListener('pointerlockchange', () => changes.push(document.pointerLockElement));
  document.addEventListener('pointerlockerror', event => errors.push(event));
  vm.runInNewContext(script, { window, document, Element, Document, Event, MouseEvent, DOMException, MutationObserver, navigator: window.navigator, URL, console, setTimeout, clearTimeout, queueMicrotask });
  assert.equal(typeof window.__pokebonkInstallPointerLock, 'function', 'macOS Pointer Lock compatibility factory must be provided');
  window.__pokebonkInstallPointerLock();
  const target = document.createElement('canvas');
  const fire = (type, fields = {}) => window.dispatchEvent(Object.assign(new Event(type, { cancelable: true }), fields));
  return { window, document, target, calls, changes, errors, pending, fire, originalRequest, originalExit };
}

test('installing the bridge on a menu never acquires the mouse', async () => {
  const { window, document, calls, fire } = load();
  fire('focus');
  window.__pokebonkPointerLock.move(12, -5, 0);
  await flush();
  assert.equal(document.pointerLockElement, null);
  assert.equal(calls.length, 0);
});

test('a normal request locks the requested element only after native confirmation', async () => {
  const { target, document, calls, changes } = load();
  const request = target.requestPointerLock();
  assert.equal(document.pointerLockElement, null);
  await request;
  assert.equal(document.pointerLockElement, target);
  assert.deepEqual(changes, [target]);
  assert.deepEqual(calls, [{ command: 'pointer_lock', args: { locked: true } }]);
});

test('unadjusted movement rejects as unsupported and permits the game plain-request fallback', async () => {
  const { target, document, calls } = load();
  await assert.rejects(target.requestPointerLock({ unadjustedMovement: true }), { name: 'NotSupportedError' });
  assert.equal(document.pointerLockElement, null);
  assert.equal(calls.length, 0);
  await target.requestPointerLock();
  assert.equal(document.pointerLockElement, target);
});

test('native rejection emits an error without pretending to be locked', async () => {
  const { target, document, changes, errors } = load({ rejectLock: true });
  await assert.rejects(target.requestPointerLock());
  await flush();
  assert.equal(document.pointerLockElement, null);
  assert.equal(changes.length, 0);
  assert.equal(errors.length, 1);
});

test('native movement reaches the locked element and document with no mouse button held', async () => {
  const { window, document, target } = load();
  const local = [], global = [];
  target.addEventListener('mousemove', event => local.push(event));
  document.addEventListener('mousemove', event => global.push(event));
  await target.requestPointerLock();
  window.__pokebonkPointerLock.move(13, -7, 0);
  assert.equal(local.length, 1);
  assert.equal(global.length, 1);
  assert.equal(local[0].target, target);
  assert.equal(local[0].movementX, 13);
  assert.equal(local[0].movementY, -7);
  assert.equal(local[0].buttons, 0);
});

test('native movement preserves held modifier keys and clears them after release', async () => {
  const { window, target } = load();
  const moves = [];
  target.addEventListener('mousemove', event => moves.push(event));
  await target.requestPointerLock();
  const modifiers = { shiftKey: true, ctrlKey: true, altKey: true, metaKey: true };
  window.__pokebonkPointerLock.move(3, -2, 0, modifiers);
  window.__pokebonkPointerLock.move(3, -2, 0, {});
  for (const key of Object.keys(modifiers)) {
    assert.equal(moves[0][key], true, `${key} must match the native input event`);
    assert.equal(moves[1][key], false, `${key} must clear when no longer held`);
  }
});

for (const trigger of ['exitPointerLock', 'Escape', 'blur', 'native release']) {
  test(`${trigger} releases native capture and clears the locked DOM element`, async () => {
    const { window, document, target, calls, changes, fire } = load();
    await target.requestPointerLock();
    if (trigger === 'exitPointerLock') document.exitPointerLock();
    else if (trigger === 'native release') window.__pokebonkPointerLock.release();
    else fire(trigger === 'Escape' ? 'keydown' : 'blur', { key: 'Escape' });
    await flush();
    assert.equal(document.pointerLockElement, null);
    assert.deepEqual(changes, [target, null]);
    if (trigger !== 'native release') assert.ok(calls.some(call => call.command === 'pointer_lock' && call.args.locked === false));
    const moves = [];
    target.addEventListener('mousemove', event => moves.push(event));
    window.__pokebonkPointerLock.move(4, 8, 0);
    assert.equal(moves.length, 0);
  });
}

test('exiting before the native request acknowledgement cannot lock the mouse again', async () => {
  const { document, target, calls, changes, pending } = load({ deferLock: true });
  const request = target.requestPointerLock().catch(error => error);
  await flush();
  assert.equal(pending.length, 1);
  document.exitPointerLock();
  pending[0].resolve();
  await request;
  await flush();
  assert.equal(document.pointerLockElement, null);
  assert.equal(changes.filter(element => element === target).length, 0);
  assert.equal(calls.at(-1).args.locked, false);
});

test('removing the locked element releases native capture', async () => {
  const { target, document, calls, changes } = load();
  await target.requestPointerLock();
  target.remove();
  await flush();
  assert.equal(document.pointerLockElement, null);
  assert.deepEqual(changes, [target, null]);
  assert.equal(calls.at(-1).args.locked, false);
});

test('an iframe cannot install the native Pointer Lock compatibility API', () => {
  const { window, target, document, calls, originalRequest, originalExit } = load({ iframe: true });
  assert.equal(target.requestPointerLock, originalRequest);
  assert.equal(document.exitPointerLock, originalExit);
  assert.equal(window.__pokebonkPointerLock, undefined);
  assert.equal(calls.length, 0);
});

test('returning to the foreground never locks the mouse without a new game request', async () => {
  const { target, document, calls, fire } = load();
  await target.requestPointerLock();
  fire('blur');
  await flush();
  const lockRequests = calls.filter(call => call.args.locked).length;
  fire('focus');
  await flush();
  assert.equal(document.pointerLockElement, null);
  assert.equal(calls.filter(call => call.args.locked).length, lockRequests);
  await target.requestPointerLock();
  assert.equal(document.pointerLockElement, target);
});
