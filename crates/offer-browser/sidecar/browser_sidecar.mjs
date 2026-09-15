#!/usr/bin/env node
/**
 * SwissArmyNoife browser.session Playwright sidecar (ADR 013 / sak596-e+).
 * One JSON object per stdin line → one JSON object per stdout line.
 */
import { createInterface } from "node:readline";
import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { randomUUID } from "node:crypto";

const MAX_EVENTS = 200;
const MAX_TEXT = 64 * 1024;

/** @type {Map<string, Session>} */
const sessions = new Map();
let browser = null;

/**
 * @typedef {{
 *   context: import('playwright').BrowserContext,
 *   pages: import('playwright').Page[],
 *   active: number,
 *   locked: boolean,
 *   refs: Map<string, import('playwright').Locator>,
 *   consoleBuf: object[],
 *   networkBuf: object[],
 * }} Session
 */

async function ensureBrowser() {
  if (!browser) {
    browser = await chromium.launch({ headless: true });
  }
  return browser;
}

function pushEvent(buf, ev) {
  buf.push(ev);
  while (buf.length > MAX_EVENTS) buf.shift();
  let total = 0;
  for (let i = buf.length - 1; i >= 0; i--) {
    total += JSON.stringify(buf[i]).length;
    if (total > MAX_TEXT) {
      buf.splice(0, i);
      break;
    }
  }
}

function redact(s) {
  return String(s || "")
    .replace(/(authorization\s*[:=]\s*)\S+/gi, "$1[REDACTED]")
    .replace(/(cookie\s*[:=]\s*)\S+/gi, "$1[REDACTED]")
    .replace(/Bearer\s+\S+/gi, "Bearer [REDACTED]");
}

function attachListeners(session, page) {
  page.on("console", (msg) => {
    pushEvent(session.consoleBuf, {
      type: msg.type(),
      text: redact(msg.text()).slice(0, 4000),
      ts: Date.now(),
    });
  });
  page.on("pageerror", (err) => {
    pushEvent(session.consoleBuf, {
      type: "pageerror",
      text: redact(err.message || String(err)).slice(0, 4000),
      ts: Date.now(),
    });
  });
  page.on("requestfailed", (req) => {
    pushEvent(session.networkBuf, {
      url: redact(req.url()).slice(0, 2000),
      method: req.method(),
      failure: req.failure()?.errorText || "failed",
      ok: false,
      ts: Date.now(),
    });
  });
  page.on("response", (res) => {
    if (res.status() >= 400) {
      pushEvent(session.networkBuf, {
        url: redact(res.url()).slice(0, 2000),
        method: res.request().method(),
        status: res.status(),
        ok: false,
        ts: Date.now(),
      });
    }
  });
}

async function getSession(profile) {
  let s = sessions.get(profile);
  if (s) return s;
  mkdirSync(profile, { recursive: true });
  mkdirSync(join(profile, "shots"), { recursive: true });
  const b = await ensureBrowser();
  const context = await b.newContext({ acceptDownloads: false });
  const page = await context.newPage();
  s = {
    context,
    pages: [page],
    active: 0,
    locked: false,
    refs: new Map(),
    consoleBuf: [],
    networkBuf: [],
  };
  attachListeners(s, page);
  sessions.set(profile, s);
  return s;
}

function activePage(s) {
  return s.pages[s.active] || s.pages[0];
}

function requireUnlocked(s) {
  if (s.locked) {
    const e = new Error("browser locked");
    e.code = "policy.denied";
    throw e;
  }
}

