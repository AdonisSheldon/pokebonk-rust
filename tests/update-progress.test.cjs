const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const test = require('node:test');

function load(state) {
  const elements = Object.fromEntries(['stage', 'percentage', 'amount', 'version', 'progress'].map(id => [id, {
    textContent: '', value: null,
    removeAttribute(name) { if (name === 'value') this.value = null; },
  }]));
  const document = { documentElement: {}, getElementById: id => elements[id] };
  const window = { __pokebonkUpdateProgress: state };
  vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../web/update-progress.js'), 'utf8'), { window, document });
  return { window, document, elements };
}

test('download renders a real percentage including zero', () => {
  const view = load({ language: 'en', stage: 'Downloading update…', percentage: 0, downloaded: 0, total: 1048576, version: '0.1.6' });
  assert.equal(view.elements.progress.value, 0);
  assert.equal(view.elements.percentage.textContent, '0%');
  view.window.__pokebonkRenderUpdate({ language: 'en', stage: 'Downloading update…', percentage: 50, downloaded: 524288, total: 1048576, version: '0.1.6' });
  assert.equal(view.elements.progress.value, 50);
  assert.equal(view.elements.percentage.textContent, '50%');
  assert.equal(view.elements.amount.textContent, '512 KB / 1.0 MB');
  assert.equal(view.elements.version.textContent, 'v0.1.6');
});

test('unknown length and installation stay indeterminate', () => {
  const view = load({ language: 'zh-CN', stage: '正在下载更新…', percentage: null, downloaded: 524288, total: null, version: '0.1.6' });
  assert.equal(view.elements.progress.value, null);
  assert.equal(view.elements.percentage.textContent, '');
  assert.equal(view.elements.amount.textContent, '512 KB');
  assert.equal(view.document.documentElement.lang, 'zh-CN');
  view.window.__pokebonkRenderUpdate({ language: 'zh-CN', stage: '正在安装更新…', percentage: null, downloaded: null, total: null, version: '0.1.6' });
  assert.equal(view.elements.progress.value, null);
  assert.equal(view.elements.amount.textContent, '');
  assert.equal(view.elements.stage.textContent, '正在安装更新…');
});
