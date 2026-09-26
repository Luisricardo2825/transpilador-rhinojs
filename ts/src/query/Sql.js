export const DEFAULT_LIMIT = 100;

export function normalize(query) {
  return query.trim().replace(/\s+/g, " ");
}

export function addLimit(query, limit = DEFAULT_LIMIT) {
  const normalized = normalize(query);
  const maxRows = Math.max(1, Number(limit || DEFAULT_LIMIT));

  return normalized.toLowerCase().includes(" limit ")
    ? normalized
    : `${normalized} LIMIT ${maxRows}`;
}