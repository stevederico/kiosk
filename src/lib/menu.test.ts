import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { MENU_PATH, fetchMenu, parseMenu } from './menu.ts';

/** First item of the live https://kiosk.bixbyapps.com/menu_items response. */
const HOTDOG = {
  id: 1,
  name: 'Hotdog & Soda',
  category: null,
  created_at: '2026-10-28T01:10:18.559Z',
  updated_at: '2026-10-28T01:10:18.559Z',
  price: '1.5'
};

function stubFetch(status: number, body: unknown, seen: string[] = []): typeof fetch {
  return async (input) => {
    seen.push(String(input));
    return new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
  };
}

describe('parseMenu', () => {
  it('accepts the Rails menu shape', () => {
    assert.deepEqual(parseMenu([HOTDOG]), [HOTDOG]);
  });

  it('rejects a numeric price', () => {
    assert.throws(() => parseMenu([{ ...HOTDOG, price: 1.5 }]), /expected format/);
  });

  it('rejects a non-array body', () => {
    assert.throws(() => parseMenu({ error: 'x' }), /expected format/);
  });
});

describe('fetchMenu', () => {
  it('requests the menu endpoint', async () => {
    const seen: string[] = [];
    await fetchMenu(stubFetch(200, [HOTDOG], seen));
    assert.deepEqual(seen, [MENU_PATH]);
  });

  it('returns the parsed items', async () => {
    assert.equal((await fetchMenu(stubFetch(200, [HOTDOG])))[0].name, 'Hotdog & Soda');
  });

  it('surfaces an HTTP failure', async () => {
    await assert.rejects(fetchMenu(stubFetch(500, [])), /HTTP 500/);
  });
});
