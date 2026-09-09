// Generate small, real RGBA PNG packs for exercising the complete production loader.
// No image library or VaM runtime is required. Never overwrites existing pack folders.
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { deflateSync } from 'node:zlib';
const here = dirname(fileURLToPath(import.meta.url));
const destination = process.argv[2];
if (!destination) { console.error('Usage: npm run fixtures -- "<DesktopPet/pets>"'); process.exit(1); }
function crc32(data) {
  let crc = 0xffffffff;
  for (const byte of data) { crc ^= byte; for (let i=0;i<8;i++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0); }
  return (crc ^ 0xffffffff) >>> 0;
}
function chunk(name,data) {
  const type=Buffer.from(name); const len=Buffer.alloc(4); len.writeUInt32BE(data.length); const crc=Buffer.alloc(4); crc.writeUInt32BE(crc32(Buffer.concat([type,data])));
  return Buffer.concat([len,type,data,crc]);
}
function png(width,height,index,color) {
  const raw=Buffer.alloc((width*4+1)*height);
  const centerY=height*0.63+Math.sin(index/12*Math.PI*2)*height*0.04;
  for(let y=0;y<height;y++) for(let x=0;x<width;x++) {
    const offset=y*(width*4+1)+1+x*4;
    if (((x-width/2)/(width*0.23))**2+((y-centerY)/(height*0.27))**2 < 1) {
      const eye = Math.abs(y-(centerY-height*0.05))<height*0.025 && [width*0.43,width*0.57].some(ex=>Math.abs(x-ex)<width*0.015);
      raw[offset]=eye?25:color[0];raw[offset+1]=eye?35:color[1];raw[offset+2]=eye?40:color[2];raw[offset+3]=255;
    }
  }
  const ihdr=Buffer.alloc(13);ihdr.writeUInt32BE(width,0);ihdr.writeUInt32BE(height,4);ihdr[8]=8;ihdr[9]=6;
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',ihdr),chunk('IDAT',deflateSync(raw)),chunk('IEND',Buffer.alloc(0))]);
}
for(const file of ['pet.example.json','dog.pet.json']) {
  const pet=JSON.parse(await readFile(resolve(here,'../docs/examples',file),'utf8'));
  const root=resolve(destination,pet.id);
  await mkdir(root); // EEXIST is deliberate: preserve user assets.
  let clipIndex=0;
  for(const clip of Object.values(pet.animations)) {
    const dir=join(root,clip.source.dir);await mkdir(dir,{recursive:true});
    const colors=[[110,210,175],[250,200,100],[205,150,240],[110,165,245],[230,160,165]];
    for(let i=1;i<=12;i++) await writeFile(join(dir,`${i}.png`),png(pet.render.canvas_width,pet.render.canvas_height,i,colors[clipIndex%colors.length]));
    clipIndex++;
  }
  await writeFile(join(root,'pet.json'),JSON.stringify(pet,null,2)+'\n');
  console.log(`Created ${root}. Use Tray → Reload Content, then Next Pet.`);
}