async function buildSnapshot(s) {
  const page = activePage(s);
  s.refs.clear();
  let n = 1;
  let snapshot = "";
  try {
    const ax = await page.accessibility.snapshot();
    const walk = (node, depth) => {
      if (!node) return;
      const ref = `e${n++}`;
      const role = node.role || "generic";
      const name = node.name || "";
      const pad = "  ".repeat(depth);
      snapshot += `${pad}- ${role}${name ? ` ${JSON.stringify(name)}` : ""} [${ref}]\n`;
      // Prefer role+name locator; fall back to text.
      let locator = page.getByRole(role, name ? { name } : undefined);
      s.refs.set(ref, locator.first());
      for (const child of node.children || []) walk(child, depth + 1);
    };
    walk(ax, 0);
  } catch {
    const body = page.locator("body");
    const ref = `e${n++}`;
    s.refs.set(ref, body);
    const text = (await body.innerText().catch(() => "")).slice(0, 8000);
    snapshot = `- document [${ref}]\n  - text ${JSON.stringify(text)}\n`;
  }
  return {
    url: page.url(),
    title: await page.title(),
    snapshot,
  };
}

function resolveRef(s, ref) {
  const loc = s.refs.get(ref);
  if (!loc) {
    const e = new Error(`unknown ref: ${ref}`);
    e.code = "schema.invalid";
    throw e;
  }
  return loc;
}

