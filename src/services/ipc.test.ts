import {it,expect} from 'vitest';
import {request} from './ipc';
it('never substitutes fake history outside Tauri',async()=>{await expect(request({action:'status'})).rejects.toThrow('desktop application')});
