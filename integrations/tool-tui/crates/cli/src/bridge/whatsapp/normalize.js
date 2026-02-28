import { normalizeE164 } from './utils.js';

const WHATSAPP_USER_JID_RE = /^(\d+)(?::\d+)?@s\.whatsapp\.net$/i;
const WHATSAPP_LID_RE = /^(\d+)@lid$/i;

function stripWhatsAppTargetPrefixes(value) {
  let candidate = String(value ?? '').trim();
  for (;;) {
    const before = candidate;
    candidate = candidate.replace(/^whatsapp:/i, '').trim();
    if (candidate === before) {
      return candidate;
    }
  }
}

export function isWhatsAppGroupJid(value) {
  const candidate = stripWhatsAppTargetPrefixes(value);
  const lower = candidate.toLowerCase();
  if (!lower.endsWith('@g.us')) {
    return false;
  }
  const localPart = candidate.slice(0, candidate.length - '@g.us'.length);
  if (!localPart || localPart.includes('@')) {
    return false;
  }
  return /^[0-9]+(-[0-9]+)*$/.test(localPart);
}

export function isWhatsAppUserTarget(value) {
  const candidate = stripWhatsAppTargetPrefixes(value);
  return WHATSAPP_USER_JID_RE.test(candidate) || WHATSAPP_LID_RE.test(candidate);
}

function extractUserJidPhone(jid) {
  const userMatch = jid.match(WHATSAPP_USER_JID_RE);
  if (userMatch) {
    return userMatch[1];
  }
  const lidMatch = jid.match(WHATSAPP_LID_RE);
  if (lidMatch) {
    return lidMatch[1];
  }
  return null;
}

export function normalizeWhatsAppTarget(value) {
  const candidate = stripWhatsAppTargetPrefixes(value);
  if (!candidate) {
    return null;
  }

  if (isWhatsAppGroupJid(candidate)) {
    const localPart = candidate.slice(0, candidate.length - '@g.us'.length);
    return `${localPart}@g.us`;
  }

  if (isWhatsAppUserTarget(candidate)) {
    const phone = extractUserJidPhone(candidate);
    if (!phone) {
      return null;
    }
    const normalized = normalizeE164(phone);
    return normalized.length > 1 ? normalized : null;
  }

  if (candidate.includes('@')) {
    return null;
  }

  const normalized = normalizeE164(candidate);
  return normalized.length > 1 ? normalized : null;
}

export function normalizeWhatsAppToJid(value) {
  const normalized = normalizeWhatsAppTarget(value);
  if (!normalized) {
    return null;
  }

  if (isWhatsAppGroupJid(normalized)) {
    return normalized;
  }

  return `${normalized.replace(/^\+/, '')}@s.whatsapp.net`;
}
