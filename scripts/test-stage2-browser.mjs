// Actual Stage 2 browser behavior. No injected input events are called IME evidence.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {chromium,webkit,firefox} from 'playwright';
const engine=process.env.RUSTX_BROWSER||'chromium';
const gpu=process.env.RUSTX_TEST_GPU==='1';
if(gpu&&engine!=='chromium')throw new Error('The instrumented GPU test currently targets Chromium.');
const report={date:'2026-11-02',engine,backend:gpu?'WebGPU with loss/retry':'Rust CPU',platform:process.platform,arch:process.arch,checks:[],limitations:['No Windows, actual Safari, physical mobile, OS IME or screen-reader acceptance','API counters and local samples are not full platform/performance certification']};
const evidence=new URL('../docs/evidence/',import.meta.url);
await fs.mkdir(evidence,{recursive:true});
const suffix=`${engine}-${gpu?'gpu':'cpu'}`;
let browser,page;const errors=[];
try {
 browser=await ({chromium,webkit,firefox}[engine]).launch({headless:process.env.RUSTX_HEADED!=='1',timeout:60000,...(process.env.CHROME_PATH?{executablePath:process.env.CHROME_PATH}:{}),...(gpu?{args:['--enable-unsafe-webgpu']}:{})});
 report.version=browser.version();
 const context=await browser.newContext({viewport:{width:1200,height:1000},deviceScaleFactor:1});
 page=await context.newPage();page.setDefaultTimeout(15000);page.on('pageerror',e=>errors.push(e.message));
 if(gpu)await page.addInitScript(()=>{
  const stats=window.stage2Presentation={textureToBuffer:0,readMaps:0,submits:0,presents:0,cpuTransfers:0,adapters:[]};
  function watch(proto,name,callback){if(!proto?.[name])return;const original=proto[name];proto[name]=function(...args){callback(args);return original.apply(this,args);};}
  watch(globalThis.GPUCommandEncoder?.prototype,'copyTextureToBuffer',()=>stats.textureToBuffer++);
  watch(globalThis.GPUBuffer?.prototype,'mapAsync',args=>{if(args[0]&1)stats.readMaps++;});
  watch(globalThis.GPUQueue?.prototype,'submit',()=>stats.submits++);
  watch(globalThis.GPUCanvasContext?.prototype,'getCurrentTexture',()=>stats.presents++);
  watch(globalThis.CanvasRenderingContext2D?.prototype,'putImageData',()=>stats.cpuTransfers++);
  if(navigator.gpu){const request=navigator.gpu.requestAdapter.bind(navigator.gpu);navigator.gpu.requestAdapter=async(...args)=>{const adapter=await request(...args);if(adapter){const info=adapter.info||await adapter.requestAdapterInfo?.();stats.adapters.push(info?{vendor:info.vendor,architecture:info.architecture,device:info.device,description:info.description}:{});}return adapter;};}
 });
 const base=process.env.RUSTX_URL||'http://127.0.0.1:3000';
 await page.goto(`${base}/${gpu?'':'?renderer=cpu'}`);await page.waitForFunction(()=>window.rustx?.draws()>0,null,{timeout:60000});
 assert.equal(await page.evaluate(()=>window.rustx.renderer()),gpu?'gpu':'cpu');
 const moduleResponse=await page.request.get(`${base}/pkg/rustx_lab_bg.wasm`);assert(moduleResponse.ok());report.wasmSha256=createHash('sha256').update(await moduleResponse.body()).digest('hex');
 const frames=()=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
 const inspection=()=>page.evaluate(()=>window.rustx.inspect());
 const text=(id,part)=>page.waitForFunction(({id,part})=>window.rustx.inspect().nodes.find(n=>n.id===id)?.text.includes(part),{id,part});
 const click=async id=>{await page.getByTestId(id).focus();await frames();await page.getByTestId(id).click();await frames();};
 const top=async()=>{await page.mouse.move(5,5);await page.mouse.wheel(0,-100000);await page.waitForFunction(()=>window.rustx.inspect().nodes.find(n=>n.id==='nav-home').rect[1]>=0);};
 await click('nav-engine');await text('engine-unicode','🦀');
 const initial=await inspection();
 const countRows=()=>page.locator('[data-testid^="record-"]').count();
 assert((await countRows())<=120);
 const dashboard=initial.nodes.find(n=>n.id==='engine-dashboard');
 assert(dashboard&&dashboard.rect[2]>1000);
 report.checks.push('responsive dashboard and mixed Persian/English/emoji semantic text exist in the actual browser');
 const dark=initial.theme;
 await click('theme-toggle');await page.waitForFunction(dark=>window.rustx.inspect().theme!==dark,dark);
 await click('theme-toggle');await page.waitForFunction(dark=>window.rustx.inspect().theme===dark,dark);
 report.checks.push('public theme control round-trips the Rust theme');
 await click('board-right');await text('camera-state','Camera 120.0');
 await click('board-zoom');await text('camera-state','zoom 1.25');
 await click('board-reset');await text('camera-state','Camera 0.0, 0.0 · zoom 1.00');
 const board=await inspection();
 assert(board.metrics.shapes_visible>0&&board.metrics.shapes_visible<=1000);
 assert.equal(board.nodes.find(n=>n.id==='drawing-board').drawing_count,10000);
 report.drawingVisible=board.metrics.shapes_visible;
 const surface=page.getByTestId('drawing-board');
 await surface.click({position:{x:20,y:15}});await text('board-picked','Selected object: 1');
 const bounds=await surface.boundingBox();await page.mouse.move(bounds.x+60,bounds.y+45);await page.mouse.down();await page.mouse.move(bounds.x+100,bounds.y+65,{steps:4});await page.mouse.up();await text('camera-state','Camera -40.0, -20.0');
 await click('board-reset');await surface.focus();await page.keyboard.press('ArrowRight');await text('camera-state','Camera 40.0');await click('board-reset');
 report.checks.push('10,000-object drawing scene is culled; camera buttons, real pointer pick/drag and keyboard pan work');
 await page.getByTestId('record-0').focus();await frames();
 const original=await page.getByTestId('record-0').getAttribute('data-node');
 await page.getByTestId('rows-end').evaluate(el=>el.click());
 await page.waitForFunction(()=>window.rustx.inspect().nodes.some(n=>n.id==='record-99999'));await frames();
 assert.equal(await page.getByTestId('record-0').getAttribute('data-node'),original);
 assert.equal(await page.getByTestId('record-0').evaluate(el=>el===document.activeElement),true);
 assert((await countRows())<=120);
 assert.equal(await page.locator('[role="listitem"][aria-posinset="100000"][aria-setsize="100000"]').count(),1);
 const list=page.getByRole('list');assert.equal(await list.getAttribute('aria-label'),'100000 records');
 assert(await list.getAttribute('aria-owns'));
 report.checks.push('virtual window retains the actual focused row and node identity, with bounded materialization and list position/ownership metadata');
 // Focusing a last visible row must reveal its nested scroll viewport.
 await page.getByTestId('record-99999').focus();await frames();
 await click('record-99999');await text('selected-row','Selected record: 100000');
 await page.getByTestId('rows-start').evaluate(el=>el.click());await text('row-range','0..31');await frames();
 await page.getByTestId('record-0').focus();await frames();
 const row=await page.getByTestId('record-0').boundingBox();
 await page.mouse.move(row.x+Math.min(30,row.width/2),row.y+Math.min(10,row.height/2));
 const outerBefore=(await inspection()).nodes.find(n=>n.id==='nav-home').rect[1];
 await page.mouse.wheel(0,320);await page.waitForFunction(()=>window.rustx.inspect().nodes.find(n=>n.id==='row-range').text!=='Materialized window: 0..31 / 100000');
 assert.equal((await inspection()).nodes.find(n=>n.id==='nav-home').rect[1],outerBefore);
 assert((await countRows())<=120);
 report.checks.push('wheel over a row routes to the nested Rust virtual list');
 await top();
 await page.setViewportSize({width:390,height:844});await frames();
 const narrow=await inspection();assert(narrow.nodes.filter(n=>n.id?.startsWith('nav-')).every(n=>n.rect[0]+n.rect[2]<=391));
 assert(narrow.nodes.find(n=>n.id==='engine-dashboard').rect[2]<=390);
 await page.screenshot({path:new URL(`stage2-${suffix}-390.png`,evidence).pathname});
 await page.setViewportSize({width:1200,height:1000});await frames();
 report.checks.push('390px responsive navigation/dashboard reflow and wide resize');
 if(gpu) {
  const normal=await page.evaluate(()=>window.stage2Presentation);
  assert(normal.submits>0&&normal.presents>0);assert.equal(normal.textureToBuffer,0);assert.equal(normal.cpuTransfers,0);
  report.normalGpuPresentation=normal;
  const preserved=(await inspection()).nodes.find(n=>n.id==='selected-row').text;
  await page.evaluate(()=>window.rustx.destroyGPU());
  await page.waitForFunction(()=>window.rustx.renderer()==='cpu');await frames();
  assert.equal((await inspection()).nodes.find(n=>n.id==='selected-row').text,preserved);
  await page.evaluate(()=>window.rustx.retryGPU());await page.waitForFunction(()=>window.rustx.renderer()==='gpu');await frames();
  assert.equal((await inspection()).nodes.find(n=>n.id==='selected-row').text,preserved);
  report.checks.push('actual GPUDevice.destroy triggers CPU recovery without remounting application state, then a fresh GPU device renders the retained scene');
 }
 const before=await page.evaluate(()=>window.rustx.draws());await page.waitForTimeout(400);assert.equal(await page.evaluate(()=>window.rustx.draws()),before);
 report.checks.push('settled application has no repeated presented frames');
 await page.screenshot({path:new URL(`stage2-${suffix}.png`,evidence).pathname});
 // Fresh DPR-2 context exercises allocation/reflow instead of stretching old pixels.
 const scaled=await browser.newContext({viewport:{width:390,height:844},deviceScaleFactor:2});
 const scaledPage=await scaled.newPage();await scaledPage.goto(`${base}/?renderer=cpu`);await scaledPage.waitForFunction(()=>window.rustx?.draws()>0);
 assert.deepEqual(await scaledPage.locator('canvas').evaluate(el=>[el.width,el.height]),[780,1688]);await scaled.close();
 report.checks.push('DPR 2 allocates the expected physical canvas dimensions');
 assert.deepEqual(errors,[]);report.result='passed';report.finalMetrics=(await inspection()).metrics;
} catch(error) {
 report.result='failed';report.error=String(error);report.errors=errors;
 if(page){report.inspection=await page.evaluate(()=>window.rustx?.inspect()).catch(()=>null);await page.screenshot({path:new URL(`stage2-${suffix}-failure.png`,evidence).pathname}).catch(()=>{});}
 throw error;
} finally {
 await fs.writeFile(new URL(`stage2-browser-${suffix}.json`,evidence),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));await browser?.close();
}
