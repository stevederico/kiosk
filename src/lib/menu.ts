/**
 * Menu loading for the kiosk page (`GET /menu_items`).
 *
 * The endpoint returns Rails' `MenuItem.all.to_json`; `price` is a decimal
 * string such as `"1.5"`, shown on the tile exactly as served.
 */

/** One row of `menu_items` as the API returns it. */
export interface MenuItem {
  id: number;
  name: string;
  category: string | null;
  created_at: string;
  updated_at: string;
  price: string;
}

/** Path of the menu endpoint (same origin; Vite proxies it in development). */
export const MENU_PATH = '/menu_items';

function isMenuItem(value: unknown): value is MenuItem {
  if (typeof value !== 'object' || value === null) return false;
  const record: Record<string, unknown> = { ...value };
  return typeof record.id === 'number'
    && typeof record.name === 'string'
    && (record.category === null || typeof record.category === 'string')
    && typeof record.created_at === 'string'
    && typeof record.updated_at === 'string'
    && typeof record.price === 'string';
}

/**
 * Validate a decoded `/menu_items` body.
 *
 * @throws {Error} When the body is not an array of menu items
 */
export function parseMenu(body: unknown): MenuItem[] {
  if (!Array.isArray(body) || !body.every(isMenuItem)) {
    throw new Error('The menu response was not in the expected format.');
  }
  return body;
}

/**
 * Fetch and validate the menu.
 *
 * @param fetchFn - `fetch`, injectable for tests
 * @throws {Error} On a network failure, a non-2xx status, or a malformed body
 */
export async function fetchMenu(fetchFn: typeof fetch = fetch): Promise<MenuItem[]> {
  const response = await fetchFn(MENU_PATH, { headers: { Accept: 'application/json' } });
  if (!response.ok) {
    throw new Error(`The menu could not be loaded (HTTP ${response.status}).`);
  }
  const body: unknown = await response.json();
  return parseMenu(body);
}
