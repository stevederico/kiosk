#!/bin/bash
# Drive the same cart flow on the Rails page and this page (agent-browser,
# Brave CDP 9222) and print state per step; diff the two outputs.
# usage: scenario.sh <url> <rails|rust>
export AGENT_BROWSER_SESSION=kiosk-parity
B="agent-browser --cdp 9222"
URL=$1; IMPL=$2
if [ "$IMPL" = rails ]; then
  TILE='#menu-grid > div'; ROW='#cart > div'; ROWCLICK='#cart > div'; EDIT='#cart > div button';
  PLUS='#edit-modal button:nth-of-type(2)'; OK='#edit-modal .flex.gap-4 button:nth-child(2)'; RM='#edit-modal button:nth-of-type(3)';
  PLACE='#place-order-button'; CANCEL='#clear-cart-button'; RET='#checkout-screen button'
else
  TILE='#menu-grid > button'; ROW='#cart > li'; ROWCLICK='#cart > li > button'; EDIT='#cart > li button[aria-label^=Edit]';
  PLUS='button[aria-label="Increase quantity"]'; OK='[data-testid=confirm-quantity]'; RM='[data-testid=remove-item]';
  PLACE='[data-testid=place-order]'; CANCEL='[data-testid=cancel-order]'; RET='[data-testid=return-to-menu]'
fi
state() {
  $B eval "(()=>{const t=id=>{const e=document.getElementById(id);return e?e.textContent.trim()+(e.className.includes('opacity-0')?'(hidden)':''):null};const co=document.getElementById('checkout-screen');const coShown=!!co&&!co.classList.contains('hidden')&&co.offsetParent!==null;return JSON.stringify({step:'$1',sub:t('subtotal'),tax:t('tax'),total:t('total-amount'),rows:document.querySelectorAll('$ROW').length,checkout:coShown?document.getElementById('checkout-total').textContent:null,grid:document.querySelectorAll('$TILE').length,gridVisible:document.getElementById('menu-grid')?.offsetParent!==null,cart:localStorage.getItem('cart'),lstotal:localStorage.getItem('total')})})()" | sed 's/^"//; s/"$//; s/\\"/"/g'
}
clk() { $B eval "(()=>{const els=document.querySelectorAll('$1');const e=els[$2];if(!e)return 'MISSING $1 $2';e.click();return 'ok'})()" >/dev/null; sleep 0.3; }
$B open "$URL" >/dev/null; $B eval 'localStorage.clear(); "x"' >/dev/null; $B reload >/dev/null; sleep 2
state 0-empty
clk "$TILE" 0; clk "$TILE" 0; clk "$TILE" 1; state 1-added
clk "$EDIT" 0; clk "$PLUS" 0; clk "$PLUS" 0; clk "$OK" 0; state 2-edited
clk "$EDIT" 1; clk "$RM" 0; state 3-modal-removed
clk "$TILE" 8; clk "$ROWCLICK" 1; state 4-row-removed
clk "$TILE" 8; clk "$PLACE" 0; state 5-checkout
clk "$RET" 0; state 6-returned
$B reload >/dev/null; sleep 2; state 7-reloaded
clk "$CANCEL" 0; state 8-cancelled
