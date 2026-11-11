// Compatibility with upstream src/preload.ts; Tauri's remote capability limits
// native access to these same two HTTPS origins, independently of this JS guard.
(() => {
  const url = window.location;
  if (url.protocol !== 'https:' || !['pokebonk.com', 'www.pokebonk.com'].includes(url.hostname)
      || (url.port && url.port !== '443')) return;
  const invoke = (command, args) => window.__TAURI__.core.invoke(command, args);
  let fullscreen = false;
  function sync(state) {
    const next = state.mode === 'fullscreen';
    if (fullscreen !== next) {
      fullscreen = next;
      document.dispatchEvent(new Event('fullscreenchange'));
    }
    return state;
  }
  const display = () => invoke('display').then(sync);
  const setDisplay = (patch) => invoke('set_display', { patch }).then(sync);
  const action = (action) => invoke('native_action', { action });
  const report = error => console.warn('[PokeBonk Rust]', String(error));
  Object.defineProperty(window, 'pokebonkDesktop', {
    value: Object.freeze({
      version: '1.4.0',
      clientVersion: '0.1.3',
      engine: 'webkit',
      platform: 'darwin',
      display,
      setDisplay,
      fullscreen: () => fullscreen,
      quit: () => action('quit').catch(report),
      relaunch: () => action('relaunch').catch(report),
      retry: () => action('reconnect').catch(report),
      // Discord is reported disabled by display(); no native integration is enabled.
      presence: () => {},
      update: Object.freeze({
        state: async () => ({ state: 'off' }),
        onChange: () => {},
        install: () => {},
      }),
    }),
    writable: false,
    configurable: false,
  });
  window.addEventListener('pb:window-change', () => display().catch(report));
  window.addEventListener('keydown', event => {
    const key = event.key.toLowerCase();
    const modifier = event.metaKey || event.ctrlKey;
    if (event.key === 'F11' || (event.metaKey && event.ctrlKey && key === 'f')) {
      event.preventDefault();
      setDisplay({ mode: fullscreen ? 'window' : 'fullscreen' }).catch(report);
    } else if (event.key === 'F5' || (modifier && key === 'r')) {
      event.preventDefault();
      action('reload').catch(report);
    } else if (modifier && ['+', '-', '=', '0'].includes(key)) {
      event.preventDefault();
    }
  }, true);
  display().catch(report);
})();
