import { Pointer } from 'lucide-react';
import { Button } from '@stevederico/skateboard-ui/shadcn/ui/button';
import { cn } from '@stevederico/skateboard-ui/shadcn/lib/utils';
import CartLine from './CartLine.tsx';
import { formatMoney, grandTotalOf, taxOf } from '../lib/cart.ts';
import type { CartState } from '../lib/cart.ts';

/** Props for {@link CartPanel}. */
export interface CartPanelProps {
  cart: CartState;
  showTotals: boolean;
  onRemove: (index: number) => void;
  onEdit: (index: number) => void;
  onCancelOrder: () => void;
  onPlaceOrder: () => void;
}

/**
 * Left column: logo, welcome, receipt lines, order buttons and totals.
 *
 * As on the Rails page, Tax and Total stay invisible while the cart has items
 * and appear once it is empty or the order is placed (`showTotals`).
 */
export default function CartPanel({ cart, showTotals, onRemove, onEdit, onCancelOrder, onPlaceOrder }: CartPanelProps) {
  const hasItems = cart.items.length > 0;
  const hidden = cn(!showTotals && 'opacity-0');
  return (
    <aside className="flex h-4/5 w-1/5 flex-col overflow-y-auto border border-kiosk-ink bg-kiosk-surface" aria-label="Your order">
      <header>
        <img src="/images/frankie-logo.png" alt="Frankie Logo" width={560} height={177} className="h-auto w-full" />
        <h1 className="mb-4 border-t text-center text-4xl font-semibold text-balance">Welcome to the Frankie Food Court Kiosk!</h1>
        <p className="flex items-center justify-center gap-2 text-sm text-kiosk-muted">
          <Pointer size={16} aria-hidden="true" />
          <span>Please tap your selection to begin your order.</span>
        </p>
      </header>

      <div className="flex-1">
        {hasItems && (
          <p className="mb-4 bg-kiosk-hint p-3 text-2xl text-kiosk-muted">
            You can change or remove an item on the receipt by tapping it.
          </p>
        )}
        <ul id="cart" className="mb-4">
          {cart.items.map((item, index) => (
            <CartLine key={item.id} item={item} onRemove={() => onRemove(index)} onEdit={() => onEdit(index)} />
          ))}
        </ul>
      </div>

      {hasItems && (
        <div className="absolute bottom-0 left-0 flex h-1/5 w-1/5 flex-col-reverse gap-2">
          <Button
            onClick={onPlaceOrder}
            data-testid="place-order"
            className="h-auto w-full rounded-none border border-kiosk-ink bg-kiosk-confirm py-3 text-base font-semibold text-kiosk-on-primary hover:bg-kiosk-confirm-hover"
          >
            PLACE ORDER
          </Button>
          <Button
            onClick={onCancelOrder}
            data-testid="cancel-order"
            className="h-auto w-full rounded-none border border-kiosk-ink bg-kiosk-primary py-2 text-base text-kiosk-on-primary hover:bg-kiosk-primary"
          >
            CANCEL ORDER
          </Button>
        </div>
      )}

      <dl className="border-2 border-kiosk-ink text-3xl">
        <div className="flex justify-between">
          <dt>Sub-Total:</dt>
          <dd id="subtotal">{formatMoney(cart.total)}</dd>
        </div>
        <div className="flex justify-between">
          <dt className={hidden}>Tax:</dt>
          <dd id="tax" className={hidden}>{formatMoney(taxOf(cart.total))}</dd>
        </div>
        <div className="flex justify-between font-bold">
          <dt className={hidden}>Total:</dt>
          <dd id="total-amount" className={hidden}>{formatMoney(grandTotalOf(cart.total))}</dd>
        </div>
      </dl>
    </aside>
  );
}
