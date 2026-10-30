import { Button } from '@stevederico/skateboard-ui/shadcn/ui/button';
import { formatMoney, grandTotalOf } from '../lib/cart.ts';

/** The Rails page's inline PIN pad drawing, byte for byte. */
const PIN_PAD_SVG = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 150'%3E%3Crect width='200' height='150' fill='%234A90E2' rx='0'/%3E%3Crect x='20' y='20' width='160' height='60' fill='%232E5BBA' rx='0'/%3E%3Ctext x='100' y='50' text-anchor='middle' fill='white' font-size='12'%3EPIN PAD%3C/text%3E%3Ccircle cx='50' cy='110' r='8' fill='%23333'/%3E%3Ccircle cx='80' cy='110' r='8' fill='%23333'/%3E%3Ccircle cx='110' cy='110' r='8' fill='%23333'/%3E%3Ccircle cx='65' cy='130' r='8' fill='%23333'/%3E%3Ccircle cx='95' cy='130' r='8' fill='%23333'/%3E%3C/svg%3E";

/** Props for {@link CheckoutScreen}. */
export interface CheckoutScreenProps {
  total: number;
  onReturn: () => void;
}

/**
 * Mock payment step: balance due with tax and a PIN pad prompt. Like the
 * Rails page, nothing is sent to the server; "Return to Menu" goes back.
 */
export default function CheckoutScreen({ total, onReturn }: CheckoutScreenProps) {
  return (
    <section id="checkout-screen" className="flex h-full flex-col items-center justify-center border border-kiosk-ink bg-kiosk-surface" aria-labelledby="balance-due">
      <div className="text-center">
        <h2 id="balance-due" className="mb-8 text-3xl font-bold text-balance">
          Balance Due: <span id="checkout-total">{formatMoney(grandTotalOf(total))}</span>
        </h2>
        <p className="mb-8 text-lg text-kiosk-muted">Follow prompts on PIN pad.</p>
        <div className="mb-8">
          <img src={PIN_PAD_SVG} alt="PIN Pad" width={192} height={144} className="mx-auto h-36 w-48" />
        </div>
        <Button
          onClick={onReturn}
          data-testid="return-to-menu"
          className="h-auto rounded-none border border-kiosk-ink bg-kiosk-primary px-8 py-3 text-base font-semibold text-kiosk-on-primary hover:bg-kiosk-primary-hover"
        >
          Return to Menu
        </Button>
      </div>
    </section>
  );
}
