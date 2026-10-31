import { Sandwich } from 'lucide-react';
import { Button } from '@stevederico/skateboard-ui/shadcn/ui/button';
import { formatMoney, lineTotal } from '../lib/cart.ts';
import type { CartItem } from '../lib/cart.ts';

/** Props for {@link CartLine}. */
export interface CartLineProps {
  item: CartItem;
  onRemove: () => void;
  onEdit: () => void;
}

/**
 * One receipt line. Tapping the line removes it (Rails behavior); the Edit
 * button opens the quantity dialog. The remove target is a full-row button
 * under the content so Edit stays a separate, valid control.
 */
export default function CartLine({ item, onRemove, onEdit }: CartLineProps) {
  return (
    <li className="relative mb-3 border-b border-kiosk-rule p-2 pb-3 hover:bg-kiosk-row-hover">
      <button
        type="button"
        onClick={onRemove}
        aria-label={`Remove ${item.name}`}
        data-testid="cart-line-remove"
        className="absolute inset-0 cursor-pointer"
      />
      <div className="pointer-events-none relative flex items-center gap-3">
        <span className="flex size-12 items-center justify-center bg-kiosk-thumb text-kiosk-thumb-ink">
          <Sandwich size={16} aria-hidden="true" />
        </span>
        <div className="flex-1">
          <div className="font-medium text-kiosk-name">{item.name}</div>
          <div className="mt-1 flex items-center gap-2">
            <span className="text-sm text-kiosk-muted">Quantity {item.qty}</span>
            <Button
              onClick={onEdit}
              data-testid="cart-line-edit"
              aria-label={`Edit quantity of ${item.name}`}
              className="pointer-events-auto h-auto rounded-none border-0 bg-kiosk-accept px-3 py-1 text-xs font-medium text-kiosk-on-primary hover:bg-kiosk-accept-hover"
            >
              Edit
            </Button>
          </div>
        </div>
        <span className="text-lg font-bold">{formatMoney(lineTotal(item))}</span>
      </div>
    </li>
  );
}
