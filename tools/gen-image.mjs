#!/usr/bin/env node
/**
 * Generate an image with Gemini (Nano Banana) via the REST API.
 * Reads GEMINI_API_KEY from the environment or ~/.gemini/.env. Never prints it.
 *
 * Usage:
 *   node tools/gen-image.mjs --out art/rock.png [--aspect 1:1] [--model <id>] "prompt text"
 */
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';

const MODELS = ['gemini-3.1-flash-image-preview', 'gemini-2.5-flash-image'];

function loadKey() {
  if (process.env.GEMINI_API_KEY) return process.env.GEMINI_API_KEY.trim();
  const envPath = join(homedir(), '.gemini', '.env');
  if (existsSync(envPath)) {
    for (const line of readFileSync(envPath, 'utf8').split(/\r?\n/)) {
      const m = line.match(/^\s*GEMINI_API_KEY\s*=\s*"?([^"\s]+)"?\s*$/);
      if (m) return m[1];
    }
  }
  throw new Error('GEMINI_API_KEY not found in environment or ~/.gemini/.env');
}

function parseArgs(argv) {
  const opts = { out: null, aspect: null, model: null, prompt: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--out') opts.out = argv[++i];
    else if (a === '--aspect') opts.aspect = argv[++i];
    else if (a === '--model') opts.model = argv[++i];
    else opts.prompt.push(a);
  }
  opts.prompt = opts.prompt.join(' ').trim();
  if (!opts.out || !opts.prompt) {
    console.error('usage: gen-image.mjs --out <file.png> [--aspect 1:1] [--model id] "prompt"');
    process.exit(2);
  }
  return opts;
}

async function generate(model, key, prompt, aspect) {
  const url = `https://generativelanguage.googleapis.com/v1beta/models/${model}:generateContent`;
  const body = {
    contents: [{ parts: [{ text: prompt }] }],
    generationConfig: {
      responseModalities: ['IMAGE'],
      ...(aspect ? { imageConfig: { aspectRatio: aspect } } : {}),
    },
  };
  const res = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'x-goog-api-key': key },
    body: JSON.stringify(body),
  });
  const text = await res.text();
  if (!res.ok) throw new Error(`${model}: HTTP ${res.status}: ${text.slice(0, 400)}`);
  const json = JSON.parse(text);
  const parts = json.candidates?.[0]?.content?.parts ?? [];
  const img = parts.find((p) => p.inlineData?.data);
  if (!img) throw new Error(`${model}: no image in response: ${text.slice(0, 400)}`);
  return { buffer: Buffer.from(img.inlineData.data, 'base64'), mime: img.inlineData.mimeType };
}

const opts = parseArgs(process.argv.slice(2));
const key = loadKey();
const models = opts.model ? [opts.model] : MODELS;
let lastErr = null;
for (const model of models) {
  try {
    const { buffer, mime } = await generate(model, key, opts.prompt, opts.aspect);
    mkdirSync(dirname(opts.out), { recursive: true });
    writeFileSync(opts.out, buffer);
    console.log(`ok ${model} -> ${opts.out} (${buffer.length} bytes, ${mime})`);
    process.exit(0);
  } catch (err) {
    lastErr = err;
    console.error(`failed: ${err.message}`);
  }
}
console.error(lastErr?.message ?? 'no model succeeded');
process.exit(1);
