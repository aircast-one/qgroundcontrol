import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { scriptRefusal, scripting } from './ScriptingScreen';

describe('ScriptingScreenTest', () => {
  it('the_page_lists_the_scripts_the_core_found', () => {
    const page = scripting(parseJson('{"available":true,"enabled":true,"enable":null,"scripts":["hello.lua"],"busy":true,"progress":0.5,"status":"Upload succeeded: /APM/scripts/hello.lua","unsupportedText":"x"}'));
    expect(page?.scripts).toEqual(['hello.lua']);
    expect(page?.busy).toBe(true);
    expect(page?.progress).toBe(0.5);
    expect(scripting(parseJson('{"available":false}'))?.available).toBe(false);
  });

  it('a_refused_transfer_opens_the_lua_dialog_with_the_core_text_or_the_fallback', () => {
    expect(scriptRefusal('Lua Delete', 'Another FTP operation is in progress', 'Delete failed')).toEqual({ title: 'Lua Delete', text: 'Another FTP operation is in progress' });
    expect(scriptRefusal('Lua Upload', '', 'Upload failed')).toEqual({ title: 'Lua Upload', text: 'Upload failed' });
    expect(scriptRefusal('Lua Download', null, 'Download failed')).toBeNull();
  });
});
