import { expect, it } from 'vitest';
import { alphaHit } from '../src/pet/HitTester';
import type { Frame } from '../src/pet/types';
const frame: Frame = { image: {} as CanvasImageSource, width:2, height:2, alpha:new Uint8Array([0,19,20,255]) };
it('maps physical local coordinates back to source pixels at 150% DPI', () => {
  const viewport={width:300,height:300};
  expect(alphaHit(frame,{x:20,y:20},viewport,20)).toBe(false);
  expect(alphaHit(frame,{x:200,y:20},viewport,20)).toBe(false);
  expect(alphaHit(frame,{x:20,y:200},viewport,20)).toBe(true);
  expect(alphaHit(frame,{x:200,y:200},viewport,20)).toBe(true);
});
it('rejects outside, empty, transparent and right/bottom edge pixels', () => {
  for(const p of [{x:-1,y:0},{x:0,y:-1},{x:300,y:100},{x:100,y:300}]) expect(alphaHit(frame,p,{width:300,height:300},20)).toBe(false);
  expect(alphaHit(null,{x:0,y:0},{width:1,height:1},20)).toBe(false);
  expect(alphaHit(frame,{x:0,y:0},{width:1,height:1},0)).toBe(false);
});
