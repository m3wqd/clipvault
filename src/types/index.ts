export interface Item { id:number; kind:'text'|'url'|'code'|'image'; text:string; first_copied:number; last_copied:number; copy_count:number; pinned:boolean;favorite:boolean;image_path:string|null;source?:string|null }
export interface Status {paused:boolean;incognito?:boolean;backend:string;error:string|null}
export type Request = {action:'list';query:string;pinned:boolean;kind:string;favorite?:boolean;collection?:number|null;tag?:string;since?:number;frequent?:boolean;offset?:number}|{action:'copy'|'delete'|'thumbnail'|'tags'|'delete_snippet';id:number}|{action:'pin'|'favorite';id:number;value:boolean}|{action:'pause';value:boolean}|{action:'status'|'settings'|'collections'}
|{action:'save_settings';settings:Settings}|{action:'snippets';query:string}|{action:'save_snippet';id:number|null;title:string;body:string;category:string}|{action:'copy_text';text:string}|{action:'create_collection';name:string}|{action:'rename_collection';id:number;name:string}|{action:'delete_collection';id:number}|{action:'assign';id:number;collection:number;value:boolean}|{action:'set_tags';id:number;names:string[]}|{action:'clear';all:boolean;confirmation:string};
export interface Settings {max_items:number;max_bytes:number;retention_days:number;incognito:boolean;close_to_tray:boolean;theme:'dark'|'light';language:'en'|'ru';shortcut:string;skip_sensitive:boolean;excluded_apps:string[]}
export interface Snippet {id:number;title:string;body:string;category:string}
export interface Named {id:number;name:string}

