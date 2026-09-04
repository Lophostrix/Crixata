import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const extRoot = path.resolve(__dirname, '..');
const distRoot = path.resolve(extRoot, 'dist');

console.log('Testing Crixata Chrome Extension Implementation...');

// 1. Verify dist manifest and files
const manifestPath = path.resolve(distRoot, 'manifest.json');
assert(fs.existsSync(manifestPath), 'dist/manifest.json must exist');
const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));

assert.strictEqual(manifest.manifest_version, 3, 'Manifest version must be 3');
assert.strictEqual(manifest.action.default_popup, 'popup.html', 'default_popup must be popup.html');
assert.strictEqual(manifest.background.service_worker, 'background.js', 'service_worker must be background.js');
assert.strictEqual(manifest.content_scripts[0].js[0], 'content.js', 'content script must be content.js');

// Verify files pointed to by manifest actually exist in dist
assert(fs.existsSync(path.resolve(distRoot, manifest.action.default_popup)), 'popup.html must exist in dist');
assert(fs.existsSync(path.resolve(distRoot, manifest.background.service_worker)), 'background.js must exist in dist');
assert(fs.existsSync(path.resolve(distRoot, manifest.content_scripts[0].js[0])), 'content.js must exist in dist');

for (const [size, iconPath] of Object.entries(manifest.icons)) {
  assert(fs.existsSync(path.resolve(distRoot, iconPath)), `Icon ${size} (${iconPath}) must exist in dist`);
}

// Verify dist/src does not exist
assert(!fs.existsSync(path.resolve(distRoot, 'src')), 'dist/src must not exist');

console.log('✔ Build layout & manifest.json verification passed');

// 2. Verify content.js has no invalid ES module syntax for classic content scripts
const contentJs = fs.readFileSync(path.resolve(distRoot, 'content.js'), 'utf8');
assert(!/^\s*export\s+/m.test(contentJs), 'content.js must not contain export statements');
assert(!/^\s*import\s+/m.test(contentJs), 'content.js must not contain import statements');
assert(contentJs.includes('15000') || contentJs.includes('15e3'), 'content.js must limit text to ~15,000 characters');

console.log('✔ content.js script integrity passed');

// 3. Verify popup.html contents
const popupHtml = fs.readFileSync(path.resolve(distRoot, 'popup.html'), 'utf8');

// Privacy disclaimer exact text
const requiredDisclaimer = 'No user data, browsing history, or policy content is ever tracked, collected, or sent to a remote server.';
assert(popupHtml.includes(requiredDisclaimer), 'popup.html must contain verbatim privacy disclaimer');

// Required elements
assert(popupHtml.includes('id="offline-alert"'), 'popup.html must have offline alert element');
assert(popupHtml.includes('Desktop app not running'), 'popup.html must mention "Desktop app not running"');
assert(popupHtml.includes('id="reanalyze-btn"'), 'popup.html must have reanalyze button');
assert(popupHtml.includes('Re-analyze this page'), 'popup.html must have "Re-analyze this page" button text');
assert(popupHtml.includes('id="grade-badge"'), 'popup.html must have grade badge');
assert(popupHtml.includes('id="source-badge"'), 'popup.html must have source label');
assert(popupHtml.includes('id="data-collected"'), 'popup.html must have data-collected field');
assert(popupHtml.includes('id="data-purposes"'), 'popup.html must have data-purposes field');
assert(popupHtml.includes('id="third-party-details"'), 'popup.html must have third-party details field');
assert(popupHtml.includes('id="retention-period"'), 'popup.html must have retention period field');
assert(popupHtml.includes('id="right-delete"'), 'popup.html must have right-delete field');
assert(popupHtml.includes('id="right-export"'), 'popup.html must have right-export field');
assert(popupHtml.includes('id="right-optout"'), 'popup.html must have right-optout field');
assert(popupHtml.includes('id="tracking-ads"'), 'popup.html must have tracking & ads field');
assert(popupHtml.includes('id="arbitration-alert"'), 'popup.html must have arbitration waiver field');
assert(popupHtml.includes('id="clarity-notes"'), 'popup.html must have clarity notes field');

console.log('✔ popup.html structure & disclaimer passed');

