// WKWebView has no public host Pointer Lock delegate. Install only in the
// official game's guarded desktop bridge; Windows keeps its native web API.
window.__pokebonkInstallPointerLock = () => {
  delete window.__pokebonkInstallPointerLock;
  if (window.top !== window) return;
  const invoke = (locked) => window.__TAURI__.core.invoke('pointer_lock', { locked });
  let target = null;
  let sequence = 0;
  const change = () => document.dispatchEvent(new Event('pointerlockchange'));
  const release = () => {
    sequence++;
    observer.disconnect();
    if (!target) return;
    target = null;
    change();
  };
  const exit = () => {
    release();
    invoke(false).catch(error => console.warn('[PokeBonk pointer lock]', String(error)));
  };
  const observer = new MutationObserver(() => {
    if (target && !target.isConnected) exit();
  });
  Object.defineProperty(document, 'pointerLockElement', { configurable: true, get: () => target });
  Element.prototype.requestPointerLock = function(options) {
    if (options?.unadjustedMovement) {
      return Promise.reject(new DOMException('Native input uses macOS pointer acceleration', 'NotSupportedError'));
    }
    if (!this.isConnected || this.ownerDocument !== document || !document.hasFocus() || document.hidden) {
      return Promise.reject(new DOMException('Pointer lock requires a connected element in the focused page', 'WrongDocumentError'));
    }
    const requested = ++sequence;
    return invoke(true).then(() => {
      if (requested !== sequence) return;
      if (!this.isConnected || !document.hasFocus() || document.hidden) { exit(); return; }
      const previous = target;
      target = this;
      observer.observe(document, { childList: true, subtree: true });
      if (previous !== target) change();
    }, error => {
      document.dispatchEvent(new Event('pointerlockerror'));
      throw new DOMException(String(error), 'NotAllowedError');
    });
  };
  document.exitPointerLock = exit;
  window.__pokebonkPointerLock = Object.freeze({
    release,
    move(dx, dy, buttons, modifiers = {}) {
      if (!target) return;
      if (!target.isConnected || !document.hasFocus() || document.hidden) { exit(); return; }
      const event = new MouseEvent('mousemove', {
        bubbles: true, clientX: window.innerWidth / 2, clientY: window.innerHeight / 2, buttons,
        shiftKey: modifiers.shiftKey === true, ctrlKey: modifiers.ctrlKey === true,
        altKey: modifiers.altKey === true, metaKey: modifiers.metaKey === true,
      });
      // WebKit ignores movementX/Y in MouseEventInit; define the relative deltas explicitly.
      Object.defineProperties(event, { movementX: { value: dx }, movementY: { value: dy } });
      target.dispatchEvent(event);
    },
  });
  window.addEventListener('keydown', event => {
    if (event.key === 'Escape' && target) {
      event.preventDefault();
      event.stopImmediatePropagation();
      exit();
    }
  }, true);
  window.addEventListener('blur', exit);
  window.addEventListener('pagehide', exit);
  document.addEventListener('visibilitychange', () => { if (document.hidden) exit(); });
};
