import {invoke,isTauri} from '@tauri-apps/api/core';
import type {Request} from '../types';
export async function request<T>(request:Request):Promise<T>{if(!isTauri()) throw new Error('Open ClipVault as a desktop application. Browser mode has no access to your clipboard history.');return invoke<T>('request',{request});}
