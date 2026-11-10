document.addEventListener('DOMContentLoaded', () => {
  document.title = 'Audio Focus Test';
  document.body.innerHTML = `<h1>Audio focus regression</h1>
    <p>Start, switch to another app, then return. The gain must remain 0.0001.</p>
    <button id="start">Start quiet local audio</button>
    <button id="stop">Pause intentionally</button><pre id="report"></pre>`;
  const active = new AudioContext();
  const held = new AudioContext();
  held.suspend();
  const gain = active.createGain();
  gain.gain.value = 0.0001;
  const tone = active.createOscillator();
  tone.connect(gain).connect(active.destination);
  tone.start();
  // A silent one-second WAV tests HTML media independently from Web Audio.
  const bytes = new Uint8Array(8044), wav = new DataView(bytes.buffer);
  const tag = (offset, value) => [...value].forEach((c, i) => bytes[offset + i] = c.charCodeAt(0));
  tag(0, 'RIFF'); wav.setUint32(4, 8036, true); tag(8, 'WAVE'); tag(12, 'fmt ');
  wav.setUint32(16, 16, true); wav.setUint16(20, 1, true); wav.setUint16(22, 1, true);
  wav.setUint32(24, 8000, true); wav.setUint32(28, 8000, true);
  wav.setUint16(32, 1, true); wav.setUint16(34, 8, true);
  tag(36, 'data'); wav.setUint32(40, 8000, true); bytes.fill(128, 44);
  const media = new Audio(URL.createObjectURL(new Blob([bytes], { type: 'audio/wav' })));
  media.loop = true;
  media.volume = 0.37;
  window.audioReport = () => ({
    focused: document.hasFocus(), active: active.state, held: held.state,
    gain: gain.gain.value, mediaPaused: media.paused, mediaVolume: media.volume,
  });
  document.getElementById('start').onclick = async () => {
    await active.resume();
    await media.play();
  };
  document.getElementById('stop').onclick = async () => { await active.suspend(); media.pause(); };
  setInterval(() => document.getElementById('report').textContent = JSON.stringify(window.audioReport(), null, 2), 100);
});
