import { useCallback, useEffect, useState } from 'react';
import CartPanel from './CartPanel.tsx';
import CheckoutScreen from './CheckoutScreen.tsx';
import MenuGrid from './MenuGrid.tsx';
import QuantityDialog from './QuantityDialog.tsx';
import { EMPTY_CART, addItem, loadCart, removeItem, saveCart, setQuantity } from '../lib/cart.ts';
import type { CartState } from '../lib/cart.ts';
import { fetchMenu } from '../lib/menu.ts';
import type { MenuItem } from '../lib/menu.ts';

/** `localStorage`, or `undefined` where the browser blocks it. */
function browserStorage(): Storage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

/**
 * The food court kiosk: menu grid on the right, receipt on the left, a mock
 * PIN pad checkout, and a quantity dialog. Port of the Rails `kiosk.html`.
 */
export default function KioskView() {
  const [cart, setCart] = useState<CartState>(() => loadCart(browserStorage()));
  const [view, setView] = useState<'menu' | 'checkout'>('menu');
  const [showTotals, setShowTotals] = useState(false);
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [menu, setMenu] = useState<MenuItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadMenu = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      setMenu(await fetchMenu());
    } catch (err) {
      console.error('Failed to load menu', err);
      setError(err instanceof Error ? err.message : 'The menu could not be loaded.');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadMenu();
  }, [loadMenu]);

  /** Every cart change saves and re-hides Tax/Total, as `updateCart()` did. */
  const updateCart = (next: CartState) => {
    setCart(next);
    setShowTotals(false);
    saveCart(browserStorage(), next);
  };

  const editingItem = editingIndex === null ? null : cart.items[editingIndex] ?? null;

  const handlePlaceOrder = () => {
    setShowTotals(true);
    setView('checkout');
  };

  const handleRemoveFromDialog = () => {
    if (editingIndex !== null) updateCart(removeItem(cart, editingIndex));
    setEditingIndex(null);
  };

  const handleConfirmQuantity = (qty: number) => {
    if (editingIndex !== null) updateCart(setQuantity(cart, editingIndex, qty));
    setEditingIndex(null);
  };

  return (
    <div className="flex h-screen bg-kiosk-backdrop font-kiosk text-kiosk-ink">
      <CartPanel
        cart={cart}
        showTotals={showTotals || cart.items.length === 0}
        onRemove={(index) => updateCart(removeItem(cart, index))}
        onEdit={setEditingIndex}
        onCancelOrder={() => updateCart(EMPTY_CART)}
        onPlaceOrder={handlePlaceOrder}
      />
      <main className="flex-1">
        <div className="border border-kiosk-ink bg-kiosk-surface p-6">
          {view === 'menu' && (
            <MenuGrid
              items={menu}
              isLoading={isLoading}
              error={error}
              onRetry={() => void loadMenu()}
              onSelect={(item) => updateCart(addItem(cart, item.id, item.name, item.price))}
            />
          )}
        </div>
        {view === 'checkout' && <CheckoutScreen total={cart.total} onReturn={() => setView('menu')} />}
      </main>
      <QuantityDialog
        item={editingItem}
        onCancel={() => setEditingIndex(null)}
        onConfirm={handleConfirmQuantity}
        onRemove={handleRemoveFromDialog}
      />
    </div>
  );
}
