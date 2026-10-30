/**
 * Kiosk cart logic, ported from the Rails app's `public/kiosk.html` script.
 *
 * The running `total` is kept the same way the original page kept it: adjusted
 * on every add/remove/edit rather than recomputed, and persisted next to the
 * cart under the same `localStorage` keys (`cart`, `total`). A browser that
 * used the Rails kiosk restores its cart unchanged after the switch.
 */

/** One line on the receipt. `price` is the parsed menu price. */
export interface CartItem {
  id: number;
  name: string;
  price: number;
  qty: number;
}

/** Cart lines plus the running subtotal. */
export interface CartState {
  items: CartItem[];
  total: number;
}

/** Minimal `Storage` surface, so tests can pass an in-memory stand-in. */
export interface CartStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

/** Sales tax applied at checkout (8%). */
export const TAX_RATE = 0.08;

/** `localStorage` keys used by the Rails kiosk page. */
export const CART_KEY = 'cart';
export const TOTAL_KEY = 'total';

export const EMPTY_CART: CartState = { items: [], total: 0 };

/**
 * Add one of a menu item. A line with the same id gains a unit; otherwise a
 * new line is appended.
 *
 * @param price - Menu price as served (`"1.5"`), parsed like `parseFloat`
 */
export function addItem(state: CartState, id: number, name: string, price: string | number): CartState {
  const unit = typeof price === 'number' ? price : parseFloat(price);
  const existing = state.items.findIndex((item) => item.id === id);
  const items = existing >= 0
    ? state.items.map((item, i) => (i === existing ? { ...item, qty: item.qty + 1 } : item))
    : [...state.items, { id, name, price: unit, qty: 1 }];
  return { items, total: state.total + unit };
}

/** Remove the line at `index` and take its value off the total. */
export function removeItem(state: CartState, index: number): CartState {
  const item = state.items[index];
  if (!item) return state;
  return {
    items: state.items.filter((_, i) => i !== index),
    total: state.total - item.price * item.qty
  };
}

/** Set the quantity of the line at `index` (the edit dialog's OK). */
export function setQuantity(state: CartState, index: number, qty: number): CartState {
  const item = state.items[index];
  if (!item) return state;
  const nextQty = clampQuantity(qty);
  return {
    items: state.items.map((line, i) => (i === index ? { ...line, qty: nextQty } : line)),
    total: state.total - item.price * item.qty + item.price * nextQty
  };
}

/** Quantities never go below one in the edit dialog. */
export function clampQuantity(qty: number): number {
  return Number.isFinite(qty) && qty >= 1 ? Math.floor(qty) : 1;
}

/** Dollar string with two decimals: `$4.98`. A missing total reads as zero. */
export function formatMoney(amount: number): string {
  return `$${(amount || 0).toFixed(2)}`;
}

/** Tax on the running subtotal. */
export function taxOf(total: number): number {
  return (total || 0) * TAX_RATE;
}

/** Subtotal plus tax (the checkout "Balance Due"). */
export function grandTotalOf(total: number): number {
  return (total || 0) * (1 + TAX_RATE);
}

/** Price of one line (`price × qty`). */
export function lineTotal(item: CartItem): number {
  return item.price * item.qty;
}

function isCartItem(value: unknown): value is CartItem {
  if (typeof value !== 'object' || value === null) return false;
  const record: Record<string, unknown> = { ...value };
  return typeof record.id === 'number'
    && typeof record.name === 'string'
    && typeof record.price === 'number'
    && typeof record.qty === 'number';
}

/**
 * Restore the cart saved by this page (or by the Rails page before it).
 * Unreadable or malformed data yields an empty cart instead of a crash.
 */
export function loadCart(storage: CartStorage | undefined): CartState {
  if (!storage) return EMPTY_CART;
  try {
    const rawItems = storage.getItem(CART_KEY);
    const parsed: unknown = rawItems ? JSON.parse(rawItems) : [];
    const items = Array.isArray(parsed) ? parsed.filter(isCartItem) : [];
    const rawTotal = storage.getItem(TOTAL_KEY);
    const total = rawTotal ? parseFloat(rawTotal) : 0;
    return { items, total: Number.isFinite(total) ? total : 0 };
  } catch (error) {
    console.error('Failed to restore saved cart', error);
    return EMPTY_CART;
  }
}

/** Persist the cart under the Rails page's keys. Storage failures are logged. */
export function saveCart(storage: CartStorage | undefined, state: CartState): void {
  if (!storage) return;
  try {
    storage.setItem(CART_KEY, JSON.stringify(state.items));
    storage.setItem(TOTAL_KEY, state.total.toString());
  } catch (error) {
    console.error('Failed to save cart', error);
  }
}
