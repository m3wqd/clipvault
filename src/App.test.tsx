// @vitest-environment jsdom
import React from 'react';
import {render,screen,fireEvent,waitFor,cleanup} from '@testing-library/react';
import {afterEach,it,expect,vi} from 'vitest';
const {invoke,listeners}=vi.hoisted(()=>({invoke:vi.fn(),listeners:new Map<string,()=>void>()}));
vi.mock('./services/ipc',()=>({request:invoke}));
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async(name:string,cb:()=>void)=>{listeners.set(name,cb);return()=>listeners.delete(name)})}));
import {App} from './App';
afterEach(()=>{cleanup();invoke.mockReset();listeners.clear()});
function setup(){invoke.mockImplementation(async(r:{action:string})=>{if(r.action==='status')return {paused:false,backend:'Test adapter',error:null};if(r.action==='collections')return [];if(r.action==='tags')return [];if(r.action==='list')return [{id:1,kind:'text',text:'Recorded clipboard text',first_copied:1,last_copied:2,copy_count:1,pinned:false,favorite:false,image_path:null}];return null});}
it('loads history and handles copy, pin, delete and events through IPC',async()=>{setup();render(<App/>);expect(screen.getByText('Loading history…')).toBeTruthy();await screen.findAllByText('Recorded clipboard text');fireEvent.click(screen.getByRole('button',{name:'Copy item'}));await waitFor(()=>expect(invoke).toHaveBeenCalledWith({action:'copy',id:1}));fireEvent.click(screen.getByRole('button',{name:'Pin item'}));await waitFor(()=>expect(invoke).toHaveBeenCalledWith({action:'pin',id:1,value:true}));fireEvent.click(screen.getByRole('button',{name:'Delete item'}));await waitFor(()=>expect(invoke).toHaveBeenCalledWith({action:'delete',id:1}));const calls=invoke.mock.calls.length;listeners.get('history-changed')?.();await waitFor(()=>expect(invoke.mock.calls.length).toBeGreaterThan(calls));});
it('debounces search and applies type filter',async()=>{setup();render(<App/>);await screen.findAllByText('Recorded clipboard text');fireEvent.change(screen.getByRole('textbox',{name:'Search history'}),{target:{value:'needle'}});fireEvent.click(screen.getByRole('button',{name:'Links'}));await waitFor(()=>expect(invoke).toHaveBeenCalledWith(expect.objectContaining({action:'list',query:'needle',kind:'url'})));});
it('shows backend errors rather than substituted data',async()=>{invoke.mockRejectedValue(new Error('Database unavailable'));render(<App/>);await screen.findByRole('alert');expect(screen.getByRole('alert').textContent).toContain('Database unavailable');});
