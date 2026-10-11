<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    dismissToast,
    getState,
    setAutostart,
    setSettings,
    snooze,
    type ConfigInput,
    type ShutdState,
  } from './lib/backend';
  import { dayLabel, formatClock, humanRemaining } from './lib/countdown';

  let state = $state<ShutdState | null>(null);
  let screen = $state<'status' | 'settings'>('status');
  let now = $state(Date.now());
  let form = $state<ConfigInput | null>(null);
  let autostart = $state(false);

  const shutdownAt = $derived(state ? new Date(state.shutdownAt) : null);
  const remaining = $derived(shutdownAt ? shutdownAt.getTime() - now : 0);
  const toast = $derived(state?.toast ?? null);
  const toastLeft = $derived(toast ? new Date(toast.dismissAt).getTime() - now : 0);
  // The toast window loads the same bundle with ?view=toast; the panel is
  // everything else.
  const isToast = new URLSearchParams(window.location.search).get('view') === 'toast';

  onMount(() => {
    let unlisten: (() => void) | undefined;
    listen<ShutdState>('state-changed', (event) => applyState(event.payload)).then((u) => {
      unlisten = u;
    });
    getState().then(applyState).catch(console.error);
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => {
      clearInterval(timer);
      unlisten?.();
    };
  });

  function applyState(next: ShutdState) {
    state = next;
    autostart = next.settings.autostart;
    form ??= {
      startTime: next.settings.startTime,
      snoozeInterval: next.settings.snoozeInterval,
      notification: { before: next.settings.before, duration: next.settings.duration },
    };
  }

  async function onSnooze() {
    try {
      applyState(await snooze());
    } catch (error) {
      console.error(error);
    }
  }

  async function onDismiss() {
    try {
      applyState(await dismissToast());
    } catch (error) {
      console.error(error);
    }
  }

  function saveField(field: 'startTime' | 'snoozeInterval' | 'before' | 'duration', value: string) {
    if (!form) return;
    const next: ConfigInput = { ...form, notification: { ...form.notification } };
    if (field === 'startTime') next.startTime = value;
    else if (field === 'snoozeInterval') next.snoozeInterval = Number(value);
    else if (field === 'before') next.notification.before = Number(value);
    else next.notification.duration = Number(value);
    form = next;
    setSettings(next).then(applyState).catch((error) => console.error('save failed', error));
  }

  function toggleAutostart() {
    const next = !autostart;
    autostart = next;
    setAutostart(next).catch((error) => {
      autostart = !next;
      console.error(error);
    });
  }

  // The toast countdown hitting zero dismisses the toast; the shutdown
  // then proceeds, matching the Go version's timeout behavior.
  $effect(() => {
    if (toast && toastLeft <= 0) void onDismiss();
  });
</script>

