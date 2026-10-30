#!/bin/bash
# Compare the Rails kiosk and this app over HTTP. Read-only GETs plus POSTs
# that Rails rejects; point it only at local copies, never at production.
# parity.sh RAILS_BASE RUST_BASE : prints one line per check, "SAME"/"DIFF"
R=$1; N=$2; UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"
OLD="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36"
norm() { sed -E 's/"(created|updated)_at":"[^"]*"/"\1_at":"T"/g'; }
probe() { # base method path ua [extra curl args...]
  local base=$1 m=$2 p=$3 ua=$4; shift 4
  local body; body=$(mktemp)
  local meta; meta=$(curl -s --path-as-is -X "$m" -A "$ua" -o "$body" -w '%{http_code}|%{content_type}' "$@" "$base$p")
  echo "$meta|$(norm < "$body" | sha256sum | cut -c1-12)|$(wc -c < "$body")"
  rm -f "$body"
}
check() { # label mode(full|status|statusct) method path ua [curl args]
  local label=$1 mode=$2 m=$3 p=$4 ua=$5; shift 5
  local a b; a=$(probe "$R" "$m" "$p" "$ua" "$@"); b=$(probe "$N" "$m" "$p" "$ua" "$@")
  local ca cb
  case $mode in
    full) ca=$a; cb=$b;;
    statusbody) ca=$(echo "$a" | cut -d'|' -f1,3); cb=$(echo "$b" | cut -d'|' -f1,3);;
    statusct) ca=$(echo "$a" | cut -d'|' -f1,2 | tr A-Z a-z); cb=$(echo "$b" | cut -d'|' -f1,2 | tr A-Z a-z);;
    status) ca=$(echo "$a" | cut -d'|' -f1); cb=$(echo "$b" | cut -d'|' -f1);;
  esac
  if [ "$ca" = "$cb" ]; then s=SAME; else s=DIFF; fi
  printf '%-4s %-42s rails=%-60s rust=%s\n' "$s" "$label" "$a" "$b"
}
check "GET /"                     statusct GET  /                  "$UA"
check "GET /?a=b"                 statusct GET  "/?a=b"            "$UA"
check "HEAD /"                    statusct HEAD /                  "$UA" -I
check "GET /menu_items"           full     GET  /menu_items        "$UA"
check "GET /menu_items.json"      full     GET  /menu_items.json   "$UA"
check "GET /menu_items.xml"       full     GET  /menu_items.xml    "$UA"
check "GET /menu_items/"          full     GET  /menu_items/       "$UA"
check "GET //menu_items"          full     GET  //menu_items       "$UA"
check "GET /menu_items?x=1"       full     GET  "/menu_items?x=1"  "$UA"
check "GET /menu_items.json/"     full     GET  /menu_items.json/  "$UA"
check "GET /menu_items/.json"     statusbody GET /menu_items/.json "$UA"
check "GET /menu_items/1"         statusbody GET /menu_items/1     "$UA"
check "GET /MENU_ITEMS"           statusbody GET /MENU_ITEMS       "$UA"
check "GET /%6Benu_items"         statusbody GET /%6Benu_items     "$UA"
check "POST /menu_items"          statusbody POST /menu_items      "$UA"
check "GET /orders"               statusbody GET /orders           "$UA"
check "DELETE /orders"            statusbody DELETE /orders        "$UA"
check "POST /orders json no origin" full POST /orders              "$UA" -H 'Content-Type: application/json' -d '{"total":"9.99","items":[1]}'
check "POST /orders form no origin" full POST /orders              "$UA" -d 'total=1&items=x'
check "POST /orders cross-origin" full     POST /orders            "$UA" -H 'Origin: https://evil.example' -H 'Content-Type: application/json' -d '{}'
check "GET /up"                   full     GET  /up                "$UA"
check "GET /nope"                 full     GET  /nope              "$UA"
check "GET /index.html"           statusbody GET /index.html       "$UA"
check "GET /favicon.ico"          statusbody GET /favicon.ico      "$UA"
check "GET /manifest.json"        statusbody GET /manifest.json    "$UA"
check "GET /service-worker.js"    statusbody GET /service-worker.js "$UA"
check "GET /sitemap.xml"          statusbody GET /sitemap.xml      "$UA"
check "GET /cable"                status   GET  /cable             "$UA"
check "GET /rails/active_storage/x" statusbody GET /rails/active_storage/blobs/x "$UA"
check "GET /api/health"           status   GET  /api/health        "$UA"
check "GET /kiosk.html"           status   GET  /kiosk.html        "$UA"
check "GET /kiosk"                status   GET  /kiosk             "$UA"
for f in icon.png icon.svg images/frankie-logo.png robots.txt 400.html 404.html 406-unsupported-browser.html 422.html 500.html 404 500; do
  check "GET /$f"                 statusbody GET "/$f"             "$UA"
done
check "GET /images/frankie-logo"  statusbody GET /images/frankie-logo "$UA"
check "GET /images/"              statusbody GET /images/           "$UA"
check "old Chrome GET /"          full     GET  /                  "$OLD"
check "old Chrome GET /menu_items" full    GET  /menu_items        "$OLD"
check "old Chrome GET /icon.png"  statusbody GET /icon.png         "$OLD"
check "curl UA GET /menu_items"   full     GET  /menu_items        "curl/8.0"
check "IE11 GET /"                full     GET  /                  "Mozilla/5.0 (Windows NT 10.0; Trident/7.0; rv:11.0) like Gecko"
