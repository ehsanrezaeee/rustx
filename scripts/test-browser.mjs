import {createRequire} from 'node:module';import assert from 'node:assert/strict';
const require=createRequire(import.meta.url);const {chromium}=require('playwright');
const browser=await chromium.launch({executablePath:process.env.CHROME_PATH||undefined,headless:true});
try{
 const page=await browser.newPage({viewport:{width:1200,height:1000}}),errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 await page.goto((process.env.RUSTX_URL||'http://127.0.0.1:3000')+'/?renderer=cpu');
 await page.waitForFunction(()=>window.rustx?.draws()>0);
 await page.getByTestId('nav-lab').click();await page.getByTestId('inc-ALPHA').click();await page.getByTestId('reverse').click();
 await page.waitForFunction(()=>document.querySelector('[data-testid="count-ALPHA"]')?.textContent.endsWith('1'));
 await page.getByTestId('name-input').fill('سلام RustX');
 await page.waitForFunction(()=>document.querySelector('[data-testid="echo"]')?.textContent==='سلام RustX');
 assert.equal(await page.getByTestId('name-input').evaluate(e=>e===document.activeElement),true);
 await page.getByTestId('nav-people').click();await page.getByTestId('load-fixture').click();
 await page.waitForFunction(()=>document.querySelector('[data-testid="api-state"]')?.textContent.includes('6 people'));
 await page.getByTestId('load-slow').click();await page.getByTestId('cancel').click();
 await page.waitForFunction(()=>document.querySelector('[data-testid="api-state"]')?.textContent.includes('Cancelled'));
 const a=await page.evaluate(()=>window.rustx.draws());await page.waitForTimeout(300);assert.equal(await page.evaluate(()=>window.rustx.draws()),a);assert.deepEqual(errors,[]);
 console.log('Browser CPU suite passed. Real IME/screen-reader checks are separate.');
}finally{await browser.close();}
