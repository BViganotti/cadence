/* Cadence Studio — original, dependency-free browser client. */
(() => {
  "use strict";

  if (window.location.search.includes("token=")) {
    window.history.replaceState(null, "", window.location.pathname);
  }

  const $ = (selector) => document.querySelector(selector);
  const $$ = (selector) => Array.from(document.querySelectorAll(selector));

  const state = {
    summary: null,
    selectedPart: 0,
    selectedMeasure: 0,
    selectedVoice: 1,
    selectedEvent: null,
    selectedNote: 0,
    duration: "quarter",
    dots: 0,
    octave: 4,
    pitch: "C",
    previewAll: false,
    zoom: 1,
    busy: false,
    ready: false,
    refreshTimer: 0,
  };

  const DURATIONS = [
    ["whole", "whole"],
    ["half", "half"],
    ["quarter", "quarter"],
    ["eighth", "eighth"],
    ["16th", "16th"],
    ["32nd", "32nd"],
    ["64th", "64th"],
  ];
  const PITCHES = ["C", "D", "E", "F", "G", "A", "B"];
  const DYNAMICS = [
    ["", "none"],
    ["ppp", "ppp"],
    ["pp", "pp"],
    ["p", "p"],
    ["mp", "mp"],
    ["mf", "mf"],
    ["f", "f"],
    ["ff", "ff"],
    ["fff", "fff"],
    ["sfz", "sfz"],
    ["fp", "fp"],
    ["sf", "sf"],
  ];
  const KEYS = [
    [-7, "Cb major / Ab minor"],
    [-6, "Gb major / Eb minor"],
    [-5, "Db major / Bb minor"],
    [-4, "Ab major / F minor"],
    [-3, "Eb major / C minor"],
    [-2, "Bb major / G minor"],
    [-1, "F major / D minor"],
    [0, "C major / A minor"],
    [1, "G major / E minor"],
    [2, "D major / B minor"],
    [3, "A major / F# minor"],
    [4, "E major / C# minor"],
    [5, "B major / G# minor"],
    [6, "F# major / D# minor"],
    [7, "C# major / A# minor"],
  ];

  const stage = $("#scoreStage");
  const toast = $("#toast");
  const ZOOM_LEVELS = [
    { value: 0.7, label: "70%", attribute: "small" },
    { value: 1, label: "100%", attribute: "normal" },
    { value: 1.4, label: "140%", attribute: "large" },
    { value: 2, label: "200%", attribute: "xlarge" },
  ];

  function applyZoom(delta) {
    const index = ZOOM_LEVELS.findIndex((level) => level.value === state.zoom);
    const next = clamp((index < 0 ? 1 : index) + delta, 0, ZOOM_LEVELS.length - 1);
    state.zoom = ZOOM_LEVELS[next].value;
    stage.dataset.zoom = ZOOM_LEVELS[next].attribute === "normal" ? "" : ZOOM_LEVELS[next].attribute;
    $("#zoomLabel").textContent = ZOOM_LEVELS[next].label;
  }

  function clamp(value, min, max) {
    return Math.max(min, Math.min(max, value));
  }

  function fail(message) {
    showToast(message, "error");
    setConnection("error");
  }

  function showToast(message, kind = "") {
    toast.textContent = message;
    toast.className = `toast ${kind}`;
    toast.hidden = false;
    window.clearTimeout(showToast.timer);
    showToast.timer = window.setTimeout(() => {
      toast.hidden = true;
    }, kind === "error" ? 6500 : 3200);
  }

  function setConnection(kind) {
    const dot = $("#statusDot");
    dot.classList.remove("ok", "bad");
    const label = $("#connectionLabel");
    if (kind === "ok") {
      dot.classList.add("ok");
      label.textContent = "live";
    } else if (kind === "error") {
      dot.classList.add("bad");
      label.textContent = "offline";
    } else {
      label.textContent = "connecting";
    }
  }

  async function api(path, options = {}) {
    const response = await fetch(path, {
      credentials: "same-origin",
      cache: "no-store",
      ...options,
      headers: {
        ...(options.body ? { "Content-Type": options.contentType || "application/json" } : {}),
        ...(options.headers || {}),
      },
    });
    const type = response.headers.get("Content-Type") || "";
    if (!response.ok) {
      let message = `${response.status} ${response.statusText}`;
      if (type.includes("application/json")) {
        try {
          const payload = await response.json();
          if (payload && payload.error) message = payload.error;
        } catch (_) {
          /* keep status text */
        }
      }
      throw new Error(message);
    }
    if (type.includes("application/json")) return response.json();
    return response;
  }

  async function edit(command) {
    if (state.busy) return;
    state.busy = true;
    try {
      await api("/api/edit", { method: "POST", body: JSON.stringify(command) });
      await refresh();
    } catch (error) {
      fail(error.message);
    } finally {
      state.busy = false;
    }
  }

  function selectedPartInfo() {
    if (!state.summary) return null;
    return state.summary.parts[state.selectedPart] || null;
  }

  function renderOptions() {
    const durationSelect = $("#inspDuration");
    durationSelect.innerHTML = "";
    const durationEntry = $("#durations");
    durationEntry.innerHTML = "";
    for (const [value, label] of DURATIONS) {
      const option = document.createElement("option");
      option.value = value;
      option.textContent = label;
      durationSelect.append(option);
      const chip = document.createElement("button");
      chip.type = "button";
      chip.className = "chip";
      chip.dataset.duration = value;
      chip.textContent = label;
      chip.title = `Enter a ${label} note`;
      if (value === state.duration) chip.classList.add("active");
      chip.addEventListener("click", () => {
        state.duration = value;
        $$("#durations .chip").forEach((item) => {
          item.classList.toggle("active", item.dataset.duration === value);
        });
      });
      durationEntry.append(chip);
    }

    const dynamicSelect = $("#inspDynamic");
    dynamicSelect.innerHTML = "";
    for (const [value, label] of DYNAMICS) {
      const option = document.createElement("option");
      option.value = value;
      option.textContent = label;
      dynamicSelect.append(option);
    }

    const keySelect = $("#keySelect");
    keySelect.innerHTML = "";
    for (const [value, label] of KEYS) {
      const option = document.createElement("option");
      option.value = String(value);
      option.textContent = label;
      keySelect.append(option);
    }
    keySelect.value = "0";

    const octave = $("#octave");
    octave.innerHTML = "";
    for (let value = 2; value <= 6; value += 1) {
      const option = document.createElement("option");
      option.value = String(value);
      option.textContent = `C${value}`;
      if (value === state.octave) option.selected = true;
      octave.append(option);
    }

    const pad = $("#pitchPad");
    pad.innerHTML = "";
    for (const pitch of PITCHES) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "pitch-key";
      button.textContent = pitch;
      button.title = `Add ${pitch}${state.octave}`;
      button.addEventListener("click", () => {
        state.pitch = pitch;
        addNote(`${pitch}${state.octave}`);
      });
      pad.append(button);
    }
  }

  function renderParts() {
    const host = $("#parts");
    host.innerHTML = "";
    const summary = state.summary;
    if (!summary) return;
    $("#partCount").textContent = String(summary.parts.length);
    const full = document.createElement("button");
    full.type = "button";
    full.className = "part";
    if (state.previewAll) full.classList.add("active");
    const fullName = document.createElement("span");
    fullName.className = "part-name";
    fullName.textContent = "Full score";
    const fullNotes = document.createElement("span");
    fullNotes.className = "part-notes";
    fullNotes.textContent = `${summary.parts.length} staves`;
    full.append(fullName, fullNotes);
    full.addEventListener("click", () => {
      state.previewAll = true;
      state.selectedEvent = null;
      state.selection = null;
      renderParts();
      refreshPreview().catch((error) => fail(error.message));
    });
    host.append(full);
    summary.parts.forEach((part) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "part";
      if (part.index === state.selectedPart && !state.previewAll) button.classList.add("active");
      const name = document.createElement("span");
      name.className = "part-name";
      name.textContent = part.name || part.id;
      const notes = document.createElement("span");
      notes.className = "part-notes";
      notes.textContent = `${part.notes} notes`;
      button.append(name, notes);
      button.addEventListener("click", () => {
        state.previewAll = false;
        selectPart(part.index);
      });
      host.append(button);
    });
  }

  function renderMeasures() {
    const host = $("#measures");
    host.innerHTML = "";
    const part = selectedPartInfo();
    if (!part) return;
    $("#measureCount").textContent = String(part.measures);
    for (let index = 0; index < part.measures; index += 1) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "chip";
      button.textContent = String(index + 1);
      button.title = `Measure ${index + 1}`;
      if (index === state.selectedMeasure) button.classList.add("active");
      button.addEventListener("click", () => {
        state.selectedMeasure = index;
        state.selectedEvent = null;
        renderMeasures();
        updateSelectionUI();
        highlightSelection();
      });
      host.append(button);
    }
    $("#measureBadge").textContent = `m${state.selectedMeasure + 1}`;
  }

  function renderDiagnostics() {
    const summary = state.summary;
    const host = $("#diagnostics");
    host.innerHTML = "";
    if (!summary) return;
    const diagnostics = summary.diagnostics;
    $("#diagnosticCount").textContent = String(diagnostics.errors + diagnostics.warnings);
    const badge = $("#diagnosticBadge");
    badge.className = "badge";
    if (diagnostics.errors > 0) {
      badge.classList.add("bad");
      badge.textContent = `${diagnostics.errors} errors`;
    } else if (diagnostics.warnings > 0) {
      badge.classList.add("warn");
      badge.textContent = `${diagnostics.warnings} warnings`;
    } else {
      badge.classList.add("ok");
      badge.textContent = "no diagnostics";
    }
    if (!diagnostics.items.length) {
      const empty = document.createElement("div");
      empty.className = "diagnostic";
      empty.textContent = "Clean score. No underfull or overfull measures.";
      host.append(empty);
      return;
    }
    for (const item of diagnostics.items) {
      const row = document.createElement("div");
      row.className = `diagnostic ${item.severity}`;
      row.textContent = item.message;
      host.append(row);
    }
  }

  function renderSummary() {
    const summary = state.summary;
    if (!summary) return;
    $("#scoreTitle").textContent = summary.metadata.title || "Untitled";
    if (document.activeElement !== $("#titleInput")) {
      $("#titleInput").value = summary.metadata.title || "";
    }
    if (document.activeElement !== $("#subtitleInput")) {
      $("#subtitleInput").value = summary.metadata.subtitle || "";
    }
    if (document.activeElement !== $("#composerInput")) {
      $("#composerInput").value = summary.metadata.composer || "";
    }
    const meta = [];
    if (summary.metadata.subtitle) meta.push(summary.metadata.subtitle);
    if (summary.metadata.composer) meta.push(summary.metadata.composer);
    meta.push(`${summary.measureCount} measures`);
    meta.push(`${summary.noteCount} notes`);
    meta.push(`${summary.chordCount} chords`);
    meta.push(`${summary.restCount} rests`);
    meta.push(summary.source);
    $("#scoreMeta").textContent = meta.join(" · ");
    $("#footerStatus").textContent =
      `v${summary.version} · ${summary.parts.length} part(s) · ${summary.eventCount} events · ` +
      `${summary.diagnostics.errors} error(s), ${summary.diagnostics.warnings} warning(s)`;
    $("#undoBtn").disabled = !summary.canUndo;
    $("#redoBtn").disabled = !summary.canRedo;
    renderParts();
    renderMeasures();
    renderDiagnostics();
  }

  async function refresh() {
    if (state.busy && !state.ready) return;
    try {
      const summary = await api("/api/state");
      state.summary = summary;
      state.selectedPart = clamp(state.selectedPart, 0, Math.max(0, summary.parts.length - 1));
      const part = selectedPartInfo();
      if (part) state.selectedMeasure = clamp(state.selectedMeasure, 0, Math.max(0, part.measures - 1));
      renderSummary();
      await refreshPreview();
      await refreshSelection();
      state.ready = true;
      setConnection("ok");
    } catch (error) {
      fail(error.message);
    }
  }

  async function refreshPreview() {
    const part = selectedPartInfo();
    if (!part) return;
    const partParameter = state.previewAll ? "all" : String(state.selectedPart);
    const response = await api(
      `/api/layout.svg?part=${encodeURIComponent(partParameter)}&hits=1`,
    );
    const svg = await response.text();
    stage.innerHTML = svg;
    stage.setAttribute("aria-busy", "false");
    $("#pageBadge").textContent = `${(svg.match(/<g id="page-/g) || []).length} page(s)`;
    stage.querySelectorAll(".hit").forEach((hit) => {
      hit.addEventListener("click", (event) => {
        event.stopPropagation();
        selectEvent(
          Number(hit.dataset.part),
          Number(hit.dataset.measure),
          Number(hit.dataset.voice),
          Number(hit.dataset.event),
        );
      });
    });
    highlightSelection();
  }

  function highlightSelection() {
    if (!state.selectedEvent) return;
    const target = `[data-measure="${state.selectedMeasure}"][data-voice="${state.selectedVoice}"][data-event="${state.selectedEvent}"]`;
    const hit = stage.querySelector(target);
    if (hit) {
      hit.setAttribute("fill", "rgba(227, 180, 95, 0.24)");
      hit.setAttribute("stroke", "#b8852e");
      hit.setAttribute("stroke-width", "1.2");
    }
  }

  function selectEvent(part, measure, voice, event) {
    if (Number.isInteger(part) && part >= 0) state.selectedPart = part;
    state.selectedMeasure = measure;
    state.selectedVoice = voice;
    state.selectedEvent = event;
    state.selectedNote = 0;
    renderMeasures();
    refreshSelection();
    highlightSelection();
  }

  async function refreshSelection() {
    if (state.selectedEvent === null) {
      updateSelectionUI();
      return;
    }
    try {
      const payload = await api(
        `/api/event?part=${state.selectedPart}&measure=${state.selectedMeasure}` +
          `&voice=${state.selectedVoice}&event=${state.selectedEvent}`,
      );
      state.selection = payload;
      updateSelectionUI();
    } catch (error) {
      state.selectedEvent = null;
      state.selection = null;
      updateSelectionUI();
      fail(error.message);
    }
  }

  function updateSelectionUI() {
    const selection = state.selection;
    const badge = $("#selectionBadge");
    const info = $("#selectionInfo");
    const hint = $("#eventHint");
    if (!selection) {
      badge.textContent = "none";
      badge.className = "badge";
      info.textContent = "Click a notehead or rest in the score.";
      hint.textContent = "no selection";
      $("#inspPitch").value = "";
      $("#inspLyric").value = "";
      $("#addChord").disabled = true;
      $("#deleteEvent").disabled = true;
      $("#applyPitch").disabled = true;
      $("#applyDuration").disabled = true;
      $("#applyDynamic").disabled = true;
      $("#applyLyric").disabled = true;
      return;
    }
    badge.textContent = selection.isRest ? "rest" : "note";
    badge.className = "badge ok";
    hint.textContent = `m${selection.measure + 1} · v${selection.voice} · event ${selection.event + 1}`;
    const notes = selection.notes
      .map((note) => `${note.pitch}${note.lyric ? ` “${note.lyric}”` : ""}`)
      .join(", ");
    info.textContent = selection.isRest
      ? `Rest · ${selection.duration}${selection.dots ? " dotted" : ""} · measure ${selection.measureNumber}`
      : `${notes} · ${selection.duration}${selection.dots ? " dotted" : ""} · measure ${selection.measureNumber}`;
    const noteSelect = $("#inspNote");
    noteSelect.innerHTML = "";
    selection.notes.forEach((note, index) => {
      const option = document.createElement("option");
      option.value = String(index);
      option.textContent = `#${index + 1} · ${note.pitch}`;
      noteSelect.append(option);
    });
    state.selectedNote = clamp(state.selectedNote, 0, Math.max(0, selection.notes.length - 1));
    noteSelect.value = String(state.selectedNote);
    const activeNote = selection.notes[state.selectedNote] || null;
    $("#inspPitch").value = activeNote ? activeNote.pitch : "";
    $("#inspDuration").value = selection.duration;
    $("#inspDots").value = String(selection.dots);
    $("#inspDynamic").value = selection.dynamic || "";
    $("#inspLyric").value = activeNote ? activeNote.lyric || "" : "";
    $("#keySelect").value = String(selection.fifths ?? 0);
    if (selection.tempo) $("#tempoInput").value = String(selection.tempo);
    $("#addChord").disabled = selection.isRest;
    $("#deleteEvent").disabled = false;
    $("#applyPitch").disabled = selection.isRest;
    $("#applyDuration").disabled = false;
    $("#applyDynamic").disabled = false;
    $("#applyLyric").disabled = selection.isRest;
  }

  async function selectPart(index) {
    try {
      await api("/api/select", {
        method: "POST",
        body: JSON.stringify({ part: index }),
      });
      state.selectedPart = index;
      state.selectedMeasure = 0;
      state.selectedEvent = null;
      state.selection = null;
      await refresh();
    } catch (error) {
      fail(error.message);
    }
  }

  function addNote(pitch) {
    edit({
      op: "add_note",
      part: state.selectedPart,
      measure: state.selectedMeasure,
      voice: Number($("#voice").value),
      pitch,
      duration: state.duration,
      dots: Number($("#dots").value),
    });
  }

  function bind() {
    $("#importBtn").addEventListener("click", () => $("#fileInput").click());
    $("#fileInput").addEventListener("change", async (event) => {
      const file = event.target.files && event.target.files[0];
      if (!file) return;
      try {
        await importFile(file);
      } finally {
        event.target.value = "";
      }
    });
    $("#undoBtn").addEventListener("click", async () => {
      try {
        await api("/api/undo", { method: "POST", body: "{}" });
        state.selectedEvent = null;
        state.selection = null;
        await refresh();
      } catch (error) {
        fail(error.message);
      }
    });
    $("#redoBtn").addEventListener("click", async () => {
      try {
        await api("/api/redo", { method: "POST", body: "{}" });
        state.selectedEvent = null;
        state.selection = null;
        await refresh();
      } catch (error) {
        fail(error.message);
      }
    });
    $("#resetBtn").addEventListener("click", async () => {
      try {
        await api("/api/reset", { method: "POST", body: "{}" });
        state.selectedPart = 0;
        state.selectedMeasure = 0;
        state.selectedEvent = null;
        state.selection = null;
        showToast("Demo score restored", "ok");
        await refresh();
      } catch (error) {
        fail(error.message);
      }
    });
    $("#octave").addEventListener("change", (event) => {
      state.octave = Number(event.target.value);
      renderOptions();
    });
    $("#addRest").addEventListener("click", () => {
      edit({
        op: "add_rest",
        part: state.selectedPart,
        measure: state.selectedMeasure,
        voice: Number($("#voice").value),
        duration: state.duration,
        dots: Number($("#dots").value),
      });
    });
    $("#inspNote").addEventListener("change", (event) => {
      state.selectedNote = Number(event.target.value);
      if (!state.selection) return;
      const note = state.selection.notes[state.selectedNote];
      if (note) {
        $("#inspPitch").value = note.pitch;
        $("#inspLyric").value = note.lyric || "";
      }
    });
    $("#applyPitch").addEventListener("click", () => {
      if (!state.selection) return;
      edit({
        op: "set_pitch",
        part: state.selectedPart,
        measure: state.selectedMeasure,
        voice: state.selectedVoice,
        event: state.selectedEvent,
        note: state.selectedNote,
        pitch: $("#inspPitch").value.trim(),
      });
    });
    $("#applyDuration").addEventListener("click", () => {
      if (!state.selection) return;
      edit({
        op: "set_duration",
        part: state.selectedPart,
        measure: state.selectedMeasure,
        voice: state.selectedVoice,
        event: state.selectedEvent,
        duration: $("#inspDuration").value,
        dots: Number($("#inspDots").value),
      });
    });
    $("#applyDynamic").addEventListener("click", () => {
      if (!state.selection) return;
      edit({
        op: "set_dynamic",
        part: state.selectedPart,
        measure: state.selectedMeasure,
        voice: state.selectedVoice,
        event: state.selectedEvent,
        dynamic: $("#inspDynamic").value,
      });
    });
    $("#applyLyric").addEventListener("click", () => {
      if (!state.selection) return;
      edit({
        op: "set_lyric",
        part: state.selectedPart,
        measure: state.selectedMeasure,
        voice: state.selectedVoice,
        event: state.selectedEvent,
        note: state.selectedNote,
        text: $("#inspLyric").value,
        syllabic: "single",
      });
    });
    $("#addChord").addEventListener("click", () => {
      if (!state.selection) return;
      edit({
        op: "add_chord_note",
        part: state.selectedPart,
        measure: state.selectedMeasure,
        voice: state.selectedVoice,
        event: state.selectedEvent,
        pitch: $("#inspPitch").value.trim(),
      });
    });
    $("#deleteEvent").addEventListener("click", () => {
      if (!state.selection) return;
      edit({
        op: "delete_event",
        part: state.selectedPart,
        measure: state.selectedMeasure,
        voice: state.selectedVoice,
        event: state.selectedEvent,
      });
      state.selectedEvent = null;
      state.selection = null;
    });
    $("#zoomOut").addEventListener("click", () => applyZoom(-1));
    $("#zoomIn").addEventListener("click", () => applyZoom(1));
    $("#applyTitle").addEventListener("click", () => {
      edit({ op: "set_title", title: $("#titleInput").value.trim() || "Untitled" });
    });
    $("#applyMetadata").addEventListener("click", async () => {
      if (state.busy) return;
      state.busy = true;
      try {
        await api("/api/edit", {
          method: "POST",
          body: JSON.stringify({
            op: "set_metadata",
            field: "subtitle",
            value: $("#subtitleInput").value.trim(),
          }),
        });
        await api("/api/edit", {
          method: "POST",
          body: JSON.stringify({
            op: "set_metadata",
            field: "composer",
            value: $("#composerInput").value.trim(),
          }),
        });
        await refresh();
      } catch (error) {
        fail(error.message);
      } finally {
        state.busy = false;
      }
    });
    $("#applyTempo").addEventListener("click", () => {
      edit({
        op: "set_tempo",
        part: state.selectedPart,
        measure: state.selectedMeasure,
        bpm: Number($("#tempoInput").value),
      });
    });
    $("#applyKey").addEventListener("click", () => {
      edit({
        op: "set_key",
        part: state.selectedPart,
        measure: state.selectedMeasure,
        fifths: Number($("#keySelect").value),
      });
    });
    $("#applyTranspose").addEventListener("click", () => {
      edit({
        op: "transpose",
        part: state.selectedPart,
        semitones: Number($("#transposeSelect").value),
      });
    });
    $("#addPart").addEventListener("click", () => {
      const name = window.prompt("Part name", `Part ${(state.summary?.parts.length || 0) + 1}`);
      if (name === null) return;
      edit({ op: "add_part", name: name.trim() || "New part" });
    });
    $("#deletePart").addEventListener("click", () => {
      if (!window.confirm("Delete the selected part and all of its measures?")) return;
      edit({ op: "delete_part", part: state.selectedPart });
    });
    $("#applyTransposeAll").addEventListener("click", () => {
      edit({
        op: "transpose_all",
        semitones: Number($("#transposeSelect").value),
      });
    });
    $("#addMeasure").addEventListener("click", () => {
      edit({ op: "add_measure", part: state.selectedPart });
    });
    $("#duplicateMeasure").addEventListener("click", () => {
      edit({
        op: "duplicate_measure",
        part: state.selectedPart,
        measure: state.selectedMeasure,
      });
    });
    $("#deleteMeasure").addEventListener("click", () => {
      edit({
        op: "delete_measure",
        part: state.selectedPart,
        measure: state.selectedMeasure,
      });
      state.selectedMeasure = 0;
      state.selectedEvent = null;
      state.selection = null;
    });
    $$("[data-export]").forEach((button) => {
      button.addEventListener("click", () => download(button.dataset.export));
    });
    stage.addEventListener("dragover", (event) => {
      if (event.dataTransfer && Array.from(event.dataTransfer.types).includes("Files")) {
        event.preventDefault();
        stage.classList.add("dragover");
      }
    });
    stage.addEventListener("dragleave", () => stage.classList.remove("dragover"));
    stage.addEventListener("drop", async (event) => {
      event.preventDefault();
      stage.classList.remove("dragover");
      const file = event.dataTransfer && event.dataTransfer.files && event.dataTransfer.files[0];
      if (!file) return;
      await importFile(file);
    });
  }

  async function importFile(file) {
    try {
      const text = await file.text();
      await api("/api/import", {
        method: "POST",
        body: text,
        contentType: "application/xml",
      });
      state.selectedMeasure = 0;
      state.selectedEvent = null;
      state.selection = null;
      showToast(`Imported ${file.name}`, "ok");
      await refresh();
    } catch (error) {
      fail(error.message);
    }
  }

  async function download(kind) {
    try {
      const part = encodeURIComponent(state.selectedPart);
      const response = await fetch(`/api/export/${kind}?part=${part}`, {
        credentials: "same-origin",
        cache: "no-store",
      });
      if (!response.ok) throw new Error(`export failed (${response.status})`);
      const blob = await response.blob();
      const disposition = response.headers.get("Content-Disposition") || "";
      let filename = `cadence.${kind === "musicxml" ? "musicxml" : kind}`;
      const match = disposition.match(/filename="([^"]+)"/);
      if (match) filename = match[1];
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = filename;
      document.body.append(anchor);
      anchor.click();
      anchor.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 4000);
      showToast(`Exported ${filename}`, "ok");
    } catch (error) {
      fail(error.message);
    }
  }

  function scheduleRefresh() {
    window.clearTimeout(state.refreshTimer);
    state.refreshTimer = window.setTimeout(() => {
      refresh().catch((error) => fail(error.message));
    }, 90);
  }

  function installShortcuts() {
    document.addEventListener("keydown", (event) => {
      const target = event.target;
      if (target && target.closest && target.closest("input, select, textarea")) return;
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
        event.preventDefault();
        if (event.shiftKey) {
          $("#redoBtn").click();
        } else {
          $("#undoBtn").click();
        }
        return;
      }
      if (event.ctrlKey && event.key.toLowerCase() === "y") {
        event.preventDefault();
        $("#redoBtn").click();
        return;
      }
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (/^[1-7]$/.test(event.key)) {
        const index = Number(event.key) - 1;
        const chip = $$("#durations .chip")[index];
        if (chip) {
          chip.click();
          event.preventDefault();
        }
        return;
      }
      if (event.key.length === 1 && PITCHES.includes(event.key.toUpperCase())) {
        addNote(`${event.key.toUpperCase()}${state.octave}`);
        event.preventDefault();
        return;
      }
      if (event.key === "Delete" || event.key === "Backspace") {
        if (state.selection) {
          $("#deleteEvent").click();
          event.preventDefault();
        }
        return;
      }
      if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
        const delta = event.key === "ArrowLeft" ? -1 : 1;
        const part = selectedPartInfo();
        if (!part) return;
        state.selectedMeasure = clamp(state.selectedMeasure + delta, 0, part.measures - 1);
        state.selectedEvent = null;
        state.selection = null;
        renderMeasures();
        updateSelectionUI();
        event.preventDefault();
      }
    });
  }

  function connectEvents() {
    try {
      const source = new EventSource("/api/events");
      source.addEventListener("open", () => setConnection("ok"));
      source.addEventListener("update", () => scheduleRefresh());
      source.addEventListener("error", () => setConnection("error"));
      window.addEventListener("beforeunload", () => source.close());
    } catch (_) {
      setConnection("error");
    }
  }

  renderOptions();
  bind();
  installShortcuts();
  stage.innerHTML = `<div class="empty-state"><span class="spinner" aria-hidden="true"></span><p>Engraving…</p></div>`;
  refresh().then(connectEvents);
  setConnection("connecting");
})();
