# Vercel frontend-only preview

User approved replacing the failed Rust-function deployment with a static UI-only
preview. Local production remains the Rust/systemd service on port5050. No tunnel,
API proxy, authentication, cloud audio or Bluetooth access is configured here.

`vercel.json` explicitly sets framework=null and a dependency-free Node build.
`scripts/build_vercel_preview.cjs` emits `.vercel/output/config.json` (Build Output
API version3) and only allowlisted files under static/. There are no functions.
The generated HTML derives from src/index.html without modifying it: preview
labels in Home/opening, API helper fails closed, backend polling removed. Heart
animations, local font/icons and Quotes remain. All /api paths return503 JSON;
unknown paths return404. Source/binaries/reports are not copied into the output.
No Speed Insights/analytics package or tracking code is added.

Local checks:

```bash
node --test tests/frontend.test.cjs tests/vercel-preview.test.cjs
node scripts/build_vercel_preview.cjs
# See docs/MOBILE_UI.md for the external test-only browser dependencies.
BROWSER_TEST_MODULES=/path/to/test/node_modules node tests/vercel-preview.browser.cjs
```

Deploy the current Arena branch through the existing Vercel project `ayshamusic`
in team afine/afine1. Production promotion is limited to this static preview; do
not migrate the Linux service or weaken deployment protection. The connector's
nullable framework update can leave the dashboard Rust preset unchanged, so the
explicit framework=null in versioned vercel.json is important. Verify the actual
build output has no functions; a READY status alone is not live HTTP acceptance.
Validate root HTML, icons, no runtime500 and /api/status503, plus browser behavior.
If external HTTP checks are blocked by the agent's network, state that limitation.

Rollback: redeploy a previously verified static preview revision through the same
project. The old Rust deployments are known-broken at startup and are not a good
healthy rollback target. Keep local service access unchanged. Connecting music
later requires an approved authenticated remote backend path and separate tests.
