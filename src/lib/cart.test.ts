import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  EMPTY_CART,
  addItem,
  clampQuantity,
  formatMoney,
  grandTotalOf,
  lineTotal,
  loadCart,
  removeItem,
  saveCart,
  setQuantity,
  taxOf
} from './cart.ts';
import type { CartState, CartStorage } from './cart.ts';

/** In-memory storage stand-in. */
function memoryStorage(seed: Record<string, string> = {}): CartStorage & { data: Record<string, string> } {
  const data = { ...seed };
  return {
    data,
    getItem: (key) => (key in data ? data[key] : null),
    setItem: (key, value) => {
      data[key] = value;
    }
  };
}

const withHotdog = (): CartState => addItem(EMPTY_CART, 1, 'Hotdog & Soda', '1.5');

describe('addItem', () => {
  it('appends a new line at quantity one', () => {
    assert.deepEqual(withHotdog().items, [{ id: 1, name: 'Hotdog & Soda', price: 1.5, qty: 1 }]);
  });

  it('merges a repeat tap into the same line', () => {
    assert.equal(addItem(withHotdog(), 1, 'Hotdog & Soda', '1.5').items[0].qty, 2);
  });

  it('adds the parsed price to the running total', () => {
    assert.equal(addItem(withHotdog(), 2, 'Slice Pepperoni', '1.99').total, 3.49);
  });

  it('does not mutate the previous state', () => {
    const before = withHotdog();
    addItem(before, 1, 'Hotdog & Soda', '1.5');
    assert.equal(before.items[0].qty, 1);
  });
});

describe('removeItem', () => {
  it('drops the line and its value', () => {
    const cart = addItem(addItem(withHotdog(), 1, 'Hotdog & Soda', '1.5'), 9, 'Soda', '0.79');
    const after = removeItem(cart, 0);
    assert.deepEqual([after.items.length, after.total.toFixed(2)], [1, '0.79']);
  });

  it('ignores an index past the end', () => {
    const cart = withHotdog();
    assert.equal(removeItem(cart, 5), cart);
  });
});

describe('setQuantity', () => {
  it('replaces the line value in the total', () => {
    assert.equal(setQuantity(withHotdog(), 0, 3).total, 4.5);
  });

  it('never goes below one', () => {
    assert.equal(setQuantity(withHotdog(), 0, 0).items[0].qty, 1);
  });
});

describe('clampQuantity', () => {
  it('floors to a whole unit', () => {
    assert.equal(clampQuantity(2.7), 2);
  });

  it('turns NaN into one', () => {
    assert.equal(clampQuantity(Number.NaN), 1);
  });
});

describe('money', () => {
  it('formats two decimals', () => {
    assert.equal(formatMoney(1.5), '$1.50');
  });

  it('computes 8% tax like the Rails page', () => {
    assert.equal(formatMoney(taxOf(9.95)), '$0.80');
  });

  it('computes the balance due with tax', () => {
    assert.equal(formatMoney(grandTotalOf(9.95)), '$10.75');
  });

  it('prices a line by quantity', () => {
    assert.equal(lineTotal({ id: 1, name: 'x', price: 1.99, qty: 3 }).toFixed(2), '5.97');
  });
});

describe('loadCart / saveCart', () => {
  it('round-trips through the Rails keys', () => {
    const storage = memoryStorage();
    saveCart(storage, withHotdog());
    assert.deepEqual(loadCart(storage), withHotdog());
  });

  it('reads a cart saved by the Rails page', () => {
    const storage = memoryStorage({ cart: '[{"id":8,"name":"Chicken Bake","price":3.99,"qty":2}]', total: '7.98' });
    assert.deepEqual(loadCart(storage), { items: [{ id: 8, name: 'Chicken Bake', price: 3.99, qty: 2 }], total: 7.98 });
  });

  it('falls back to empty on corrupt JSON', () => {
    assert.deepEqual(loadCart(memoryStorage({ cart: '{oops' })), EMPTY_CART);
  });

  it('drops malformed lines', () => {
    assert.equal(loadCart(memoryStorage({ cart: '[{"id":"x"}]', total: '1' })).items.length, 0);
  });

  it('handles missing storage', () => {
    assert.deepEqual(loadCart(undefined), EMPTY_CART);
  });
});