{#if isToast}
  {#if toast && shutdownAt && state}
    <main class="toast">
      <div class="toast-top">
        <span class="glyph"><PowerIcon /></span>
        <div>
          <div class="toast-title">Shutdown at {formatClock(shutdownAt)}</div>
          <div class="toast-sub">in {humanRemaining(remaining)} &middot; snooze for {state.settings.snoozeInterval} min?</div>
        </div>
      </div>
      <div class="meter">
        <i
          class:critical={toastLeft < 60_000}
          style={`width:${Math.min(100, (toastLeft / (state.settings.duration * 60_000)) * 100)}%`}
        ></i>
      </div>
      <div class="btn-row">
        <button class="btn ghost" onclick={onDismiss}>Dismiss</button>
        <button class="btn primary" onclick={onSnooze}>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M12 5v7" /><path d="M6.3 7.3a7 7 0 1 0 11.4 0" /></svg>
          Snooze +{state.settings.snoozeInterval} min
        </button>
      </div>
    </main>
  {/if}
{:else if screen === 'settings' && form}
  <main class="panel">
    <header>
      <button class="icon-btn" aria-label="Back" onclick={() => (screen = 'status')}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 18l-6-6 6-6" /></svg>
      </button>
      <h3>Settings</h3>
    </header>
    <div class="card">
      <div class="row">
        <label for="start-time">Shutdown time</label>
        <input id="start-time" type="time" value={form.startTime} onchange={(e) => saveField('startTime', e.currentTarget.value)} />
      </div>
    </div>
    <div class="card">
      <div class="row">
        <label for="snooze-interval">Snooze interval</label>
        <select id="snooze-interval" value={form.snoozeInterval} onchange={(e) => saveField('snoozeInterval', e.currentTarget.value)}>
          {#each [5, 10, 15, 30, 60] as minutes}<option value={minutes}>{minutes} min</option>{/each}
        </select>
      </div>
      <div class="row">
        <label for="notify-before">Notify before shutdown</label>
        <select id="notify-before" value={form.notification.before} onchange={(e) => saveField('before', e.currentTarget.value)}>
          {#each [5, 10, 15, 30] as minutes}<option value={minutes}>{minutes} min</option>{/each}
        </select>
      </div>
      <div class="row">
        <label for="toast-duration">Toast auto-dismiss</label>
        <select id="toast-duration" value={form.notification.duration} onchange={(e) => saveField('duration', e.currentTarget.value)}>
          {#each [5, 10, 15] as minutes}<option value={minutes}>{minutes} min</option>{/each}
        </select>
      </div>
    </div>
    <div class="card">
      <div class="row">
        <label for="autostart">Launch at startup</label>
        <input id="autostart" type="checkbox" checked={autostart} onchange={toggleAutostart} />
      </div>
    </div>
    <p class="note">Changes apply immediately and hot-reload the scheduler.</p>
  </main>
{:else if shutdownAt && state}
  <main class="panel">
    <header>
      <h3>Shutd</h3>
      <span class="spacer"></span>
      <button class="icon-btn" aria-label="Settings" onclick={() => (screen = 'settings')}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.01a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h.01a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.01a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg>
      </button>
    </header>
    <div class="label">Next shutdown</div>
    <div class="time">{formatClock(shutdownAt)}</div>
    <div class="sub">
      {dayLabel(shutdownAt, new Date(now))} &middot; in {humanRemaining(remaining)}{#if state.snoozed > 0}&middot; snoozed +{state.settings.snoozeInterval} min{/if}
    </div>
    <div class="card">
      {#if state.snoozed > 0}
        <div class="row space">
          <span class="row-label">Snoozed</span>
          <span class="value">{state.snoozed}&times; +{state.settings.snoozeInterval} min</span>
        </div>
      {:else}
        <div class="row space">
          <span class="row-label">Snooze interval</span>
          <span class="value">{state.settings.snoozeInterval} min</span>
        </div>
      {/if}
      <div class="row space">
        <span class="row-label">Notification</span>
        <span class="value">{formatClock(new Date(shutdownAt.getTime() - state.settings.before * 60_000))} &middot; {state.settings.before} min before</span>
      </div>
    </div>
    <button class="btn primary block" onclick={onSnooze}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M12 5v7" /><path d="M6.3 7.3a7 7 0 1 0 11.4 0" /></svg>
      Snooze +{state.settings.snoozeInterval} min
    </button>
  </main>
{/if}

{#snippet PowerIcon()}
  <svg width="16" height="16" viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M16 3.5V16" /><path d="M25.2 9.1a11.5 11.5 0 1 1-18.4 0" /></svg>
{/snippet}

<style>
  main {
    height: 100%;
    padding: 14px;
    display: flex;
    flex-direction: column;
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }

  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .spacer {
    flex: 1;
  }

  .icon-btn {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .icon-btn:hover {
    background: var(--card);
    color: var(--ink);
  }

  .label {
    font-size: 12px;
    color: var(--muted);
  }

  .time {
    font-size: 36px;
    font-weight: 600;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }

  .sub {
    font-size: 13px;
    color: var(--muted);
    margin-bottom: 12px;
  }

  .card {
    background: var(--card);
    border-radius: 12px;
    padding: 10px 14px;
    margin-bottom: 8px;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .row + .row {
    margin-top: 10px;
  }

  .row-label {
    font-size: 13px;
    font-weight: 600;
  }

  .value {
    font-size: 12px;
    color: var(--muted);
  }

  .meter {
    height: 5px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--ink) 14%, var(--tray));
    margin-top: 6px;
    overflow: hidden;
  }

  .meter i {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: var(--accent);
  }

  .toast .meter i {
    background: var(--warning);
  }

  .toast .meter i.critical {
    background: var(--critical);
  }

  .btn {
    display: inline-flex;
    justify-content: center;
    align-items: center;
    gap: 8px;
    padding: 9px 14px;
    border: 0;
    border-radius: 8px;
    font-weight: 600;
    cursor: pointer;
  }

  .btn.primary {
    background: var(--accent);
    color: #fff;
  }

  .btn-row .btn.primary {
    flex: 1;
  }

  .btn.primary:hover {
    filter: brightness(1.08);
  }

  .btn.ghost {
    background: var(--card);
    color: var(--ink);
    font-weight: 400;
  }

  .btn.ghost:hover {
    background: var(--button-hover);
  }

  .btn.block {
    width: 100%;
    margin-top: 4px;
  }

  .btn-row {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }

  label {
    font-size: 13px;
  }

  select,
  input[type='time'] {
    color: var(--ink);
    background: var(--tray);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 5px 8px;
  }

  input[type='checkbox'] {
    accent-color: var(--accent);
    width: 16px;
    height: 16px;
  }

  .note {
    font-size: 12px;
    color: var(--tertiary);
    margin: 10px 2px 0;
  }

  .toast-top {
    display: flex;
    gap: 10px;
    margin-bottom: 10px;
  }

  .glyph {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: var(--card);
    flex: none;
  }

  .toast-title {
    font-size: 13px;
    font-weight: 600;
  }

  .toast-sub {
    font-size: 12px;
    color: var(--muted);
    margin-top: 1px;
  }

</style>
