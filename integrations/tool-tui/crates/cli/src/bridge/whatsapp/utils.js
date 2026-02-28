export function normalizeE164(number) {
  const withoutPrefix = number.replace(/^whatsapp:/i, "").trim();
  const digits = withoutPrefix.replace(/[^\d+]/g, "");
  if (!digits) {
    return "+";
  }
  if (digits.startsWith("+")) {
    return `+${digits.slice(1)}`;
  }
  return `+${digits}`;
}
