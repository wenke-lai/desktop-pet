import { describe, expect, it, vi } from 'vitest';
import { BehaviorEngine, interval, weightedChoice } from '../src/pet/BehaviorEngine';
import { AnimationPlayer } from '../src/pet/AnimationPlayer';
import type { BehaviorDefinition, Trigger } from '../src/pet/types';
function player() { return new AnimationPlayer({ base: { frames:[0], fps:1, loop:true, priority:0, interruptible:true }, action: { frames:[0], fps:10, loop:false, priority:10, interruptible:true } }, 'base'); }
function behavior(trigger: Trigger, extra: Partial<BehaviorDefinition> = {}): BehaviorDefinition {
  return { id:'test', trigger, choose:[{ animation:'action', weight:1 }], ...extra };
}
describe('weighted random', () => {
  it('selects each bucket at boundaries and ignores nonpositive weights', () => {
    const choices = [{ animation:'bad',weight:0 },{ animation:'a',weight:5 },{ animation:'b',weight:2 },{ animation:'c',weight:1 }];
    expect(weightedChoice(choices,()=>0)).toBe('a'); expect(weightedChoice(choices,()=>5/8)).toBe('b'); expect(weightedChoice(choices,()=>7/8)).toBe('c');
    expect(weightedChoice([{animation:'bad',weight:-1}])).toBeNull();
    expect(weightedChoice([{animation:'bad',weight:Infinity}])).toBeNull();
  });
  it('has expected statistical proportions with reproducible PRNG', () => {
    let seed = 42; const random = () => { seed = (Math.imul(seed,1664525)+1013904223)>>>0; return seed / 2**32; };
    const choices = [{animation:'a',weight:5},{animation:'b',weight:2},{animation:'c',weight:1}]; const counts: Record<string,number> = {a:0,b:0,c:0};
    for(let i=0;i<80000;i++) { const result=weightedChoice(choices,random)!; counts[result]=(counts[result]??0)+1; }
    expect(counts.a!/80000).toBeCloseTo(5/8,2); expect(counts.b!/80000).toBeCloseTo(2/8,2); expect(counts.c!/80000).toBeCloseTo(1/8,2);
  });
  it('normalizes large finite weights without overflow', () => {
    expect(weightedChoice([{animation:'a',weight:1e308},{animation:'b',weight:1e308}],()=>0.75)).toBe('b');
  });
});
describe('BehaviorEngine', () => {
  it('samples inclusive timer interval bounds', () => { expect(interval(5000,12000,()=>0)).toBe(5000); expect(interval(5000,12000,()=>1)).toBe(12000); });
  it('reschedules the timer after every attempted trigger', () => {
    const p=player(); const spy=vi.spyOn(p,'request');
    const engine = new BehaviorEngine([behavior({type:'timer',min_ms:100,max_ms:200})],p,0,()=>0);
    engine.tick(99); expect(spy).not.toHaveBeenCalled(); engine.tick(100); expect(spy).toHaveBeenCalledTimes(1);
    engine.tick(199); expect(spy).toHaveBeenCalledTimes(1); engine.tick(200); expect(spy).toHaveBeenCalledTimes(2);
  });
  it('does not burst after a timer sleep', () => {
    const p=player(); const spy=vi.spyOn(p,'request'); const e=new BehaviorEngine([behavior({type:'timer',min_ms:100,max_ms:100})],p);
    e.tick(100000); expect(spy).toHaveBeenCalledTimes(1); e.tick(100001); expect(spy).toHaveBeenCalledTimes(1);
  });
  it('only_when fallback prevents requests while a different animation is playing', () => {
    const p=player(); p.request('action'); const spy=vi.spyOn(p,'request');
    const e=new BehaviorEngine([behavior({type:'timer',min_ms:100,max_ms:100},{only_when:'fallback'})],p);
    e.tick(100); expect(spy).not.toHaveBeenCalled(); p.tick(100); e.tick(200); expect(spy).toHaveBeenCalledOnce();
  });
  it('cooldown is independent of animation duration', () => {
    const p=player(); const spy=vi.spyOn(p,'request'); const e=new BehaviorEngine([behavior({type:'click',button:'left'},{cooldown_ms:1000})],p);
    e.event('click',0,'left'); p.tick(200); e.event('click',999,'left'); expect(spy).toHaveBeenCalledOnce();
    e.event('click',1000,'left'); expect(spy).toHaveBeenCalledTimes(2);
  });
  it('cursor enter uses hysteresis, independent of alpha hover', () => {
    const p=player(); const spy=vi.spyOn(p,'request'); const e=new BehaviorEngine([behavior({type:'cursor_distance_enter',distance_px:180,reset_distance_px:230})],p);
    e.sample(0,181,false); e.sample(1,180,false); e.sample(2,190,false); e.sample(3,175,false); expect(spy).toHaveBeenCalledOnce();
    e.sample(4,230,false); e.sample(5,179,false); expect(spy).toHaveBeenCalledTimes(2);
  });
  it('does not retry a suppressed proximity enter until exit/reset', () => {
    const p=player(); const spy=vi.spyOn(p,'request'); const e=new BehaviorEngine([behavior({type:'cursor_distance_enter',distance_px:100,reset_distance_px:150},{cooldown_ms:1000})],p);
    e.sample(0,50,false); e.sample(10,160,false); e.sample(20,50,false); e.sample(2000,50,false); expect(spy).toHaveBeenCalledOnce();
    e.sample(2001,160,false); e.sample(2002,50,false); expect(spy).toHaveBeenCalledTimes(2);
  });
  it('hover requires alpha hit continuously for dwell duration, once per visit', () => {
    const p=player(); const spy=vi.spyOn(p,'request'); const e=new BehaviorEngine([behavior({type:'hover',dwell_ms:500})],p);
    e.sample(0,0,false); e.sample(600,0,false); expect(spy).not.toHaveBeenCalled();
    e.sample(1000,0,true); e.sample(1499,0,true); expect(spy).not.toHaveBeenCalled(); e.sample(1500,0,true); e.sample(2000,0,true); expect(spy).toHaveBeenCalledOnce();
    e.sample(2100,0,false); e.sample(2200,0,true); e.sample(2700,0,true); expect(spy).toHaveBeenCalledTimes(2);
  });
  it('dispatches all three click buttons and drag triggers separately', () => {
    const p=player(); const spy=vi.spyOn(p,'request');
    const definitions = [behavior({type:'click',button:'right'}),behavior({type:'click',button:'middle'}),behavior({type:'drag_start'}),behavior({type:'drag_end'})];
    const e=new BehaviorEngine(definitions,p); e.event('click',0,'left'); expect(spy).not.toHaveBeenCalled();
    e.event('click',1,'right'); e.event('click',2,'middle'); e.event('drag_start',3); e.event('drag_end',4); expect(spy).toHaveBeenCalledTimes(4);
  });
});
