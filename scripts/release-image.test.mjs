import test from 'node:test';
import assert from 'node:assert/strict';
import { releaseImage } from './release-image.mjs';

const release = (tag, prerelease=false) => ({tag_name:tag,prerelease,draft:false});
test('stable release tags include version aliases and latest', () => {
  assert.deepEqual(releaseImage('XVIIdev/Pixerve',release('v1.2.3')), {
    image:'ghcr.io/xviidev/pixerve',tags:['v1.2.3','1.2.3','latest']
  });
});
test('unprefixed version is not duplicated', () => {
  assert.deepEqual(releaseImage('xviidev/pixerve',release('1.2.3')).tags,['1.2.3','latest']);
});
test('GitHub prerelease flag cannot promote latest', () => {
  assert(!releaseImage('xviidev/pixerve',release('v1.2.3',true)).tags.includes('latest'));
});
test('prerelease version cannot promote latest even if marked stable on GitHub', () => {
  assert.deepEqual(releaseImage('xviidev/pixerve',release('v1.2.3-rc.1')).tags,['v1.2.3-rc.1','1.2.3-rc.1']);
});
test('invalid tags, drafts, and ambiguous event fields are rejected', () => {
  for(const tag of ['latest','v1.2','v01.2.3','v1.2.3\nlatest','v1.2.3;echo bad','v1.2.3+build']) {
    assert.throws(()=>releaseImage('xviidev/pixerve',release(tag)));
  }
  assert.throws(()=>releaseImage('xviidev/pixerve',{...release('v1.2.3'),draft:true}));
  assert.throws(()=>releaseImage('xviidev/pixerve',{tag_name:'v1.2.3'}));
});
test('invalid repository paths are rejected before registry access', () => {
  for(const repository of ['', 'xviidev', 'xviidev/pixerve/extra', 'bad owner/pixerve']) {
    assert.throws(()=>releaseImage(repository,release('v1.2.3')));
  }
});
