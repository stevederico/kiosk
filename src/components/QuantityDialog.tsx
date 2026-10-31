import { useEffect, useState } from 'react';
import { Minus, Plus } from 'lucide-react';
import { Button } from '@stevederico/skateboard-ui/shadcn/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@stevederico/skateboard-ui/shadcn/ui/dialog';
import { clampQuantity } from '../lib/cart.ts';
import type { CartItem } from '../lib/cart.ts';

/** Props for {@link QuantityDialog}. */
export interface QuantityDialogProps {
  item: CartItem | null;
  onCancel: () => void;
  onConfirm: (qty: number) => void;
  onRemove: () => void;
}

const SQUARE_BUTTON = 'size-12 rounded-none border-0 bg-kiosk-primary text-2xl font-bold text-kiosk-on-primary hover:bg-kiosk-primary-hover';
const WIDE_BUTTON = 'h-auto rounded-none border-0 py-3 text-base font-semibold text-kiosk-on-primary';

/**
 * "Quantity Change" dialog: minus (floor of one), plus, Remove Item, Cancel
 * and OK. The count is local until OK, as on the Rails page. Escape cancels.
 */
export default function QuantityDialog({ item, onCancel, onConfirm, onRemove }: QuantityDialogProps) {
  const [qty, setQty] = useState(item?.qty ?? 1);

  useEffect(() => {
    if (item) setQty(item.qty);
  }, [item]);

  const handleOpenChange = (open: boolean) => {
    if (!open) onCancel();
  };

  return (
    <Dialog open={item !== null} onOpenChange={handleOpenChange}>
      <DialogContent showCloseButton={false} className="kiosk-dialog mx-4 block font-kiosk w-full max-w-md gap-0 rounded-none bg-kiosk-surface p-8 text-base text-kiosk-ink ring-0 sm:max-w-md">
        <div className="-mx-8 -mt-8 mb-6 bg-kiosk-primary p-3 text-kiosk-on-primary">
          <DialogTitle className="text-lg font-semibold">Quantity Change</DialogTitle>
        </div>

        <div className="mb-6 text-center">
          <DialogDescription className="mb-2 text-base text-kiosk-muted">Change the quantity or remove the item:</DialogDescription>
          <p id="edit-item-name" className="text-lg font-semibold">{item?.name}</p>
        </div>

        <div className="mb-6 flex items-center justify-center gap-4">
          <Button aria-label="Decrease quantity" className={SQUARE_BUTTON} onClick={() => setQty((q) => clampQuantity(q - 1))}>
            <Minus className="size-6" aria-hidden="true" />
          </Button>
          <output id="edit-quantity" aria-live="polite" className="min-w-16 bg-kiosk-hint px-6 py-3 text-center text-2xl font-bold">{qty}</output>
          <Button aria-label="Increase quantity" className={SQUARE_BUTTON} onClick={() => setQty((q) => q + 1)}>
            <Plus className="size-6" aria-hidden="true" />
          </Button>
          <Button data-testid="remove-item" onClick={onRemove} className={`${WIDE_BUTTON} bg-kiosk-primary px-4 hover:bg-kiosk-primary-hover`}>
            Remove Item
          </Button>
        </div>

        <div className="flex gap-4">
          <Button onClick={onCancel} className={`${WIDE_BUTTON} flex-1 bg-kiosk-primary hover:bg-kiosk-primary-hover`}>
            Cancel
          </Button>
          <Button data-testid="confirm-quantity" onClick={() => onConfirm(qty)} className={`${WIDE_BUTTON} flex-1 bg-kiosk-accept hover:bg-kiosk-accept-hover`}>
            OK
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
