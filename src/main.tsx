import React from 'react';
import {createRoot} from 'react-dom/client';
import {App} from './App';
import './styles/app.css';
import {Popup} from './components/Popup';
import {request} from './services/ipc';
import type {Settings} from './types';
void request<Settings>({action:'settings'}).then(s=>{document.documentElement.dataset.theme=s.theme;document.documentElement.lang=s.language}).catch(()=>{});
createRoot(document.getElementById('root')!).render(<React.StrictMode>{new URLSearchParams(location.search).has('popup')?<Popup/>:<App/>}</React.StrictMode>);
