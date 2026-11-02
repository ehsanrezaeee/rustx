// Full source + prebuilt web demo + evidence + portable Git history. No Python.
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const root=fileURLToPath(new URL('../',import.meta.url));
const git=(...args)=>execFileSync('git',args,{cwd:root,encoding:'utf8'}).trim();
if(git('status','--porcelain'))throw new Error('Commit the source and evidence before packaging.');
const revision=git('rev-parse','HEAD'),short=revision.slice(0,12);
const release=path.join(root,'releases',`rustx-stage-01-${short}`);
await fs.mkdir(release,{recursive:true});
const files=git('ls-files','-z').split('\0').filter(Boolean);
for(const file of files){const to=path.join(release,file);await fs.mkdir(path.dirname(to),{recursive:true});await fs.copyFile(path.join(root,file),to);}
for(const name of ['host.js','services.js','index.html']){
 const source=await fs.readFile(path.join(root,'web',name));
 const built=await fs.readFile(path.join(root,'dist',name));
 if(!source.equals(built))throw new Error(`Rebuild dist/: stale ${name}`);
}
await fs.access(path.join(root,'dist/pkg/rustx_lab_bg.wasm'));
await fs.cp(path.join(root,'dist'),path.join(release,'dist'),{recursive:true});
await fs.writeFile(path.join(release,'GIT-REVISION.txt'),revision+'\n');
const info=JSON.parse(await fs.readFile(path.join(root,'BUILD-INFO.json'),'utf8'));
info.git_revision=revision;info.git_branch=git('branch','--show-current');
await fs.writeFile(path.join(release,'BUILD-INFO.json'),JSON.stringify(info,null,2)+'\n');
git('bundle','create',path.join(release,'rustx-history.bundle'),'--all');
const payload=[];
async function walk(dir,relative=''){
 for(const entry of await fs.readdir(dir,{withFileTypes:true})){
  const file=path.posix.join(relative,entry.name),absolute=path.join(dir,entry.name);
  if(entry.isDirectory())await walk(absolute,file);
  else payload.push([file,createHash('sha256').update(await fs.readFile(absolute)).digest('hex')]);
 }
}
await walk(release);payload.sort(([a],[b])=>a.localeCompare(b,'en'));
await fs.writeFile(path.join(release,'ARTIFACT_MANIFEST.sha256'),payload.map(([p,h])=>`${h}  ${p}\n`).join(''));
const archive=release+'.zip';
execFileSync('zip',['-q','-r',archive,path.basename(release)],{cwd:path.dirname(release)});
const digest=createHash('sha256').update(await fs.readFile(archive)).digest('hex');
await fs.writeFile(archive+'.sha256',`${digest}  ${path.basename(archive)}\n`);
console.log(`${archive}\nSHA-256: ${digest}\nGit revision: ${revision}`);
