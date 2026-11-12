(() => {
  const target = document.getElementById('target');
  const status = document.getElementById('status');
  const log = document.getElementById('log');
  let state;
  const locked = () => document.pointerLockElement === target;
  const message = text => { log.textContent = `${text}\n${log.textContent}`.slice(0, 6000); };
  function render() {
    status.textContent = [
      `document.hasFocus: ${document.hasFocus()}`,
      `requestPointerLock: ${typeof target.requestPointerLock}`,
      `exitPointerLock: ${typeof document.exitPointerLock}`,
      `pointerLockElement === target: ${locked()}`,
      `lock change / error: ${state.changes} / ${state.errors}`,
      `request: ${state.request}`,
      `LOCKED + buttons=0 + nonzero movement: ${state.freeMoves}`,
      `LOCKED + buttons!=0 + nonzero movement: ${state.dragMoves}`,
      `last movementX / Y: ${state.dx} / ${state.dy}; buttons=${state.buttons}`,
      `exitPointerLock / Esc / blur: ${state.exit} / ${state.escape} / ${state.blur}`,
    ].join('\n');
  }
  function reset() {
    state = { changes: 0, errors: 0, request: 'not requested', freeMoves: 0, dragMoves: 0, dx: 0, dy: 0, buttons: 0, exit: 'not tested', escape: 'not tested', blur: 'not tested' };
    log.textContent = '';
    render();
  }
  function checkRelease(reason) {
    const changesBefore = state.changes;
    state[reason] = 'waiting 500ms';
    render();
    setTimeout(() => {
      state[reason] = document.pointerLockElement != null ? 'FAIL: still locked'
        : state.changes > changesBefore ? 'PASS: released + change event' : 'FAIL: released without change event';
      message(`${reason}: ${state[reason]}`);
      render();
    }, 500);
  }
  target.addEventListener('click', () => {
    const changesBefore = state.changes;
    state.request = 'waiting 1500ms';
    render();
    try {
      const result = target.requestPointerLock();
      if (result && typeof result.catch === 'function') result.catch(error => {
        state.request = `FAIL: ${error.name}: ${error.message}`;
        message(state.request);
        render();
      });
    } catch (error) {
      state.request = `FAIL: ${error.name}: ${error.message}`;
      message(state.request);
      render();
      return;
    }
    setTimeout(() => {
      if (state.request === 'waiting 1500ms') {
        state.request = locked() && state.changes > changesBefore ? 'PASS: locked + change event' : 'FAIL: no lock/change';
        message(state.request);
        render();
      }
    }, 1500);
  });
  document.addEventListener('pointerlockchange', () => {
    state.changes++;
    message(`pointerlockchange: target=${locked()}`);
    render();
  });
  document.addEventListener('pointerlockerror', () => {
    state.errors++;
    state.request = 'FAIL: pointerlockerror';
    message(state.request);
    render();
  });
  document.addEventListener('mousemove', event => {
    if (!locked()) return;
    state.dx = event.movementX;
    state.dy = event.movementY;
    state.buttons = event.buttons;
    if (Number.isFinite(event.movementX) && Number.isFinite(event.movementY) && (event.movementX !== 0 || event.movementY !== 0)) {
      if (event.buttons === 0) state.freeMoves++;
      else state.dragMoves++;
    }
    render();
  });
  function exit() {
    if (!locked()) { message('exit skipped: acquire a lock first'); return; }
    checkRelease('exit');
    try { document.exitPointerLock(); }
    catch (error) { message(`exit threw: ${error}`); }
  }
  document.getElementById('exit').addEventListener('click', exit);
  document.getElementById('reset').addEventListener('click', reset);
  window.addEventListener('keydown', event => {
    if (event.key.toLowerCase() === 'x') exit();
    if (event.key === 'Escape' && locked()) checkRelease('escape');
  }, true);
  window.addEventListener('blur', () => {
    if (locked()) checkRelease('blur');
  });
  reset();
})();
