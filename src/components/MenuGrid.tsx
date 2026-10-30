import { CircleAlert } from 'lucide-react';
import { Button } from '@stevederico/skateboard-ui/shadcn/ui/button';
import { Spinner } from '@stevederico/skateboard-ui/shadcn/ui/spinner';
import { Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription } from '@stevederico/skateboard-ui/shadcn/ui/empty';
import type { MenuItem } from '../lib/menu.ts';

/** Props for {@link MenuGrid}. */
export interface MenuGridProps {
  items: MenuItem[];
  isLoading: boolean;
  error: string | null;
  onRetry: () => void;
  onSelect: (item: MenuItem) => void;
}

/**
 * The 8-column menu of tappable tiles. Each tile shows an image placeholder,
 * the item name and its price exactly as the API serves it (`$1.5`).
 */
export default function MenuGrid({ items, isLoading, error, onRetry, onSelect }: MenuGridProps) {
  if (isLoading) {
    return (
      <div className="flex justify-center p-6" role="status" aria-live="polite">
        <Spinner />
        <span className="sr-only">Loading menu</span>
      </div>
    );
  }
  if (error) {
    return (
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant="icon"><CircleAlert size={24} /></EmptyMedia>
          <EmptyTitle>The Menu Did Not Load</EmptyTitle>
          <EmptyDescription>{error} Check the connection and try again.</EmptyDescription>
        </EmptyHeader>
        <Button onClick={onRetry}>Try Again</Button>
      </Empty>
    );
  }
  return (
    <div id="menu-grid" className="grid h-full grid-cols-8 gap-4">
      {items.map((item) => (
        <Button
          key={item.id}
          onClick={() => onSelect(item)}
          data-testid="menu-item-tap"
          className="h-auto flex-col items-center justify-between gap-0 whitespace-normal rounded-none border border-kiosk-ink bg-kiosk-primary p-0 text-kiosk-on-primary shadow-2xl drop-shadow-kiosk-tile transition-colors hover:bg-kiosk-primary-hover active:scale-100"
        >
          <span className="flex h-32 w-full items-center justify-center border border-kiosk-ink bg-kiosk-placeholder">
            <span className="text-xs text-kiosk-placeholder-ink">Image</span>
          </span>
          <span className="text-center text-sm font-semibold">{item.name}</span>
          <span className="mt-auto text-xl font-bold">${item.price}</span>
        </Button>
      ))}
    </div>
  );
}
