(() => {
  const element = id => document.getElementById(id);
  const bytes = value => value < 1048576 ? `${Math.ceil(value / 1024)} KB` : `${(value / 1048576).toFixed(1)} MB`;
  window.__pokebonkRenderUpdate = state => {
    document.documentElement.lang = state.language;
    element('stage').textContent = state.stage;
    element('version').textContent = `v${state.version}`;
    if (state.percentage == null) {
      element('progress').removeAttribute('value');
      element('percentage').textContent = '';
    } else {
      element('progress').value = state.percentage;
      element('percentage').textContent = `${state.percentage}%`;
    }
    element('amount').textContent = state.downloaded == null ? '' :
      bytes(state.downloaded) + (state.total ? ` / ${bytes(state.total)}` : '');
  };
  if (window.__pokebonkUpdateProgress) window.__pokebonkRenderUpdate(window.__pokebonkUpdateProgress);
})();