// 4. Test detector module from built assets
const {
  d: detectPolicyPage,
  e: extractDomain,
  c: checkHealth,
  g: gradePolicy
} = await import('../dist/assets/api.js');

assert.strictEqual(extractDomain('https://www.example.com/some/path'), 'example.com');
assert.strictEqual(extractDomain('https://sub.domain.org/terms'), 'sub.domain.org');

// Heuristic 1: URL path
const r1 = detectPolicyPage('https://example.com/privacy');
assert.strictEqual(r1.isPolicyPage, true);
assert.strictEqual(r1.policyType, 'privacy');
assert.strictEqual(r1.confidence, 'high');

const r2 = detectPolicyPage('https://example.com/terms-of-service');
assert.strictEqual(r2.isPolicyPage, true);
assert.strictEqual(r2.policyType, 'terms');
assert.strictEqual(r2.confidence, 'high');

const r3 = detectPolicyPage('https://example.com/cookie-policy');
assert.strictEqual(r3.isPolicyPage, true);
assert.strictEqual(r3.policyType, 'cookie');
assert.strictEqual(r3.confidence, 'high');

const rTos = detectPolicyPage('https://example.com/tos');
assert.strictEqual(rTos.isPolicyPage, true);
assert.strictEqual(rTos.policyType, 'terms');
assert.strictEqual(rTos.confidence, 'high');

const rEula = detectPolicyPage('https://example.com/eula');
assert.strictEqual(rEula.isPolicyPage, true);
assert.strictEqual(rEula.policyType, 'terms');
assert.strictEqual(rEula.confidence, 'high');

const rConditions = detectPolicyPage('https://example.com/conditions');
assert.strictEqual(rConditions.isPolicyPage, true);
assert.strictEqual(rConditions.policyType, 'terms');
assert.strictEqual(rConditions.confidence, 'high');

// Heuristic 2: Title
const rTitle1 = detectPolicyPage('https://example.com/info/doc1', 'Privacy Policy - Acme Corp');
assert.strictEqual(rTitle1.isPolicyPage, true);
assert.strictEqual(rTitle1.policyType, 'privacy');
assert.strictEqual(rTitle1.confidence, 'medium');

const rTitle2 = detectPolicyPage('https://example.com/document/42', 'Terms and Conditions');
assert.strictEqual(rTitle2.isPolicyPage, true);
assert.strictEqual(rTitle2.policyType, 'terms');
assert.strictEqual(rTitle2.confidence, 'medium');

// Heuristic 3: DOM links
const rDom = detectPolicyPage('https://example.com/page', 'General Page', [
  { text: 'Privacy Policy', href: 'https://example.com/privacy' }
]);
assert.strictEqual(rDom.isPolicyPage, true);
assert.strictEqual(rDom.policyType, 'privacy');
assert.strictEqual(rDom.confidence, 'low');

// Negative case
const rNeg = detectPolicyPage('https://example.com/shop/items/123', 'Summer Shoes - Sale', [
  { text: 'Buy Now', href: '/checkout' }
]);
assert.strictEqual(rNeg.isPolicyPage, false);
assert.strictEqual(rNeg.policyType, 'unknown');
assert.strictEqual(rNeg.confidence, 'low');

console.log('✔ detector heuristics passed');

// 5. Test badge module colors in background.js
const bgJs = fs.readFileSync(path.resolve(distRoot, 'background.js'), 'utf8');
assert(bgJs.includes('#22c55e'), 'Grade A color #22c55e must be present in background.js');
assert(bgJs.includes('#3b82f6'), 'Grade B color #3b82f6 must be present in background.js');
assert(bgJs.includes('#f97316'), 'Grade C color #f97316 must be present in background.js');
assert(bgJs.includes('#ef4444'), 'Grade D color #ef4444 must be present in background.js');
assert(bgJs.includes('#6b7280'), 'Grade ? color #6b7280 must be present in background.js');

console.log('✔ badge grade colors passed');

// 6. Test API client offline behavior
const offlineHealth = await checkHealth('http://127.0.0.1:59999');
assert.strictEqual(offlineHealth.ready, false, 'checkHealth should handle offline port gracefully');
assert.strictEqual(offlineHealth.status, 'offline', 'checkHealth should return offline status');

console.log('✔ api graceful offline handling passed');

console.log('\nAll Crixata Chrome MV3 Extension verification tests passed successfully!');