async function handle(msg) {
  const op = msg.op;
  const profile = msg.profile || "";
  if (op === "ping") return { ok: true };

  const s = await getSession(profile);

  if (op === "navigate") {
    requireUnlocked(s);
    const page = activePage(s);
    await page.goto(String(msg.url), { waitUntil: "domcontentloaded", timeout: 30000 });
    return { ok: true, url: page.url(), title: await page.title() };
  }
  if (op === "snapshot") {
    const snap = await buildSnapshot(s);
    return { ok: true, ...snap };
  }
  if (op === "click") {
    requireUnlocked(s);
    await resolveRef(s, msg.ref).click({ timeout: 15000 });
    return { ok: true, op };
  }
  if (op === "type") {
    requireUnlocked(s);
    await resolveRef(s, msg.ref).pressSequentially(String(msg.text || ""), { timeout: 15000 });
    return { ok: true, op };
  }
  if (op === "fill") {
    requireUnlocked(s);
    await resolveRef(s, msg.ref).fill(String(msg.text || ""), { timeout: 15000 });
    return { ok: true, op };
  }
  if (op === "press_key") {
    requireUnlocked(s);
    const page = activePage(s);
    if (msg.ref) await resolveRef(s, msg.ref).press(String(msg.key || "Enter"));
    else await page.keyboard.press(String(msg.key || "Enter"));
    return { ok: true, op };
  }
  if (op === "scroll") {
    requireUnlocked(s);
    const page = activePage(s);
    if (msg.ref) {
      await resolveRef(s, msg.ref).scrollIntoViewIfNeeded();
    } else {
      await page.mouse.wheel(Number(msg.delta_x || 0), Number(msg.delta_y || 300));
    }
    return { ok: true, op };
  }
  if (op === "select_option") {
    requireUnlocked(s);
    const values = Array.isArray(msg.values) ? msg.values : [msg.value || msg.text].filter(Boolean);
    await resolveRef(s, msg.ref).selectOption(values);
    return { ok: true, op };
  }
  if (op === "drag") {
    requireUnlocked(s);
    const start = resolveRef(s, msg.ref);
    const end = resolveRef(s, msg.target_ref);
    await start.dragTo(end);
    return { ok: true, op };
  }
  if (op === "mouse_click_xy") {
    requireUnlocked(s);
    const page = activePage(s);
    await page.mouse.click(Number(msg.x), Number(msg.y));
    return { ok: true, op };
  }
  if (op === "highlight") {
    const loc = resolveRef(s, msg.ref);
    await loc.evaluate((el) => {
      el.style.outline = "3px solid #f90";
    });
    return { ok: true, op };
  }
  if (op === "get_bounding_box") {
    const box = await resolveRef(s, msg.ref).boundingBox();
    if (!box) return { ok: false, error: "no bounding box" };
    return { ok: true, ref: msg.ref, ...box };
  }
  if (op === "take_screenshot") {
    const page = activePage(s);
    const path = join(profile, "shots", `${randomUUID()}.png`);
    await page.screenshot({ path, fullPage: !!msg.full_page });
    return { ok: true, path, bytes: 0 };
  }
  if (op === "tabs") {
    const action = msg.tabs_action || msg.action || "list";
    if (action === "list") {
      const tabs = [];
      for (const p of s.pages) {
        tabs.push({ url: p.url(), title: await p.title() });
      }
      return { ok: true, tabs, active: s.active };
    }
    if (action === "new") {
      requireUnlocked(s);
      const page = await s.context.newPage();
      attachListeners(s, page);
      s.pages.push(page);
      s.active = s.pages.length - 1;
      if (msg.url) await page.goto(String(msg.url), { waitUntil: "domcontentloaded" });
      return { ok: true, active: s.active };
    }
    if (action === "select") {
      const idx = Number(msg.index);
      if (idx < 0 || idx >= s.pages.length) return { ok: false, error: "bad index" };
      s.active = idx;
      return { ok: true, active: s.active };
    }
    if (action === "close") {
      requireUnlocked(s);
      if (s.pages.length <= 1) return { ok: false, error: "cannot close last tab" };
      const idx = msg.index == null ? s.active : Number(msg.index);
      const page = s.pages[idx];
      await page.close().catch(() => {});
      s.pages.splice(idx, 1);
      if (s.active >= s.pages.length) s.active = s.pages.length - 1;
      return { ok: true, active: s.active };
    }
    return { ok: false, error: `unknown tabs action: ${action}` };
  }
  if (op === "lock") {
    const action = msg.lock_action || msg.action || "lock";
    s.locked = action !== "unlock";
    return { ok: true, locked: s.locked };
  }
  if (op === "console") {
    const limit = Number(msg.limit || 50);
    return { ok: true, events: s.consoleBuf.slice(-limit).reverse() };
  }
  if (op === "network") {
    const limit = Number(msg.limit || 50);
    return { ok: true, events: s.networkBuf.slice(-limit).reverse() };
  }
  if (op === "failure_report") {
    const page = activePage(s);
    const path = join(profile, "shots", `failure-${randomUUID()}.png`);
    await page.screenshot({ path, fullPage: true }).catch(() => {});
    const snap = await buildSnapshot(s);
    return {
      ok: true,
      url: page.url(),
      title: await page.title(),
      step: msg.step ?? null,
      screenshot_path: path,
      snapshot_excerpt: String(snap.snapshot || "").slice(0, 2000),
      console: s.consoleBuf.slice(-20).reverse(),
      network: s.networkBuf.slice(-20).reverse(),
      ts: Date.now(),
    };
  }
  if (op === "cdp") {
    const method = String(msg.method || "");
    if (method.startsWith("Input.")) {
      return { ok: false, error: "policy.denied: Input.* CDP methods are forbidden" };
    }
    const page = activePage(s);
    const session = await s.context.newCDPSession(page);
    const result = await session.send(method, msg.params || {});
    return { ok: true, method, result };
  }
  if (op === "close") {
    const cur = sessions.get(profile);
    if (cur) {
      await cur.context.close().catch(() => {});
      sessions.delete(profile);
    }
    return { ok: true };
  }
  return { ok: false, error: `unknown op: ${op}` };
}

const rl = createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on("line", async (line) => {
  let msg;
  try {
    msg = JSON.parse(line);
  } catch (e) {
    process.stdout.write(JSON.stringify({ ok: false, error: String(e) }) + "\n");
    return;
  }
  try {
    const resp = await handle(msg);
    process.stdout.write(JSON.stringify(resp) + "\n");
  } catch (e) {
    process.stdout.write(
      JSON.stringify({ ok: false, error: e.message || String(e), code: e.code }) + "\n"
    );
  }
});
